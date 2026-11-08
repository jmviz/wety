//! One page's Parsoid HTML to its entries.

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use sha2::{Digest, Sha256};
use wety_core::languages::Lang;

use crate::{
    model::{Block, Call, Counts, Descendant, Entry, Note, PosBlock, Section, Sense, Span},
    render::{Transclusions, calls, is_transclusion, render},
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
    for item in list.child_elements() {
        if !matches!(item.value().name(), "li" | "dd" | "dt") {
            continue;
        }
        push_block(blocks, "item", depth, item, true, tx);
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
    align(&top_items(wrapper), root.children, &mut out).then_some(out)
}

fn item_links(li: ElementRef) -> Vec<crate::model::Link> {
    li.select(&A)
        .filter(|a| !has_ancestor_below(*a, li, &["ul", "ol", "dl"]))
        .filter_map(|a| crate::render::link(a.value()))
        .collect()
}

fn descendants(list: ElementRef, given: &ItemCalls, tx: &Transclusions) -> Vec<Descendant> {
    list.child_elements()
        .filter(|li| li.value().name() == "li")
        .map(|li| Descendant {
            text: render(li, true, tx).text,
            calls: given
                .get(&li.id())
                .cloned()
                .unwrap_or_else(|| calls_within(li, &["ul", "ol", "dl"])),
            links: item_links(li),
            children: li
                .child_elements()
                .filter(|e| matches!(e.value().name(), "ul" | "ol"))
                .flat_map(|e| descendants(e, given, tx))
                .collect(),
        })
        .collect()
}

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
            "ul" | "ol" => out.extend(descendants(e, given, tx)),
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

fn senses(list: ElementRef, depth: u8, out: &mut Vec<Sense>, tx: &Transclusions) {
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
        out.push(Sense {
            depth,
            gloss: r.text,
            senseids,
            calls: calls_within(li, &["ol", "ul", "dl"]),
        });
        for sub in li.child_elements().filter(|e| e.value().name() == "ol") {
            senses(sub, depth + 1, out, tx);
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

fn pos_block(sec: &Sec, counts: &mut Counts, tx: &Transclusions) -> PosBlock {
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
        } else if e.value().name() == "ol" && block.senses.is_empty() {
            senses(e, 0, &mut block.senses, tx);
        }
    }
    for child in &sec.children {
        match child.title.as_str() {
            "Descendants" => block
                .descendants
                .extend(descendant_lists(child, counts, tx)),
            "Derived terms" => block
                .derived
                .extend(own_elements(child).flat_map(|e| calls_within(e, &[]))),
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

/// Identity of the page the HTML belongs to.
pub struct PageInfo<'a> {
    pub title: &'a str,
    pub page_id: u64,
    pub rev: u64,
    pub ns: i64,
}

/// The page's entries: one per language section, or per numbered etymology in it.
pub fn entries(info: &PageInfo, html: &str, counts: &mut Counts) -> Vec<Entry> {
    let doc = Html::parse_document(html);
    let Some(body) = doc.select(&BODY).next() else {
        counts.add("pages without body", 1);
        return vec![];
    };
    let tx = Transclusions::of(body);
    let notes = notes(&doc, &tx);
    let mut out = vec![];
    for lang_sec in sections(body).iter().filter(|s| s.level == 2) {
        let lang = Lang::from_name(&lang_sec.title).ok().map(|l| l.code());
        if lang.is_none() {
            counts.add("language headings unknown", 1);
        }
        let base = |label: Option<String>| Entry {
            page: info.title.to_string(),
            page_id: info.page_id,
            rev: info.rev,
            ns: info.ns,
            lang,
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
            entry.pos.push(pos_block(child, counts, tx));
            continue;
        }
        match child.title.as_str() {
            "Descendants" => entry
                .descendants
                .extend(descendant_lists(child, counts, tx)),
            "References" => entry.references = Some(section(child, notes, tx)),
            "Further reading" => entry.further_reading = Some(section(child, notes, tx)),
            // Parts of speech may sit under another heading ("Pronunciation 2").
            title if etymology_label(title).is_none() => {
                fill(entry, &child.children, notes, counts, tx)
            }
            _ => {}
        }
    }
}

/// Whether a part of speech lies below `sec`, other than under a further etymology.
fn contains_pos(sec: &Sec) -> bool {
    sec.children
        .iter()
        .any(|c| is_pos(c) || (etymology_label(&c.title).is_none() && contains_pos(c)))
}
