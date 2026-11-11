//! wety's entries (`crates/wiktionary-entries`), as the graph build reads them: one JSON
//! line per page × language × etymology, each part of speech in it an item or a further
//! part of speech of one. Template calls keep the shape `{name, args}`; the template
//! readers take them as they come.

use crate::{descendants::RawDescendants, items::Items};
use wety_core::{
    etymology_templates::{EtyMode, TemplateKind},
    gloss::Gloss,
    items::{ItemId, RealItem},
    langterm::Term,
    languages::Lang,
    pos::Pos,
    string_pool::StringPool,
};

use std::{
    borrow::Cow,
    fs::File,
    io::{BufRead, BufReader},
    mem,
    path::Path,
    str::FromStr,
    sync::mpsc,
    thread,
};

use anyhow::{Ok, Result};
use bytelines::ByteLines;
use flate2::read::MultiGzDecoder;
use rayon::prelude::*;
use simd_json::{Mutable, ValueAccess, to_borrowed_value};

/// The lines of a gzipped (`.gz`) or plain file, in order. They are read and
/// decompressed on another thread, so that the caller's work on them overlaps it.
///
/// # Errors
///
/// This function will return an error if the file at the given path cannot be opened.
pub fn entry_lines(path: &Path) -> Result<impl Iterator<Item = Vec<u8>>> {
    Ok(entry_batches(path)?.flatten())
}

/// The lines of a file as `entry_lines` reads them, in batches of consecutive lines.
pub(crate) fn entry_batches(path: &Path) -> Result<impl Iterator<Item = Vec<Vec<u8>>>> {
    const BATCH: usize = 1024;
    let reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let reader: Box<dyn BufRead + Send> = if path.extension().is_some_and(|ext| ext == "gz") {
        Box::new(BufReader::with_capacity(
            1 << 20,
            MultiGzDecoder::new(reader),
        ))
    } else {
        Box::new(reader)
    };
    let (sender, receiver) = mpsc::sync_channel::<Vec<Vec<u8>>>(64);
    thread::spawn(move || {
        // One Vec per line, for the convenience of an iterator; not a bottleneck.
        let mut lines = ByteLines::new(reader).into_iter().filter_map(Result::ok);
        loop {
            let batch: Vec<Vec<u8>> = lines.by_ref().take(BATCH).collect();
            // an empty batch ends the file; a closed channel, a reader that stopped
            if batch.is_empty() || sender.send(batch).is_err() {
                break;
            }
        }
    });
    Ok(receiver.into_iter())
}

impl Items {
    pub(crate) fn process_entries(
        &mut self,
        string_pool: &mut StringPool,
        path: &Path,
    ) -> Result<()> {
        let mut line_number = 0;
        for mut batch in entry_batches(path)? {
            // lines are parsed in parallel, and made items of in order
            let entries: Vec<_> = batch
                .par_iter_mut()
                .map(|line| to_borrowed_value(line))
                .collect();
            for entry in entries {
                self.process_entry(string_pool, &entry?, line_number);
                line_number += 1;
            }
        }
        Ok(())
    }

    fn process_entry(&mut self, string_pool: &mut StringPool, entry: &Json, line_number: usize) {
        self.total_ok_lines_in_file += 1;
        let calls = etymology_calls(entry);
        let templates: Vec<&Json> = calls.iter().map(AsRef::as_ref).collect();
        for (n, pos) in entry.get_array("pos").into_iter().flatten().enumerate() {
            let item = EntryPos {
                entry,
                pos,
                templates: &templates,
            };
            self.process_item(string_pool, &item, (line_number, n));
        }
    }

    /// The dump's redirects (`wiktionary-entries --redirects`): namespace, title and
    /// target per line.
    pub(crate) fn process_redirects(
        &mut self,
        string_pool: &mut StringPool,
        path: &Path,
    ) -> Result<()> {
        for line in entry_lines(path)? {
            let line = String::from_utf8_lossy(&line);
            let mut fields = line.splitn(3, '\t');
            if let (Some(_ns), Some(title), Some(target)) =
                (fields.next(), fields.next(), fields.next())
            {
                self.process_redirect(string_pool, title, target);
            }
        }
        Ok(())
    }
}

