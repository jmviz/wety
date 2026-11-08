//! Parsoid HTML to text with positioned template and footnote spans.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
};

use scraper::{ElementRef, Node};
use serde_json::Value;

use crate::model::{Call, Link, Mention, Span};

/// Elements whose content is not prose.
const SKIPPED: [&str; 5] = ["link", "style", "meta", "script", "noscript"];
const LISTS: [&str; 3] = ["ol", "ul", "dl"];

/// The calls a transclusion's `data-mw` records, in order; literal wikitext between
/// them (Parsoid joins templates that build one block) is left out.
pub fn calls(data_mw: &str) -> Vec<Call> {
    let Ok(value) = serde_json::from_str::<Value>(data_mw) else {
        return vec![];
    };
    let Some(parts) = value.get("parts").and_then(Value::as_array) else {
        return vec![];
    };
    parts
        .iter()
        .filter_map(|part| part.get("template"))
        .filter_map(|t| {
            let name = template_name(t.get("target")?.get("wt")?.as_str()?);
            let args = t
                .get("params")
                .and_then(Value::as_object)
                .map(|params| {
                    params
                        .iter()
                        .map(|(k, v)| {
                            let wt = v.get("wt").and_then(Value::as_str).unwrap_or_default();
                            (k.clone(), wt.to_string())
                        })
                        .collect()
                })
                .unwrap_or_else(BTreeMap::new);
            Some(Call { name, args })
        })
        .collect()
}

/// A template's name as the processor compares it: no `Template:` prefix, spaces for
/// underscores, the first letter lowercased (MediaWiki ignores its case).
pub fn template_name(wt: &str) -> String {
    let name = wt.trim().replace('_', " ");
    let name = name
        .strip_prefix("Template:")
        .or_else(|| name.strip_prefix("template:"))
        .unwrap_or(&name)
        .trim();
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A wiki link's target from its `href` ("./popina#Latin").
pub fn link(element: &scraper::node::Element) -> Option<Link> {
    let rel = element.attr("rel")?;
    if rel != "mw:WikiLink" {
        return None;
    }
    let href = element.attr("href")?.strip_prefix("./")?;
    let (path, anchor) = match href.split_once('#') {
        Some((p, a)) => (p, Some(a)),
        None => (href, None),
    };
    let path = path.split('?').next().unwrap_or(path);
    let decode = |s: &str| {
        urlencoding::decode(s)
            .map(|d| d.replace('_', " "))
            .unwrap_or_else(|_| s.replace('_', " "))
    };
    Some(Link {
        page: decode(path),
        anchor: anchor.map(decode),
        missing: element.classes().any(|c| c == "new"),
    })
}

/// A mention element (`<i class="mention" lang="..">`): its language, text and link.
pub fn mention_of(element: ElementRef, lang: &str) -> Mention {
    Mention {
        lang: lang.to_string(),
        term: element.text().collect::<String>().trim().to_string(),
        link: element
            .select(&crate::page::A)
            .next()
            .and_then(|a| link(a.value())),
        gloss: None,
    }
}

/// The mention in or at `element`, if any, with the gloss `element` shows for it.
pub fn mention(element: ElementRef) -> Option<Mention> {
    let found = std::iter::once(element)
        .chain(element.descendants().filter_map(ElementRef::wrap))
        .find(|e| {
            e.value().classes().any(|c| c == "mention") && e.value().attr("lang").is_some()
        })?;
    let mut m = mention_of(found, found.value().attr("lang").expect("checked"));
    m.gloss = element
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|e| e.value().classes().any(|c| c == "mention-gloss"))
        .map(|g| g.text().collect::<String>().trim().to_string())
        .filter(|g| !g.is_empty());
    Some(m)
}

pub fn is_transclusion(element: &scraper::node::Element) -> bool {
    element
        .attr("typeof")
        .is_some_and(|t| t.split(' ').any(|t| t == "mw:Transclusion"))
}

fn is_ref(element: &scraper::node::Element) -> bool {
    element
        .attr("typeof")
        .is_some_and(|t| t.contains("mw:Extension/ref"))
}

/// A page's transclusions by `about` id. Parsoid gives a transclusion's first element
/// its `data-mw`; the elements after it that continue its output share only the id,
/// and may sit in other blocks (an `{{ety}}` tree, then the prose it writes).
#[derive(Default)]
pub struct Transclusions {
    calls: HashMap<String, Vec<Call>>,
    /// Ids already given a span: a later span of one is `continued`.
    seen: RefCell<HashSet<String>>,
}

