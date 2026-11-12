//! One page's Parsoid HTML to its entries.

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    model::{
        Block, Call, Counts, Descendant, Entry, Link, Mention, Note, PosBlock, Section, Sense, Span,
    },
    render::{Transclusions, calls, is_transclusion, link, mention, mention_of, render},
};

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("a valid selector")
}

pub(crate) static A: LazyLock<Selector> = LazyLock::new(|| sel("a"));
static BODY: LazyLock<Selector> = LazyLock::new(|| sel("body"));
static NOTES: LazyLock<Selector> = LazyLock::new(|| sel("ol.mw-references > li[id]"));
static NOTE_TEXT: LazyLock<Selector> = LazyLock::new(|| sel("span.mw-reference-text"));
static CATEGORY: LazyLock<Selector> = LazyLock::new(|| sel(r#"link[rel="mw:PageProp/Category"]"#));
static TREE: LazyLock<Selector> = LazyLock::new(|| sel("[data-ety-tree-json]"));
static HEADWORD: LazyLock<Selector> = LazyLock::new(|| sel(".headword"));
static TRANSCLUSION: LazyLock<Selector> = LazyLock::new(|| sel("[data-mw]"));
static SENSEID: LazyLock<Selector> = LazyLock::new(|| sel(".senseid[data-id]"));
static LI: LazyLock<Selector> = LazyLock::new(|| sel("li"));
// A form-of definition's target: Module:form of's link, or the mention of a template
// that only marks itself as a use-with-mention ({{eo-form of}}: "imperative of lui").
static FORM_OF: LazyLock<Selector> =
    LazyLock::new(|| sel(".form-of-definition-link, .use-with-mention"));
static HEADWORD_TR: LazyLock<Selector> = LazyLock::new(|| sel(".headword-tr"));

static POS_HEADINGS: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let map: HashMap<String, String> = serde_json::from_str(include_str!("pos_headings.json"))
        .expect("pos_headings.json is valid");
    map.into_iter()
        .map(|(k, v)| (k, &*Box::leak(v.into_boxed_str())))
        .collect()
});

/// A heading and the `<section>` Parsoid wraps around it and what follows it.
struct Sec<'a> {
    level: u8,
    title: String,
    el: ElementRef<'a>,
    children: Vec<Sec<'a>>,
}