/// The calls of an entry's etymology, in order. A span that continues a transclusion
/// an earlier span opened repeats its calls and is skipped. An etymology template that
/// names a language but no word takes the word from a link template right after it
/// ("From Middle Chinese 中國": `{{der|ja|ltc|-}} {{ltc-l|中國}}`; see `link_term`),
/// unless "+" follows that word.
pub(crate) fn etymology_calls<'a>(entry: &'a Json<'a>) -> Vec<Cow<'a, Json<'a>>> {
    let blocks = entry.get("etymology").and_then(|e| e.get_array("blocks"));
    let mut calls = vec![];
    for block in blocks.into_iter().flatten() {
        let text = block.get_str("text").unwrap_or_default();
        let spans: Vec<&Json> = block
            .get_array("spans")
            .into_iter()
            .flatten()
            .filter(|s| {
                s.get_str("kind") == Some("template") && s.get_bool("continued") != Some(true)
            })
            .collect();
        for (n, &span) in spans.iter().enumerate() {
            let span_calls = span.get_array("calls").map_or(&[][..], |c| &c[..]);
            for (m, call) in span_calls.iter().enumerate() {
                let paired = (m + 1 == span_calls.len())
                    .then(|| spans.get(n + 1))
                    .flatten()
                    .filter(|next| adjacent(text, span, next) && !plus_after(text, next))
                    .and_then(|next| paired_call(call, next));
                calls.push(paired.map_or(Cow::Borrowed(call), Cow::Owned));
            }
        }
    }
    calls
}

/// Whether only whitespace stands between two spans of a block's text.
fn adjacent(text: &str, span: &Json, next: &Json) -> bool {
    match (span.get_usize("end"), next.get_usize("start")) {
        (Some(end), Some(start)) if end <= start => text
            .chars()
            .skip(end)
            .take(start - end)
            .all(char::is_whitespace),
        _ => false,
    }
}

/// Whether the text after a span goes on with "+", so that the span names only the
/// first part of a word the template's language formed ("Borrowed from Sanskrit दीर्घ
/// + आयु"), not the word itself.
fn plus_after(text: &str, span: &Json) -> bool {
    span.get_usize("end").is_some_and(|end| {
        text.chars()
            .skip(end)
            .find(|c| !c.is_whitespace())
            .is_some_and(|c| c == '+')
    })
}

/// An etymology template naming a language but no word (`{{der|ja|ltc|-}}`), given
/// the word by the span after it: the template with that word.
fn paired_call<'a>(call: &Json<'a>, next: &Json<'a>) -> Option<Json<'a>> {
    let name = call.get_str("name")?;
    let derived = EtyMode::from_str(name)
        .is_ok_and(|mode| mode.template_kind() == Some(TemplateKind::Derived));
    let args = call.get("args")?;
    let source = args.get_valid_str("2")?;
    if !derived
        || args.get_valid_term("3").is_some()
        || args.get_valid_str("4").is_some()
        || args.get_valid_str("alt").is_some()
    {
        return None;
    }
    let term = link_term(source, next)?;
    let mut call = call.clone();
    call.get_mut("args")?.insert("3", term).ok()?;
    Some(call)
}

/// The word a span's first call links to, if it is a link template: `{{m}}` or
/// `{{l}}` in the language `source` (its term), or a language's own link template
/// (`{{ltc-l}}`, `{{zh-l}}`, `{{ja-r}}`, `{{he-m}}`, ...: the page Lua linked, which
/// is the entry name; Chinese and Japanese templates link the form a page is filed
/// under, the traditional characters or the kanji, not the kana).
fn link_term(source: &str, span: &Json) -> Option<String> {
    let call = span.get_array("calls")?.first()?;
    let name = call.get_str("name")?;
    let args = call.get("args")?;
    match name {
        "m" | "l" | "m+" | "l+" | "mention" | "link" => (args.get_valid_str("1") == Some(source))
            .then(|| args.get_valid_term("2"))
            .flatten()
            .map(str::to_owned),
        _ if name.ends_with("-l") || name.ends_with("-m") || name == "ja-r" => {
            let page = span
                .get_array("links")
                .and_then(|links| links.first())
                .and_then(|link| link.get_str("page"))
                .filter(|page| !page.contains(':'));
            page.or_else(|| args.get_valid_term("1"))
                .map(|term| clean_template_term(term).to_owned())
                .filter(|term| !term.is_empty() && term != "-")
        }
        _ => None,
    }
}

pub(crate) type Json<'a> = simd_json::value::borrowed::Value<'a>;

/// The most common affix kinds.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub(crate) enum Affix {
    Prefix,
    Suffix,
    Infix,
    Base, // i.e., not an affix
}