impl Transclusions {
    pub fn of(root: ElementRef) -> Self {
        let calls = root
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|e| is_transclusion(e.value()))
            .filter_map(|e| {
                Some((
                    e.value().attr("about")?.to_string(),
                    calls(e.value().attr("data-mw")?),
                ))
            })
            .collect();
        Self {
            calls,
            seen: RefCell::default(),
        }
    }

    /// The span that opens for `about` here, if it names a transclusion.
    pub fn span(&self, about: &str, at: usize) -> Option<Span> {
        let calls = self.calls.get(about)?.clone();
        let continued = !self.seen.borrow_mut().insert(about.to_string());
        Some(Span::Template {
            start: at,
            end: at,
            calls,
            links: vec![],
            mentions: vec![],
            continued,
        })
    }
}

/// Text and spans of one block.
#[derive(Default)]
pub struct Rendered {
    pub text: String,
    pub spans: Vec<Span>,
}

struct Renderer<'p> {
    page: &'p Transclusions,
    text: String,
    chars: usize,
    spans: Vec<Span>,
    /// Template spans currently open (innermost last).
    open: Vec<usize>,
    /// A transclusion's `about` id: its span, for the siblings that continue it.
    about: HashMap<String, usize>,
    skip_lists: bool,
}

impl Renderer<'_> {
    fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.chars += s.chars().count();
    }

    fn walk(&mut self, element: ElementRef, root: bool) {
        let e = element.value();
        let name = e.name();
        if SKIPPED.contains(&name) || (self.skip_lists && !root && LISTS.contains(&name)) {
            // A template that renders only categories ({{root}}) still has its call.
            if SKIPPED.contains(&name)
                && let Some(about) = e.attr("about")
                && !self.about.contains_key(about)
                && let Some(span) = self.page.span(about, self.chars)
            {
                self.about.insert(about.to_string(), self.spans.len());
                self.spans.push(span);
            }
            return;
        }
        if is_ref(e) {
            let note = element
                .select(&crate::page::A)
                .next()
                .and_then(|a| a.value().attr("href"))
                .and_then(|h| h.split_once('#'))
                .map(|(_, f)| f.to_string())
                .unwrap_or_default();
            self.spans.push(Span::Ref {
                start: self.chars,
                note,
            });
            return;
        }
        let mut opened = None;
        if let Some(about) = e.attr("about") {
            if let Some(&i) = self.about.get(about) {
                opened = Some(i);
            } else if let Some(span) = self.page.span(about, self.chars) {
                self.spans.push(span);
                let i = self.spans.len() - 1;
                self.about.insert(about.to_string(), i);
                opened = Some(i);
            }
        }
        if let Some(i) = opened {
            self.open.push(i);
        }
        if name == "a"
            && let (Some(&i), Some(l)) = (self.open.last(), link(e))
            && let Span::Template { links, .. } = &mut self.spans[i]
        {
            links.push(l);
        }
        if let Some(lang) = e.attr("lang")
            && e.classes().any(|c| c == "mention")
            && let Some(&i) = self.open.last()
            && let Span::Template { mentions, .. } = &mut self.spans[i]
        {
            mentions.push(mention_of(element, lang));
        }
        if name == "br" {
            self.push("\n");
        }
        for child in element.children() {
            match child.value() {
                Node::Text(t) => self.push(t),
                Node::Element(_) => self.walk(ElementRef::wrap(child).expect("an element"), false),
                _ => {}
            }
        }
        if let Some(i) = opened {
            self.open.pop();
            if let Span::Template { end, .. } = &mut self.spans[i] {
                *end = (*end).max(self.chars);
            }
        }
    }
}

/// Render `element`; nested lists are left out when `skip_lists` (they are blocks of
/// their own). Text is trimmed, and offsets with it.
pub fn render(element: ElementRef, skip_lists: bool, page: &Transclusions) -> Rendered {
    let mut r = Renderer {
        page,
        text: String::new(),
        chars: 0,
        spans: vec![],
        open: vec![],
        about: HashMap::new(),
        skip_lists,
    };
    r.walk(element, true);
    let lead = r.text.chars().take_while(|c| c.is_whitespace()).count();
    let text: String = r.text.trim().to_string();
    let len = text.chars().count();
    let fix = |x: usize| x.saturating_sub(lead).min(len);
    for span in &mut r.spans {
        match span {
            Span::Template { start, end, .. } => {
                *start = fix(*start);
                *end = fix(*end);
            }
            Span::Ref { start, .. } => *start = fix(*start),
        }
    }
    Rendered {
        text,
        spans: r.spans,
    }
}