fn sections(parent: ElementRef) -> Vec<Sec> {
    let mut out = vec![];
    for child in parent.child_elements() {
        if child.value().name() != "section" {
            continue;
        }
        let heading = child
            .child_elements()
            .find(|h| matches!(h.value().name(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6"));
        match heading {
            Some(h) => out.push(Sec {
                level: h.value().name()[1..].parse().unwrap_or(0),
                title: h.text().collect::<String>().trim().to_string(),
                el: child,
                children: sections(child),
            }),
            // The lead section has no heading.
            None => out.extend(sections(child)),
        }
    }
    out
}

/// `Some(label)` for an etymology heading: `Some(None)` for "Etymology", `Some(Some("2"))`
/// for "Etymology 2".
fn etymology_label(title: &str) -> Option<Option<String>> {
    if title == "Etymology" {
        return Some(None);
    }
    let n = title.strip_prefix("Etymology ")?;
    (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .then(|| Some(n.to_string()))
}

fn pos_code(title: &str) -> Option<&'static str> {
    POS_HEADINGS.get(&title.to_lowercase()).copied()
}

fn is_pos(sec: &Sec) -> bool {
    pos_code(&sec.title).is_some()
}

/// The section's own elements: neither its heading nor its subsections.
fn own_elements<'a>(sec: &'a Sec) -> impl Iterator<Item = ElementRef<'a>> {
    sec.el.child_elements().filter(|e| {
        !matches!(
            e.value().name(),
            "section" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        )
    })
}

fn has_ancestor_below(e: ElementRef, top: ElementRef, names: &[&str]) -> bool {
    let mut node = e.parent();
    while let Some(n) = node {
        if n.id() == top.id() {
            return false;
        }
        if let Some(el) = n.value().as_element()
            && names.contains(&el.name())
        {
            return true;
        }
        node = n.parent();
    }
    false
}

fn digest(value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hash[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// A section's prose as blocks, with the trees and footnote bodies it holds.
fn section(sec: &Sec, notes: &HashMap<String, Note>, tx: &Transclusions) -> Section {
    let mut blocks = vec![];
    let mut trees = vec![];
    for e in own_elements(sec) {
        for t in std::iter::once(e)
            .filter(|e| e.value().attr("data-ety-tree-json").is_some())
            .chain(e.select(&TREE))
        {
            if let Some(v) = t
                .value()
                .attr("data-ety-tree-json")
                .and_then(|j| serde_json::from_str::<Value>(j).ok())
            {
                trees.push(v);
            }
        }
        // A drawn tree's text is not prose: its block keeps only the calls.
        if e.value().attr("data-ety-tree-json").is_some()
            || e.value().classes().any(|c| c == "etytree")
        {
            if let Some(span) = e.value().attr("about").and_then(|a| tx.span(a, 0)) {
                blocks.push(Block {
                    kind: "tree",
                    depth: 0,
                    text: String::new(),
                    spans: vec![span],
                });
            }
            continue;
        }
        match e.value().name() {
            "ul" | "ol" | "dl" => list_blocks(e, 0, &mut blocks, tx),
            "p" => push_block(&mut blocks, "paragraph", 0, e, true, tx),
            _ => push_block(&mut blocks, "other", 0, e, true, tx),
        }
    }
    let mut cited = vec![];
    let mut seen = HashSet::new();
    for b in &blocks {
        for s in &b.spans {
            if let Span::Ref { note, .. } = s
                && seen.insert(note.clone())
                && let Some(n) = notes.get(note)
            {
                cited.push(n.clone());
            }
        }
    }
    let mut out = Section {
        digest: String::new(),
        blocks,
        trees,
        notes: cited,
    };
    out.digest = digest(&(&out.blocks, &out.trees, &out.notes));
    out
}

fn push_block(
    blocks: &mut Vec<Block>,
    kind: &'static str,
    depth: u8,
    e: ElementRef,
    skip_lists: bool,
    tx: &Transclusions,
) {
    let r = render(e, skip_lists, tx);
    if !r.text.is_empty() || !r.spans.is_empty() {
        blocks.push(Block {
            kind,
            depth,
            text: r.text,
            spans: r.spans,
        });
    }
}

fn list_blocks(list: ElementRef, depth: u8, blocks: &mut Vec<Block>, tx: &Transclusions) {
    // A list a template rendered whole ("{{R:mak:Cense:1979|page=859}}" as its own
    // <ul>) carries the call itself: its first item takes it, as a zero-width span.
    // Claimed only with an item to hold it: an empty list ({{etymon}} with no tree)
    // leaves the call to the element after it.
    let mut own = list.value().attr("about");
    for item in list.child_elements() {
        if !matches!(item.value().name(), "li" | "dd" | "dt") {
            continue;
        }
        let before = blocks.len();
        push_block(blocks, "item", depth, item, true, tx);
        if let Some(span) = own.take().and_then(|a| tx.span(a, 0)) {
            if blocks.len() == before {
                blocks.push(Block {
                    kind: "item",
                    depth,
                    text: String::new(),
                    spans: vec![],
                });
            }
            blocks[before].spans.insert(0, span);
        }
        for sub in item
            .child_elements()
            .filter(|e| matches!(e.value().name(), "ul" | "ol" | "dl"))
        {
            list_blocks(sub, depth + 1, blocks, tx);
        }
    }
}

/// The calls of every transclusion in `e` that is not inside one of `skip` (below `e`).
fn calls_within(e: ElementRef, skip: &[&str]) -> Vec<Call> {
    let mut out: Vec<Call> = vec![];
    let own = is_transclusion(e.value()).then_some(e);
    for t in own.into_iter().chain(e.select(&TRANSCLUSION)) {
        if !is_transclusion(t.value()) || (t.id() != e.id() && has_ancestor_below(t, e, skip)) {
            continue;
        }
        out.extend(t.value().attr("data-mw").map(calls).unwrap_or_default());
    }
    out
}

type ItemCalls = HashMap<ego_tree::NodeId, Vec<Call>>;

/// A list line of a transclusion's wikitext, with the lines nested under it.
struct Line {
    calls: Vec<Call>,
    children: Vec<Line>,
}

/// List items of `e` with no list item between them and `e`.
fn top_items(e: ElementRef) -> Vec<ElementRef> {
    e.descendants()
        .filter_map(ElementRef::wrap)
        .filter(|li| {
            li.value().name() == "li" && li.id() != e.id() && !has_ancestor_below(*li, e, &["li"])
        })
        .collect()
}

/// Pair items with lines level by level. Items under a line that has no lines of its
/// own were rendered from elsewhere (`{{desctree}}`) and keep no calls. Returns whether
/// every level paired.
fn align(items: &[ElementRef], lines: Vec<Line>, out: &mut ItemCalls) -> bool {
    if items.len() != lines.len() {
        return false;
    }
    let mut ok = true;
    for (li, line) in items.iter().zip(lines) {
        out.insert(li.id(), line.calls);
        if !line.children.is_empty() {
            ok &= align(&top_items(*li), line.children, out);
        }
    }
    ok
}

/// When one transclusion renders a whole list (`{{top4}}` around `* {{desc|...}}` lines),
/// its list items carry no calls of their own. The wikitext between its calls holds the
/// list markup: each `*`/`#`/`:` line's calls belong to the list item at its place.
/// `None` when they do not pair up.
fn item_calls(wrapper: ElementRef) -> Option<ItemCalls> {
    let data = wrapper.value().attr("data-mw")?;
    let value: Value = serde_json::from_str(data).ok()?;
    let parts = value.get("parts")?.as_array()?;
    if !parts
        .iter()
        .any(|p| p.as_str().is_some_and(|s| s.contains('\n')))
    {
        return None;
    }
    let all = calls(data);
    let mut lines: Vec<(String, Vec<Call>)> = vec![(String::new(), vec![])];
    let mut next = all.into_iter();
    for part in parts {
        if let Some(s) = part.as_str() {
            let mut segments = s.split('\n');
            if let Some(first) = segments.next() {
                lines.last_mut().expect("a line").0.push_str(first);
            }
            lines.extend(segments.map(|seg| (seg.to_string(), vec![])));
        } else if part.get("template").is_some()
            && let Some(call) = next.next()
        {
            lines.last_mut().expect("a line").1.push(call);
        }
    }
    let flat: Vec<Vec<Call>> = lines
        .iter()
        .filter(|(text, _)| text.trim_start().starts_with(['*', '#', ':']))
        .map(|(_, calls)| calls.clone())
        .collect();
    // Nest lines by their markers' depth.
    let mut stack: Vec<(usize, Line)> = vec![(
        0,
        Line {
            calls: vec![],
            children: vec![],
        },
    )];
    for (text, calls) in lines {
        let text = text.trim_start();
        let depth = text
            .chars()
            .take_while(|c| matches!(c, '*' | '#' | ':'))
            .count();
        if depth == 0 {
            continue;
        }
        while stack.len() > 1 && stack.last().expect("a level").0 >= depth {
            let (_, done) = stack.pop().expect("a level");
            stack.last_mut().expect("the root").1.children.push(done);
        }
        stack.push((
            depth,
            Line {
                calls,
                children: vec![],
            },
        ));
    }
    while stack.len() > 1 {
        let (_, done) = stack.pop().expect("a level");
        stack.last_mut().expect("the root").1.children.push(done);
    }
    let root = stack.pop().expect("the root").1;
    let mut out = ItemCalls::new();
    if align(&top_items(wrapper), root.children, &mut out) {
        return Some(out);
    }
    pair_in_order(wrapper, &flat)
}

/// How far ahead an item looks for its line: past headings and the like.
const LOOKAHEAD: usize = 6;

/// When the nesting does not pair up (a `:*` line nests otherwise in the HTML): the
/// items in document order, each with the next line, within a few, one of whose terms
/// its text shows. An item no line matches was rendered from elsewhere and keeps no
/// calls. `None` when no item matches.
fn pair_in_order(wrapper: ElementRef, lines: &[Vec<Call>]) -> Option<ItemCalls> {
    let mut out = ItemCalls::new();
    let mut next = 0;
    for li in wrapper
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "li" && e.id() != wrapper.id())
    {
        let text = own_text(li);
        let found =
            (next..lines.len().min(next + LOOKAHEAD)).find(|&k| shows_a_term(&lines[k], &text));
        if let Some(k) = found {
            out.insert(li.id(), lines[k].clone());
            next = k + 1;
        }
    }
    (!out.is_empty()).then_some(out)
}

/// An item's text without its nested lists.
fn own_text(li: ElementRef) -> String {
    li.descendants()
        .filter(|n| {
            !n.ancestors().take_while(|a| a.id() != li.id()).any(|a| {
                a.value()
                    .as_element()
                    .is_some_and(|e| matches!(e.name(), "ul" | "ol" | "dl"))
            })
        })
        .filter_map(|n| n.value().as_text().map(|t| t.to_string()))
        .collect()
}

/// Whether `text` shows a term one of the calls gives (positional arguments from 2).
fn shows_a_term(calls: &[Call], text: &str) -> bool {
    calls.iter().any(|call| {
        call.args
            .iter()
            .filter(|(k, _)| k.parse::<usize>().is_ok_and(|n| n >= 2))
            .any(|(_, v)| {
                let term = v
                    .split('<')
                    .next()
                    .unwrap_or(v)
                    .trim()
                    .trim_start_matches('*');
                term.chars().count() > 0 && term != "-" && text.contains(term)
            })
    })
}

fn item_links(li: ElementRef) -> Vec<crate::model::Link> {
    li.select(&A)
        .filter(|a| !has_ancestor_below(*a, li, &["ul", "ol", "dl"]))
        .filter_map(|a| crate::render::link(a.value()))
        .collect()
}

/// The template whose output `e` is: the nearest element at or above it that Parsoid
/// marks as part of a transclusion.
fn rendered_by(e: ElementRef, tx: &Transclusions) -> Option<String> {
    std::iter::once(*e)
        .chain(e.ancestors())
        .filter_map(|n| n.value().as_element()?.attr("about"))
        .find_map(|about| tx.calls_of(about)?.first().map(|c| c.name.clone()))
}

/// The items of `list`. An item with no calls of its own was rendered by the template
/// of the item it hangs under (`{{desctree}}` drawing another page's list), else by the
/// transclusion around it (`{{CJKV}}`): `parent` names the former.
fn descendants(
    list: ElementRef,
    given: &ItemCalls,
    tx: &Transclusions,
    parent: Option<&str>,
) -> Vec<Descendant> {
    list.child_elements()
        .filter(|li| li.value().name() == "li")
        .map(|li| {
            let calls = given
                .get(&li.id())
                .cloned()
                .unwrap_or_else(|| calls_within(li, &["ul", "ol", "dl"]));
            let via = if calls.is_empty() {
                parent.map(str::to_string).or_else(|| rendered_by(li, tx))
            } else {
                None
            };
            let own = calls
                .first()
                .map(|c| c.name.clone())
                .or_else(|| via.clone());
            let children = li
                .child_elements()
                .filter(|e| matches!(e.value().name(), "ul" | "ol"))
                .flat_map(|e| descendants(e, given, tx, own.as_deref()))
                .collect();
            Descendant {
                text: render(li, true, tx).text,
                calls,
                via,
                links: item_links(li),
                children,
            }
        })
        .collect()
}

/// Templates that name a descendant: a Derived terms section using them (on a
/// reconstructed root's page, `* {{l|ine-pro||*wēd-o-s}}` then `** {{desc|gem-pro|*wētaz}}`)
/// is read as a Descendants section too.
const DESC_TEMPLATES: [&str; 4] = ["desc", "descendant", "desctree", "descendants tree"];

/// The top-level lists of a Descendants section, through wrappers such as {{top4}}.
fn descendant_lists(sec: &Sec, counts: &mut Counts, tx: &Transclusions) -> Vec<Descendant> {
    fn find(
        e: ElementRef,
        given: &mut ItemCalls,
        counts: &mut Counts,
        out: &mut Vec<Descendant>,
        tx: &Transclusions,
    ) {
        if is_transclusion(e.value()) && e.select(&LI).next().is_some() {
            match item_calls(e) {
                Some(found) => given.extend(found),
                None if e
                    .value()
                    .attr("data-mw")
                    .is_some_and(|d| d.contains("\\n*")) =>
                {
                    counts.add("descendant lists whose items and lines differ", 1)
                }
                None => {}
            }
        }
        match e.value().name() {
            "ul" | "ol" => out.extend(descendants(e, given, tx, None)),
            "section" => {}
            _ => e
                .child_elements()
                .for_each(|c| find(c, given, counts, out, tx)),
        }
    }
    let mut out = vec![];
    let mut given = ItemCalls::new();
    own_elements(sec).for_each(|e| find(e, &mut given, counts, &mut out, tx));
    out
}

/// Templates that define a word as another's form or name with no form-of markup: Chinese
/// short forms and names ("short for 世界博覽會", "short for 商務英語", the last linking
/// its parts) and Pali script forms ("Thai script form of -jātika"). The target is the
/// first text the template writes in a language.
const UNMARKED_FORM_OF: [&str; 5] = [
    "zh-short",
    "zh-short-comp",
    "zh-alt-name",
    "zh-altname",
    "pi-sc",
];

/// A form-of definition's target. Module:form of marks its link with the language;
/// a template that only marks itself as a use-with-mention may not:
/// `{{io-form of}}` puts `lang` on a span inside the mention, `{{de-zu-infinitive of}}`
/// on nothing. Its target is in the entry's language, `lang`.
fn form_of_target(e: ElementRef, lang: &str) -> Option<Mention> {
    mention(e).or_else(|| {
        let found = std::iter::once(e)
            .chain(e.descendants().filter_map(ElementRef::wrap))
            .find(|x| x.value().classes().any(|c| c == "mention"))?;
        let lang = std::iter::once(found)
            .chain(found.descendants().filter_map(ElementRef::wrap))
            .find_map(|x| x.value().attr("lang"))
            .unwrap_or(lang);
        let m = mention_of(found, lang);
        (!m.term.is_empty() && m.link.is_some()).then_some(m)
    })
}

/// The target of an `UNMARKED_FORM_OF` template in a definition: the first element its
/// template writes with a `lang`, linked if it holds one link.
fn unmarked_form_of_target(li: ElementRef, lang: &str) -> Option<Mention> {
    let about = li
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| is_transclusion(e.value()))
        .filter(|e| !has_ancestor_below(*e, li, &["ol", "ul", "dl"]))
        .find(|e| {
            e.value().attr("data-mw").is_some_and(|d| {
                calls(d)
                    .iter()
                    .any(|c| UNMARKED_FORM_OF.contains(&c.name.as_str()))
            })
        })?
        .value()
        .attr("about")?;
    // Parsoid marks each top-level element a template writes with its `about`
    let written = |e: &ElementRef| {
        std::iter::once(*e)
            .chain(e.ancestors().filter_map(ElementRef::wrap))
            .take_while(|x| x.id() != li.id())
            .any(|x| x.value().attr("about") == Some(about))
    };
    let mut own = li
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(written);
    let target = own.find(|e| e.value().attr("lang").is_some())?;
    let term = target.text().collect::<String>().trim().to_string();
    let links: Vec<Link> = target.select(&A).filter_map(|a| link(a.value())).collect();
    let gloss = own
        .find(|e| e.value().classes().any(|c| c == "mention-gloss"))
        .map(|g| g.text().collect::<String>().trim().to_string())
        .filter(|g| !g.is_empty());
    (!term.is_empty()).then(|| Mention {
        lang: lang.to_string(),
        term,
        link: <[Link; 1]>::try_from(links).ok().map(|[l]| l),
        gloss,
    })
}

fn senses(list: ElementRef, lang: &str, depth: u8, out: &mut Vec<Sense>, tx: &Transclusions) {
    for li in list.child_elements().filter(|e| e.value().name() == "li") {
        if li.value().classes().any(|c| c == "mw-empty-elt") {
            continue;
        }
        let r = render(li, true, tx);
        let mut senseids: Vec<String> = std::iter::once(li)
            .filter(|e| e.value().classes().any(|c| c == "senseid"))
            .chain(
                li.select(&SENSEID)
                    .filter(|e| !has_ancestor_below(*e, li, &["ol", "ul", "dl"])),
            )
            .filter_map(|e| e.value().attr("data-id").map(str::to_string))
            .collect();
        senseids.dedup();
        let mut form_of: Vec<Mention> = vec![];
        for found in li
            .select(&FORM_OF)
            .filter(|e| !has_ancestor_below(*e, li, &["ol", "ul", "dl"]))
            .filter_map(|e| form_of_target(e, lang))
            .chain(unmarked_form_of_target(li, lang))
        {
            // a definition marked both ways names its target twice
            if !form_of
                .iter()
                .any(|m| m.lang == found.lang && m.term == found.term)
            {
                form_of.push(found);
            }
        }
        out.push(Sense {
            depth,
            gloss: r.text,
            senseids,
            form_of,
            calls: calls_within(li, &["ol", "ul", "dl"]),
        });
        for sub in li.child_elements().filter(|e| e.value().name() == "ol") {
            senses(sub, lang, depth + 1, out, tx);
        }
    }
}

/// A headword's text without ruby readings (`水(すい)道(どう)` is 水道).
fn headword_text(h: ElementRef) -> String {
    h.descendants()
        .filter_map(|n| n.value().as_text().map(|t| (n, t)))
        .filter(|(n, _)| {
            !n.ancestors().take_while(|a| a.id() != h.id()).any(|a| {
                a.value()
                    .as_element()
                    .is_some_and(|e| matches!(e.name(), "rt" | "rp"))
            })
        })
        .map(|(_, t)| &**t)
        .collect::<String>()
        .trim()
        .to_string()
}

fn pos_block(sec: &Sec, lang: &str, counts: &mut Counts, tx: &Transclusions) -> PosBlock {
    let mut block = PosBlock {
        heading: sec.title.clone(),
        pos: pos_code(&sec.title),
        ..PosBlock::default()
    };
    for e in own_elements(sec) {
        if block.headword.is_empty() && e.select(&HEADWORD).next().is_some() {
            block.headword = e
                .select(&HEADWORD)
                .map(|h| headword_text(h))
                .filter(|h| !h.is_empty())
                .collect();
            block.head = calls_within(e, &[]);
            block.headword_tr = e
                .select(&HEADWORD_TR)
                .map(|t| t.text().collect::<String>().trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
        } else if e.value().name() == "ol" && block.senses.is_empty() {
            senses(e, lang, 0, &mut block.senses, tx);
        }
    }
    for child in &sec.children {
        match child.title.as_str() {
            "Descendants" => block
                .descendants
                .extend(descendant_lists(child, counts, tx)),
            "Derived terms" => {
                let calls: Vec<Call> = own_elements(child)
                    .flat_map(|e| calls_within(e, &[]))
                    .collect();
                // A root's derived terms list their descendants too: read as a tree
                if calls
                    .iter()
                    .any(|c| DESC_TEMPLATES.contains(&c.name.as_str()))
                {
                    block
                        .descendants
                        .extend(descendant_lists(child, counts, tx));
                }
                block.derived.extend(calls);
            }
            _ => {}
        }
    }
    block
}

fn categories(sec: &Sec) -> Vec<String> {
    let mut seen = HashSet::new();
    sec.el
        .select(&CATEGORY)
        .filter_map(|l| l.value().attr("href"))
        .filter_map(|h| h.strip_prefix("./Category:"))
        .map(|h| {
            let name = h.split('#').next().unwrap_or(h);
            urlencoding::decode(name)
                .map(|d| d.replace('_', " "))
                .unwrap_or_else(|_| name.replace('_', " "))
        })
        .filter(|c| seen.insert(c.clone()))
        .collect()
}

fn notes(doc: &Html, tx: &Transclusions) -> HashMap<String, Note> {
    doc.select(&NOTES)
        .filter_map(|li| {
            let id = li.value().attr("id")?.to_string();
            let body = li.select(&NOTE_TEXT).next()?;
            let text = render(body, false, tx).text;
            Some((
                id.clone(),
                Note {
                    id,
                    text,
                    calls: calls_within(body, &[]),
                },
            ))
        })
        .collect()
}

/// Language headings to codes: the canonical names of a Wiktionary language export
/// (`languages.json`, from the dump of the snapshot's date).
#[derive(Default)]
pub struct Languages(HashMap<String, String>);

impl Languages {
    /// # Errors
    ///
    /// When the file cannot be read or is not an export's `languages.json`.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let raw: HashMap<String, Value> = serde_json::from_slice(&std::fs::read(path)?)?;
        Ok(Self::from_pairs(raw.iter().filter_map(|(code, data)| {
            Some((data.get("canonicalName")?.as_str()?, code.as_str()))
        })))
    }

    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        Self(
            pairs
                .into_iter()
                .map(|(name, code)| (name.to_string(), code.to_string()))
                .collect(),
        )
    }

    fn code(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// Identity of the page the HTML belongs to.
pub struct PageInfo<'a> {
    pub title: &'a str,
    pub page_id: u64,
    pub rev: u64,
    pub ns: i64,
}

/// The page's entries: one per language section, or per numbered etymology in it.
pub fn entries(
    info: &PageInfo,
    html: &str,
    languages: &Languages,
    counts: &mut Counts,
) -> Vec<Entry> {
    let doc = Html::parse_document(html);
    let Some(body) = doc.select(&BODY).next() else {
        counts.add("pages without body", 1);
        return vec![];
    };
    let tx = Transclusions::of(body);
    let notes = notes(&doc, &tx);
    let mut out = vec![];
    for lang_sec in sections(body).iter().filter(|s| s.level == 2) {
        let lang = languages.code(&lang_sec.title);
        if lang.is_none() {
            counts.add("language headings unknown", 1);
        }
        let base = |label: Option<String>| Entry {
            page: info.title.to_string(),
            page_id: info.page_id,
            rev: info.rev,
            ns: info.ns,
            lang: lang.clone(),
            lang_heading: lang_sec.title.clone(),
            ety_label: label,
            categories: categories(lang_sec),
            ..Entry::default()
        };
        let numbered = lang_sec
            .children
            .iter()
            .any(|c| etymology_label(&c.title).is_some() && contains_pos(c));
        let lang_refs = lang_sec
            .children
            .iter()
            .find(|c| c.title == "References")
            .map(|c| section(c, &notes, &tx));
        let lang_further = lang_sec
            .children
            .iter()
            .find(|c| c.title == "Further reading")
            .map(|c| section(c, &notes, &tx));
        if numbered {
            counts.add("languages with numbered etymologies", 1);
            for ety in &lang_sec.children {
                let Some(label) = etymology_label(&ety.title) else {
                    if is_pos(ety) {
                        counts.add("parts of speech beside numbered etymologies", 1);
                    }
                    continue;
                };
                let mut entry = base(label);
                // Its own: a root category of one etymology is not another's.
                entry.categories = categories(ety);
                entry.etymology = Some(section(ety, &notes, &tx));
                fill(&mut entry, &ety.children, &notes, counts, &tx);
                if entry.references.is_none() {
                    entry.language_references = lang_refs.clone();
                }
                if entry.further_reading.is_none() {
                    entry.further_reading = lang_further.clone();
                }
                out.push(entry);
            }
        } else {
            let ety = lang_sec
                .children
                .iter()
                .find(|c| etymology_label(&c.title).is_some());
            let mut entry = base(ety.and_then(|e| etymology_label(&e.title)).flatten());
            entry.etymology = ety.map(|e| section(e, &notes, &tx));
            fill(&mut entry, &lang_sec.children, &notes, counts, &tx);
            out.push(entry);
        }
    }
    counts.add("entries", out.len() as u64);
    out
}

/// Parts of speech and the sections beside them.
fn fill(
    entry: &mut Entry,
    children: &[Sec],
    notes: &HashMap<String, Note>,
    counts: &mut Counts,
    tx: &Transclusions,
) {
    for child in children {
        if is_pos(child) {
            let block = pos_block(child, entry.lang.as_deref().unwrap_or_default(), counts, tx);
            entry.pos.push(block);
            references_below(entry, &child.children, notes, tx);
            continue;
        }
        match child.title.as_str() {
            "Descendants" => entry
                .descendants
                .extend(descendant_lists(child, counts, tx)),
            "References" => add_section(&mut entry.references, section(child, notes, tx)),
            "Further reading" => {
                add_section(&mut entry.further_reading, section(child, notes, tx));
            }
            // An unnumbered etymology's own subsections.
            "Etymology" => references_below(entry, &child.children, notes, tx),
            // Parts of speech may sit under another heading ("Pronunciation 2").
            title if etymology_label(title).is_none() => {
                fill(entry, &child.children, notes, counts, tx)
            }
            _ => {}
        }
    }
}

/// References and Further reading sections below a part of speech or an unnumbered
/// etymology ("====References====" under "===Noun==="), added to the entry's own.
fn references_below(
    entry: &mut Entry,
    children: &[Sec],
    notes: &HashMap<String, Note>,
    tx: &Transclusions,
) {
    for child in children {
        match child.title.as_str() {
            "References" => add_section(&mut entry.references, section(child, notes, tx)),
            "Further reading" => {
                add_section(&mut entry.further_reading, section(child, notes, tx));
            }
            _ if is_pos(child) || etymology_label(&child.title).is_some() => {}
            _ => references_below(entry, &child.children, notes, tx),
        }
    }
}

/// An entry's References (or Further reading) given in several places, as one section
/// in page order.
fn add_section(into: &mut Option<Section>, more: Section) {
    match into {
        None => *into = Some(more),
        Some(have) => {
            have.blocks.extend(more.blocks);
            have.trees.extend(more.trees);
            for note in more.notes {
                if !have.notes.iter().any(|n| n.id == note.id) {
                    have.notes.push(note);
                }
            }
            have.digest = digest(&(&have.blocks, &have.trees, &have.notes));
        }
    }
}

/// Whether a part of speech lies below `sec`, other than under a further etymology.
fn contains_pos(sec: &Sec) -> bool {
    sec.children
        .iter()
        .any(|c| is_pos(c) || (etymology_label(&c.title).is_none() && contains_pos(c)))
}

/// (language, term as shown, page) of each mention in the entry whose page name is not
/// the term itself: the entry names Lua computed (`popīna` → `popina`). A
/// reconstruction's page is compared by its part after the language.
/// The language and term a link-making template names: `{{inh|es|la|innodō}}` names
/// la innodō, `{{m|la|innodō}}` too. None for other templates.
fn term_argument(call: &Call) -> Option<(&str, &str)> {
    let (lang, term) = match call.name.as_str() {
        "inh" | "inherited" | "inh+" | "der" | "derived" | "der+" | "bor" | "borrowed" | "bor+"
        | "lbor" | "learned borrowing" | "slbor" | "obor" | "ubor" | "cal" | "calque" | "pcal"
        | "partial calque" | "sl" | "semantic loan" | "psm" => ("2", "3"),
        "m" | "mention" | "m+" | "l" | "link" | "l+" | "cog" | "cognate" | "ncog" | "noncog" => {
            ("1", "2")
        }
        _ => return None,
    };
    let term = call.args.get(term)?;
    let term = term.split('<').next().unwrap_or(term).trim();
    let lang = call.args.get(lang)?.split(',').next()?.trim();
    (!lang.is_empty() && !term.is_empty() && term != "-").then_some((lang, term))
}

/// A Descendants call's terms (`{{desc}}`'s positional arguments from 2, `{{l}}`'s
/// argument 2) with the line's links,
/// in order. Only where there are as many links as terms, and no term is spelled as
/// another term's page, which would show that the links are not in the terms' order
/// (82,169 of 85,812 lines naming several terms in the 2026-11-01 entries qualify).
fn paired_terms<'a>(call: &'a Call, links: &'a [Link]) -> Vec<(&'a str, &'a Link)> {
    // {{desc}} names a term per argument from 2; {{l}} one, its argument 3 being
    // the text shown.
    let last = if matches!(call.name.as_str(), "l" | "link") {
        2
    } else {
        usize::MAX
    };
    let mut numbered: Vec<(usize, &str)> = call
        .args
        .iter()
        .filter_map(|(k, v)| {
            let n = k
                .parse::<usize>()
                .ok()
                .filter(|&n| (2..=last).contains(&n))?;
            Some((n, v.as_str()))
        })
        .collect();
    numbered.sort_unstable();
    let terms: Vec<&str> = numbered
        .into_iter()
        .map(|(_, v)| v.split('<').next().unwrap_or(v).trim())
        .filter(|t| !t.is_empty() && *t != "-")
        .collect();
    if terms.is_empty() || terms.len() != links.len() {
        return vec![];
    }
    fn page(l: &Link) -> &str {
        l.page
            .strip_prefix("Reconstruction:")
            .and_then(|p| p.split_once('/'))
            .map_or(l.page.as_str(), |(_, t)| t)
    }
    let bare = |t: &'a str| t.strip_prefix('*').unwrap_or(t);
    let misordered = terms.iter().enumerate().any(|(i, t)| {
        bare(t) != page(&links[i])
            && links
                .iter()
                .enumerate()
                .any(|(j, l)| j != i && bare(t) == page(l))
    });
    if misordered {
        return vec![];
    }
    terms.into_iter().zip(links).collect()
}