pub(crate) trait JsonStr<'a> {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
    fn get_valid_term(&self, key: &str) -> Option<&str>;
    fn get_affix_term(&'a self, key: &str, affix_kind: &Affix) -> Option<Cow<'a, str>>;
}

impl<'a> JsonStr<'a> for Json<'a> {
    /// Return a cleaned version of the str if it exists.
    fn get_valid_str(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }

    /// A stricter version of `get_valid_str` for terms.
    fn get_valid_term(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .map(clean_template_term)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }

    // Used for prefix (suffix) templates to handle the case of a proto-root
    // that ends (begins) with a hyphen being placed in the prefix (suffix) arg
    // position. Usually the hyphen is not included for a prefix (suffix) term
    // in the template, so we want to add it. But in this case the hyphen is
    // already there, correctly, so we don't want to add another.
    /// Get a valid affix term and add hyphen(s) if needed.
    fn get_affix_term(&'a self, key: &str, affix_kind: &Affix) -> Option<Cow<'a, str>> {
        let mut term = Cow::from(self.get_valid_term(key)?);
        match affix_kind {
            Affix::Prefix => {
                if !term.ends_with('-') {
                    term.to_mut().push('-');
                }
            }
            Affix::Suffix => {
                if !term.starts_with('-') {
                    term.to_mut().insert(0, '-');
                }
            }
            Affix::Infix => {
                if !term.starts_with('-') {
                    term.to_mut().insert(0, '-');
                }
                if !term.ends_with('-') {
                    term.to_mut().push('-');
                }
            }
            Affix::Base => {}
        }
        Some(term)
    }
}

/// One part of speech of one entry, with the entry's etymology calls.
pub(crate) struct EntryPos<'a> {
    pub(crate) entry: &'a Json<'a>,
    pub(crate) pos: &'a Json<'a>,
    pub(crate) templates: &'a [&'a Json<'a>],
}

impl Items {
    fn process_item(
        &mut self,
        string_pool: &mut StringPool,
        item: &EntryPos,
        line: (usize, usize),
    ) {
        if let Some(page_term) = item.get_page_term(string_pool)
            && let Some(term) = item.get_canonical_term(string_pool)
            && let Some(lang) = item.get_lang()
            && let Some(pos) = item.get_pos()
            && let Some(gloss) = item.get_gloss(string_pool)
        {
            let ety_label = item.get_ety_label();
            let real = RealItem {
                ety_num: ety_label
                    .as_deref()
                    .and_then(|label| label.parse().ok())
                    .unwrap_or(1),
                ety_label,
                lang,
                term,
                pos: vec![pos],
                gloss: vec![gloss],
                page_term: (page_term != term).then_some(page_term),
                romanization: item.get_romanization(string_pool),
                is_reconstructed: item.is_reconstructed(),
            };
            let (item_id, is_new_ety) = self.add_real(real);
            self.references.cover(item_id, item);
            for id in item.get_ids(lang) {
                self.add_id(item_id, id);
            }
            self.lines.insert(line, item_id);
            self.add_derived(string_pool, item_id, item, lang);
            if is_new_ety {
                // The etymology, roots and the descendants given beside the parts
                // of speech belong to the entry: read once, with its first part
                // of speech.
                let raw_roots = item.get_roots(string_pool, lang);
                if !raw_roots.is_empty() {
                    self.raw_templates
                        .root
                        .insert(item_id, raw_roots.into_boxed_slice());
                }
                if let Some(raw_etymology) = item.get_etymology(string_pool, lang, &self.references)
                {
                    self.raw_templates.ety.insert(item_id, raw_etymology);
                }
                if let Some(raw_descendants) = item.get_entry_descendants(string_pool) {
                    self.raw_templates.desc.insert(item_id, raw_descendants);
                }
            }
            if let Some(raw_descendants) = item.get_descendants(string_pool) {
                // A Descendants section under a part of speech: several parts of
                // speech may each have one; a repeated list is kept once.
                match self.raw_templates.desc.get_mut(&item_id) {
                    Some(existing) if *existing == raw_descendants => {}
                    Some(existing) => {
                        let mut lines = Vec::from(mem::take(&mut existing.lines));
                        lines.extend(Vec::from(raw_descendants.lines));
                        *existing = RawDescendants::from(lines);
                    }
                    None => {
                        self.raw_templates.desc.insert(item_id, raw_descendants);
                    }
                }
            }
        }
    }
}

/// Templates that list terms of the entry's language, as a Derived terms section does.
const COLUMNS: [&str; 9] = [
    "col", "col1", "col2", "col3", "col4", "col5", "col-auto", "der2", "der3",
];

impl Items {
    // The entry's "Derived terms" (same language), which name the item's
    // derivatives as a Descendants section names its descendants.
    fn add_derived(
        &mut self,
        string_pool: &mut StringPool,
        item_id: ItemId,
        item: &EntryPos,
        lang: Lang,
    ) {
        for call in item.pos.get_array("derived").into_iter().flatten() {
            let Some(name) = call.get_str("name") else {
                continue;
            };
            let Some(args) = call.get("args").and_then(|a| a.as_object()) else {
                continue;
            };
            let terms: Vec<&str> = if COLUMNS.contains(&name) || name.starts_with("der") {
                if args.get("1").and_then(|v| v.as_str()).map(str::trim) != Some(lang.code()) {
                    continue;
                }
                (2..)
                    .map_while(|n| args.get(n.to_string().as_str()).and_then(|v| v.as_str()))
                    .collect()
            } else if matches!(name, "l" | "link" | "l-self") {
                args.get("2").and_then(|v| v.as_str()).into_iter().collect()
            } else {
                continue;
            };
            for term in terms {
                let term = clean_template_term(term.trim());
                if term.is_empty() || term == "-" || split_lang_prefix(term).is_some() {
                    continue;
                }
                let langterm = lang.new_langterm(string_pool, term);
                self.raw_templates
                    .derived
                    .entry(item_id)
                    .or_default()
                    .push(langterm);
            }
        }
    }
}

impl EntryPos<'_> {
    pub(crate) fn page(&self) -> Option<&str> {
        self.entry.get_valid_str("page")
    }

    pub(crate) fn lang_heading(&self) -> Option<&str> {
        self.entry.get_valid_str("lang_heading")
    }

    fn get_lang(&self) -> Option<Lang> {
        self.entry.get_valid_str("lang")?.parse().ok()
    }

    // The form of the term used in the page url, e.g. "voco"; a reconstruction's
    // page names it after its language, e.g. Reconstruction:Proto-Germanic/frijaz.
    fn get_page_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        let page = self.page()?;
        let term = if self.is_reconstructed() {
            page.strip_prefix("Reconstruction:")?.split_once('/')?.1
        } else {
            page
        };
        let term = clean_template_term(term);
        (!term.is_empty() && term != "-" && !should_ignore_term(term))
            .then(|| Term::new(string_pool, term))
    }

    // The canonical form of the term, e.g. "vocō": the headword as the page
    // renders it, the form generally used in etymology templates, which
    // Module:languages converts into the page title. See notes.md for more.
    fn get_canonical_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        let page_term = self.get_page_term(string_pool)?;
        let headword = self
            .pos
            .get_array("headword")
            .and_then(|h| h.first())
            .and_then(|h| h.as_str())
            .map(clean_template_term);
        if let Some(headword) = headword
            && !headword.is_empty()
            && !should_ignore_term(headword)
            && is_respelling(headword, page_term.resolve(string_pool))
        {
            return Some(Term::new(string_pool, headword));
        }
        Some(page_term)
    }

    fn get_pos(&self) -> Option<Pos> {
        let pos = self.pos.get_valid_str("pos")?;
        if !should_ignore_pos(pos) {
            return pos.parse().ok();
        }
        None
    }

    // "1", "2", ... for numbered etymologies, or, on some Semitic pages, "1.1",
    // "1.2", ...; none for a single unnumbered "Etymology", or none at all (e.g. a
    // PIE root page with several "Root" sections, see "men-").
    pub(crate) fn get_ety_label(&self) -> Option<Box<str>> {
        let label = self.entry.get_str("ety_label")?.trim();
        (!label.is_empty()).then(|| label.into())
    }

    /// The senses' glosses, each without its leading labels ("(zoology) ...").
    pub(crate) fn glosses(&self) -> impl Iterator<Item = &str> {
        self.pos
            .get_array("senses")
            .into_iter()
            .flatten()
            .filter_map(|sense| sense.get_str("gloss"))
            .map(drop_labels)
            .filter(|gloss| !gloss.is_empty())
    }

    fn get_gloss(&self, string_pool: &mut StringPool) -> Option<Gloss> {
        let first = self.pos.get_array("senses")?.first()?.get_str("gloss")?;
        let gloss = drop_labels(first);
        (!gloss.is_empty()).then(|| Gloss::new(string_pool, gloss))
    }

    fn get_romanization(&self, string_pool: &mut StringPool) -> Option<Term> {
        let tr = self.pos.get_array("headword_tr")?.first()?.as_str()?;
        (!tr.is_empty()).then(|| Term::new(string_pool, tr))
    }

    pub(crate) fn is_reconstructed(&self) -> bool {
        self.entry.get_i64("ns") == Some(118)
    }

    /// The etymology's prose, block by block.
    pub(crate) fn etymology_text(&self) -> Option<String> {
        let blocks = self.entry.get("etymology")?.get_array("blocks")?;
        let text: Vec<&str> = blocks
            .iter()
            .filter_map(|b| b.get_str("text"))
            .filter(|t| !t.is_empty())
            .collect();
        (!text.is_empty()).then(|| text.join("\n"))
    }

    pub(crate) fn categories(&self) -> impl Iterator<Item = &str> {
        self.entry
            .get_array("categories")
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str())
    }
}