pub fn entry_names(entry: &Entry) -> Vec<(String, String, String)> {
    let mut out = vec![];
    let mut add = |m: &crate::model::Mention| {
        let Some(link) = &m.link else { return };
        let page = link
            .page
            .strip_prefix("Reconstruction:")
            .and_then(|p| p.split_once('/'))
            .map_or(link.page.as_str(), |(_, t)| t);
        let term = m.term.strip_prefix('*').unwrap_or(&m.term);
        // A term written with markup (*témH-es[[-lós|-los]]) links a part of it, not
        // its page; one that is a link ([[colpus|colpum]]) names its page itself.
        if !term.is_empty() && term != page && !term.contains("[[") && !term.contains("{{") {
            out.push((m.lang.clone(), m.term.clone(), link.page.clone()));
        }
    };
    for block in entry.etymology.iter().flat_map(|e| &e.blocks) {
        for span in &block.spans {
            if let Span::Template {
                calls, mentions, ..
            } = span
            {
                mentions.iter().for_each(&mut add);
                // The template's own term, where it shows another form
                // ({{inh|es|la|innodō|innodāre}} shows innodāre, links innodo).
                if let ([call], [mention]) = (&calls[..], &mentions[..])
                    && let Some((lang, term)) = term_argument(call)
                    && term != mention.term
                {
                    add(&crate::model::Mention {
                        lang: lang.to_string(),
                        term: term.to_string(),
                        link: mention.link.clone(),
                        gloss: None,
                    });
                }
            }
        }
    }
    for pos in &entry.pos {
        pos.senses
            .iter()
            .flat_map(|s| &s.form_of)
            .for_each(&mut add);
    }
    // A Descendants line's terms, paired with its links in order
    // (`{{desc|ab|а́лас}}` links алас; `{{desc|no|knøsk|knósk}}` two pages).
    let mut lines: Vec<&Descendant> = entry
        .descendants
        .iter()
        .chain(entry.pos.iter().flat_map(|p| &p.descendants))
        .collect();
    while let Some(line) = lines.pop() {
        lines.extend(&line.children);
        let [call] = &line.calls[..] else { continue };
        if !matches!(call.name.as_str(), "desc" | "desctree" | "l" | "link") {
            continue;
        }
        let Some(lang) = call.args.get("1").map(|l| l.trim()) else {
            continue;
        };
        for (term, link) in paired_terms(call, &line.links) {
            add(&crate::model::Mention {
                lang: lang.to_string(),
                term: term.to_string(),
                link: Some(link.clone()),
                gloss: None,
            });
        }
    }
    out
}