/// A gloss without the labels it opens with: "(zoology, figuratively) a wolf" is
/// "a wolf".
fn drop_labels(mut gloss: &str) -> &str {
    gloss = gloss.trim();
    while let Some(rest) = gloss.strip_prefix('(')
        && let Some(close) = rest.find(')')
    {
        gloss = rest[close + 1..].trim_start();
    }
    gloss
}

/// Split `text<a:1><b<c>>` into `text` and the contents of its top-level
/// angle-bracket groups (`a:1`, `b<c>`). An unbalanced tail is dropped.
pub(crate) fn split_modifiers(s: &str) -> (&str, Vec<&str>) {
    let Some(start) = s.find('<') else {
        return (s, vec![]);
    };
    let mut modifiers = vec![];
    let mut depth = 0usize;
    let mut open = start;
    for (i, c) in s.char_indices().skip_while(|&(i, _)| i < start) {
        match c {
            '<' => {
                if depth == 0 {
                    open = i;
                }
                depth += 1;
            }
            '>' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    modifiers.push(&s[open + 1..i]);
                }
            }
            _ => {}
        }
    }
    (&s[..start], modifiers)
}

pub(crate) fn modifier<'a>(modifiers: &[&'a str], name: &str) -> Option<&'a str> {
    modifiers.iter().find_map(|m| {
        m.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(':'))
            .map(str::trim)
    })
}

/// Split a `lang:term` prefix off a term arg, as Module:links and Module:affix
/// accept it (e.g. {{af|pt|a-|grc:χρῶμα|-ia}}). Only checks the syntax of the
/// code; callers decide what an unknown code means.
pub(crate) fn split_lang_prefix(s: &str) -> Option<(&str, &str)> {
    let (code, term) = s.split_once(':')?;
    (code.starts_with(|c: char| c.is_ascii_alphabetic())
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
    .then_some((code, term))
}

/// An explicit affix type, as given by {{af}}'s typeN=/<type:> or
/// {{etymon}}'s <aftype:>.
pub(crate) fn affix_type(name: &str) -> Option<Affix> {
    Some(match name {
        "prefix" | "pre" => Affix::Prefix,
        "suffix" | "suf" => Affix::Suffix,
        "infix" | "in" | "interfix" | "inter" => Affix::Infix,
        "non-affix" | "naf" | "root" => Affix::Base,
        _ => return None,
    })
}

/// A part of speech as templates name it (pos=, <pos:...>), e.g. "verb",
/// "adjective", "diminutive suffix" (the last word naming one).
pub(crate) fn parse_pos(text: &str) -> Option<Pos> {
    text.split(|c: char| !c.is_alphabetic())
        .rev()
        .find_map(|word| {
            let code = match word.to_lowercase().as_str() {
                "noun" | "substantive" => "noun",
                "verb" => "verb",
                "adjective" | "adj" => "adj",
                "adverb" | "adv" => "adv",
                "pronoun" => "pron",
                "numeral" => "num",
                "name" => "name",
                "prefix" => "prefix",
                "suffix" => "suffix",
                "infix" => "infix",
                "interfix" => "interfix",
                "root" => "root",
                _ => return None,
            };
            code.parse().ok()
        })
}

/// The affix type implied by hyphens, as {{af}} infers it.
pub(crate) fn affix_by_hyphens(term: &str) -> Affix {
    match (term.starts_with('-'), term.ends_with('-')) {
        (true, true) => Affix::Infix,
        (true, false) => Affix::Suffix,
        (false, true) => Affix::Prefix,
        (false, false) => Affix::Base,
    }
}

/// Clean a term that appears as a template arg
pub(crate) fn clean_template_term(mut term: &str) -> &str {
    // A term may be a link whose target is the lemma, e.g.
    // {{af|it|la:[[lītus|lītoris]]|-aneo}}.
    if let Some(link) = term.strip_prefix("[[") {
        term = link.split(['|', ']']).next().unwrap_or(link);
    }
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK ($$).
    term = term.strip_prefix('*').unwrap_or(term);
    // Terms in templates may carry inline modifiers, e.g.
    // {{af|en|non-<id:not>|sense}} or {{slbor|fro|la|līber<id:free>}}. Page
    // titles cannot contain "<", so we cut there.
    term = term.split_once('<').map_or(term, |(t, _)| t);
    // Occasionally a term is linked in an ety or descendants section like e.g.
    // on page tuig: {{desc|bor=1|en|twig#Etymology_2}}. Or even more rarely,
    // using #X to link to some other subsection (e.g. language). We just take
    // everything before the first # ($$ do any languages use # that should be
    // exempt from this?)
    term = term.split_once('#').map_or(term, |(t, _)| t);
    // Sometimes senseid is given in parentheses like e.g.
    // {{root|en|ine-pro|*bʰel- (shiny)}}.
    term = term.split_once(" (").map_or(term, |(t, _)| t);
    term
}

/// Whether a "canonical" form plausibly respells the page title (adding
/// diacritics, niqqud, Old Irish "·", etc.): it must have the same number of
/// words. Wiktextract sometimes takes headword-line labels or articles into
/// the canonical form, e.g. "octobre oblique singular or" for Old French
/// octobre ({{fro-noun}}), "the pond", "a abate 3rd conjugation" or
/// "kun # second-person singular imperative of kien".
fn is_respelling(canonical: &str, title: &str) -> bool {
    canonical.split_whitespace().count() == title.split_whitespace().count()
}

// $$ These two functions needs revisiting depending on results.

// We would generally like to ignore phrases, and potentially other things.
//  Barring all phrases may be both too strict and not strict enough. Too
// strict because certain phrases may be relevant for etymologies (i.e. a
// phrase became one word in a daughter language). Not strict enough because
// many phrases are categorized as other pos. See e.g.
// https://en.wiktionary.org/wiki/this,_that,_or_the_other. Ignoring terms
// that contain any ascii punctuation is too strict, as this would ingore
// e.g. affixes with -. Ignoring terms with any ascii whitespace is too
// strict as well, as this would ignore e.g. circumfixes (e.g. "ver- -en").
fn should_ignore_term(term: &str) -> bool {
    term.contains(',')
}

fn should_ignore_pos(pos: &str) -> bool {
    pos.contains("phrase")
}

/// Entries for tests: `with` runs a function on the first part of speech of an entry
/// given as JSON, where an `"etymology_templates"` array may stand for the calls of
/// its etymology.
#[cfg(test)]
pub(crate) mod test_entry {
    use super::*;

    pub(crate) fn with<R>(json: &str, f: impl FnOnce(&EntryPos) -> R) -> R {
        let mut bytes = json.as_bytes().to_vec();
        let entry = to_borrowed_value(&mut bytes).unwrap();
        let mut empty_bytes = b"{}".to_vec();
        let empty = to_borrowed_value(&mut empty_bytes).unwrap();
        let calls = etymology_calls(&entry);
        let templates: Vec<&Json> = match entry.get_array("etymology_templates") {
            Some(calls) => calls.iter().collect(),
            None => calls.iter().map(AsRef::as_ref).collect(),
        };
        let pos = entry
            .get_array("pos")
            .and_then(|p| p.first())
            .unwrap_or(&empty);
        f(&EntryPos {
            entry: &entry,
            pos,
            templates: &templates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_term(page: &str, headword: &str) -> String {
        let json = format!(r#"{{"page": "{page}", "pos": [{{"headword": ["{headword}"]}}]}}"#);
        let mut string_pool = StringPool::new();
        test_entry::with(&json, |e| e.get_canonical_term(&mut string_pool))
            .unwrap()
            .resolve(&string_pool)
            .to_owned()
    }

    #[test]
    fn canonical_terms() {
        assert_eq!(canonical_term("matrix", "mātrīx"), "mātrīx");
        assert_eq!(canonical_term("מילה", "מִלָּה"), "מִלָּה");
        assert_eq!(canonical_term("dorat", "do·rat"), "do·rat");
        assert_eq!(canonical_term("qui docet", "quī docet"), "quī docet");
        // a headword line that is not a respelling of the title
        assert_eq!(canonical_term("pond", "the pond"), "pond");
    }

    #[test]
    fn reconstructions_are_named_without_their_language() {
        let json = r#"{"page": "Reconstruction:Proto-Germanic/frijaz", "ns": 118,
            "pos": [{"headword": ["*frijaz"]}]}"#;
        let mut sp = StringPool::new();
        let (page, canonical, reconstructed) = test_entry::with(json, |e| {
            let page = e.get_page_term(&mut sp).unwrap();
            let canonical = e.get_canonical_term(&mut sp).unwrap();
            (page, canonical, e.is_reconstructed())
        });
        assert_eq!(page.resolve(&sp), "frijaz");
        assert_eq!(canonical.resolve(&sp), "frijaz");
        assert!(reconstructed);
    }

    #[test]
    fn glosses_drop_their_labels() {
        assert_eq!(drop_labels("(zoology, figuratively) a wolf"), "a wolf");
        assert_eq!(drop_labels("(archaic) (rare) a wolf"), "a wolf");
        assert_eq!(drop_labels("a wolf (the animal)"), "a wolf (the animal)");
    }

    #[test]
    fn continued_spans_repeat_no_calls() {
        let json = r#"{"etymology": {"blocks": [
            {"kind": "tree", "text": "", "spans": [{"kind": "template", "start": 0, "end": 0,
                "calls": [{"name": "ety", "args": {"1": "la"}}]}]},
            {"kind": "paragraph", "text": "From x.", "spans": [
                {"kind": "template", "start": 0, "end": 4, "continued": true,
                    "calls": [{"name": "ety", "args": {"1": "la"}}]},
                {"kind": "ref", "start": 7, "note": "cite_note-1"},
                {"kind": "template", "start": 5, "end": 6,
                    "calls": [{"name": "m", "args": {"1": "la", "2": "x"}}]}]}]}}"#;
        let names = test_entry::with(json, |e| {
            e.templates
                .iter()
                .filter_map(|t| t.get_str("name").map(str::to_owned))
                .collect::<Vec<_>>()
        });
        assert_eq!(names, ["ety", "m"]);
    }

    #[test]
    fn a_template_naming_no_word_takes_the_next_link_templates() {
        let third = |text: &str, spans: &str| {
            let json = format!(
                r#"{{"etymology": {{"blocks": [{{"kind": "paragraph", "text": "{text}", "spans": [{spans}]}}]}}}}"#
            );
            test_entry::with(&json, |e| {
                e.templates
                    .first()
                    .and_then(|t| t.get("args"))
                    .and_then(|a| a.get_str("3"))
                    .map(str::to_owned)
            })
        };
        // "From Middle Chinese 中國": the page the link template links
        let ltc = r#"{"kind": "template", "start": 5, "end": 19,
                "calls": [{"name": "der", "args": {"1": "za", "2": "ltc", "3": "-"}}]},
            {"kind": "template", "start": 20, "end": 22,
                "calls": [{"name": "ltc-l", "args": {"1": "中國", "id": "1,1"}}],
                "links": [{"page": "中國", "anchor": "Chinese"}]}"#;
        assert_eq!(
            third("From Middle Chinese 中國.", ltc).as_deref(),
            Some("中國")
        );
        // traditional, not simplified; kanji, not kana
        let zh = r#"{"kind": "template", "start": 5, "end": 13,
                "calls": [{"name": "der", "args": {"1": "wuh", "2": "cmn"}}]},
            {"kind": "template", "start": 14, "end": 26,
                "calls": [{"name": "zh-l", "args": {"1": "幾個"}}],
                "links": [{"page": "幾個", "anchor": "Chinese"}, {"page": "几个", "anchor": "Chinese"}]}"#;
        assert_eq!(
            third("From Mandarin 幾個／几个 (jǐgè).", zh).as_deref(),
            Some("幾個")
        );
        let ja = r#"{"kind": "template", "start": 0, "end": 22,
                "calls": [{"name": "bor", "args": {"1": "en", "2": "ja", "3": ""}}]},
            {"kind": "template", "start": 23, "end": 35,
                "calls": [{"name": "ja-r", "args": {"1": "棒読み", "2": "ぼうよみ"}}],
                "links": [{"page": "棒読み", "anchor": "Japanese"}]}"#;
        assert_eq!(
            third("Borrowed from Japanese 棒読み (bōyomi).", ja).as_deref(),
            Some("棒読み")
        );
        // {{m}} only in the template's language
        let m = |lang: &str| {
            format!(
                r#"{{"kind": "template", "start": 5, "end": 11,
                    "calls": [{{"name": "der", "args": {{"1": "en", "2": "fr", "3": "-"}}}}]}},
                {{"kind": "template", "start": 12, "end": 16,
                    "calls": [{{"name": "m", "args": {{"1": "{lang}", "2": "-obe"}}}}]}}"#
            )
        };
        assert_eq!(
            third("From French -obe.", &m("fr")).as_deref(),
            Some("-obe")
        );
        assert_eq!(third("From French -obe.", &m("la")).as_deref(), Some("-"));
        // not across prose ("From Japanese; compare 棒読み")
        let apart = r#"{"kind": "template", "start": 5, "end": 13,
                "calls": [{"name": "der", "args": {"1": "en", "2": "ja", "3": "-"}}]},
            {"kind": "template", "start": 23, "end": 26,
                "calls": [{"name": "ja-r", "args": {"1": "棒読み"}}],
                "links": [{"page": "棒読み", "anchor": "Japanese"}]}"#;
        assert_eq!(
            third("From Japanese; compare 棒読み.", apart).as_deref(),
            Some("-")
        );
        // nor to the first part of a compound ("Sanskrit दीर्घ + आयु")
        let compound = r#"{"kind": "template", "start": 14, "end": 22,
                "calls": [{"name": "bor", "args": {"1": "ms", "2": "sa", "3": "-"}}]},
            {"kind": "template", "start": 23, "end": 28,
                "calls": [{"name": "m", "args": {"1": "sa", "2": "दीर्घ"}}]},
            {"kind": "template", "start": 31, "end": 34,
                "calls": [{"name": "m", "args": {"1": "sa", "2": "आयु"}}]}"#;
        assert_eq!(
            third("Borrowed from Sanskrit दीर्घ + आयु.", compound).as_deref(),
            Some("-")
        );
    }

    #[test]
    fn descendants_of_several_parts_of_speech_are_joined() {
        let mut sp = StringPool::new();
        let mut items = Items::new().unwrap();
        let pos = |pos: &str, desc: &str| {
            format!(
                r#"{{"pos": "{pos}", "headword": ["māter"], "senses": [{{"depth": 0, "gloss": "mother"}}],
                    "descendants": [{{"text": "x", "calls": [{{"name": "desc", "args": {{"1": "{desc}", "2": "x"}}}}]}}],
                    "derived": [{{"name": "col", "args": {{"1": "la", "2": "māternus"}}}}]}}"#
            )
        };
        let json = format!(
            r#"{{"page": "mater", "lang": "la", "lang_heading": "Latin", "ns": 0, "pos": [{}, {}, {}]}}"#,
            pos("noun", "it"),
            pos("noun", "it"),
            pos("verb", "fr")
        );
        let mut bytes = json.into_bytes();
        let entry = to_borrowed_value(&mut bytes).unwrap();
        for (n, pos) in entry.get_array("pos").unwrap().iter().enumerate() {
            items.process_item(
                &mut sp,
                &EntryPos {
                    entry: &entry,
                    pos,
                    templates: &[],
                },
                (0, n),
            );
        }
        let (_, desc) = items.raw_templates.desc.iter().next().unwrap();
        // the repeated list once, then the other
        assert_eq!(desc.langterms().count(), 2);
        let (_, derived) = items.raw_templates.derived.iter().next().unwrap();
        assert_eq!(derived.len(), 3);
    }

    #[test]
    fn clean_template_terms() {
        assert_eq!("gaberaną", clean_template_term("*gaberaną"));
        assert_eq!("bʰel-", clean_template_term("*bʰel- (shiny)"));
        assert_eq!("twig", clean_template_term("twig#Etymology_2"));
        assert_eq!("līber", clean_template_term("līber<id:free>"));
        assert_eq!("non-", clean_template_term("non-<id:not><t:no>"));
        assert_eq!("lītus", clean_template_term("[[lītus|lītoris]]"));
        assert_eq!("lītus", clean_template_term("[[lītus]]"));
        assert_eq!(
            parse_pos("augmentative suffix").map(Pos::name),
            Some("suffix")
        );
        assert_eq!(parse_pos("adjective").map(Pos::name), Some("adj"));
        assert_eq!(parse_pos("feminine of {{m|pt|novo}}"), None);
    }
}
