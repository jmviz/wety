// Footnotes of an etymology section, as credits: each note the section's prose
// cites, from the entry's `notes`. A note whose marker stands inside an
// {{etydate}} supports a date. Citations inside the prose ("Kroonen (2013)") are
// not notes; reading them is model work.

use crate::entries::EntryPos;

use simd_json::ValueAccess;

use std::sync::LazyLock;

use regex::Regex;
use wety_core::assertions::{Role, Work, digest};

/// A footnote: the work it cites, where in it, and what for.
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub(crate) struct RawNote {
    pub(crate) work: Work,
    pub(crate) locator: Option<Box<str>>,
    pub(crate) role: Role,
}

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());
// “catt”, in eDIL: the headword cited in a dictionary
static HEADWORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"“([^”]+)”,? in ").unwrap());
static PAGES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r",\s*(?:pages?|p\.|pp\.|columns?|col\.|§|sections?|volume \S+, pages?)\s+[^,;:]+")
        .unwrap()
});
static YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((?:c\. |circa )?(1[0-9]{3}|20[0-9]{2})[a-z]?[,;)–-]").unwrap());

fn plain(html: &str) -> String {
    let text = TAG.replace_all(html, "");
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace('\u{2060}', "");
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
}

/// A rendered citation read into a work and a locator: the page, or the
/// headword a dictionary is cited for. One work cited from many entries is one
/// work.
pub(crate) fn parse_citation(html: &str, role: Role) -> Option<RawNote> {
    let text = plain(html);
    let mut locators = vec![];
    let mut work = text.clone();
    if let Some(m) = HEADWORD.captures(&text) {
        locators.push(m[1].trim().to_owned());
        work = format!("{}{}", &text[..m.get(0)?.start()], &text[m.get(0)?.end()..]);
    }
    if let Some(m) = PAGES.find(&work) {
        locators.push(m.as_str().trim_start_matches(',').trim().to_owned());
        work = format!("{}{}", &work[..m.start()], &work[m.end()..]);
    }
    let work = work.trim().trim_end_matches([',', '.', ':', ';']).trim();
    if work.is_empty() {
        return None;
    }
    let year = YEAR.captures(work).and_then(|c| c[1].parse::<u16>().ok());
    Some(RawNote {
        work: Work {
            id: format!("work:wiktionary:{}", digest(&serde_json::json!(work))).into(),
            citation: work.into(),
            year,
        },
        locator: (!locators.is_empty()).then(|| locators.join(", ").into()),
        role,
    })
}

impl EntryPos<'_> {
    /// The section's footnotes, each work once.
    pub(crate) fn get_notes(&self) -> Box<[RawNote]> {
        let Some(etymology) = self.entry.get("etymology") else {
            return Box::default();
        };
        let mut roles: Vec<(&str, Role)> = vec![];
        for block in etymology.get_array("blocks").into_iter().flatten() {
            let spans = block
                .get_array("spans")
                .map(Vec::as_slice)
                .unwrap_or_default();
            // the {{etydate}} spans of the block, as (start, end)
            let dates: Vec<(u64, u64)> = spans
                .iter()
                .filter(|s| {
                    s.get_array("calls")
                        .into_iter()
                        .flatten()
                        .any(|c| c.get_str("name") == Some("etydate"))
                })
                .filter_map(|s| Some((s.get_u64("start")?, s.get_u64("end")?)))
                .collect();
            for span in spans.iter().filter(|s| s.get_str("kind") == Some("ref")) {
                let (Some(note), Some(at)) = (span.get_str("note"), span.get_u64("start")) else {
                    continue;
                };
                let role = if dates.iter().any(|&(start, end)| start <= at && at <= end) {
                    Role::Data
                } else {
                    Role::Follows
                };
                if !roles.iter().any(|(n, _)| *n == note) {
                    roles.push((note, role));
                }
            }
        }
        let mut notes: Vec<RawNote> = vec![];
        for body in etymology.get_array("notes").into_iter().flatten() {
            let (Some(id), Some(text)) = (body.get_str("id"), body.get_str("text")) else {
                continue;
            };
            let role = roles
                .iter()
                .find(|(n, _)| *n == id)
                .map_or(Role::Follows, |&(_, r)| r);
            if let Some(note) = parse_citation(text, role)
                && !notes.contains(&note)
            {
                notes.push(note);
            }
        }
        notes.into_boxed_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entries::test_entry;

    #[test]
    fn a_note_inside_an_etydate_supports_the_date() {
        let json = r#"{"etymology": {
            "blocks": [{"kind": "paragraph", "text": "From x, first attested in 1923.", "spans": [
                {"kind": "ref", "start": 6, "note": "cite_note-a"},
                {"kind": "template", "start": 8, "end": 30, "calls": [{"name": "etydate", "args": {"1": "1923"}}]},
                {"kind": "ref", "start": 30, "note": "cite_note-b"}]}],
            "notes": [
                {"id": "cite_note-a", "text": "A. Author (2000), A Book, page 3", "calls": []},
                {"id": "cite_note-b", "text": "B. Writer (1923), Novel", "calls": []}]}}"#;
        let notes = test_entry::with(json, |e| e.get_notes());
        let roles: Vec<_> = notes.iter().map(|n| (&*n.work.citation, n.role)).collect();
        assert_eq!(
            roles,
            [
                ("A. Author (2000), A Book", Role::Follows),
                ("B. Writer (1923), Novel", Role::Data)
            ]
        );
    }

    fn read(html: &str) -> (String, Option<u16>, Option<String>) {
        let note = parse_citation(html, Role::Follows).unwrap();
        (
            note.work.citation.into(),
            note.work.year,
            note.locator.map(String::from),
        )
    }

    #[test]
    fn citations_read_into_works_and_locators() {
        assert_eq!(
            read(
                r#"<span class="cited-source">Gregory Toner, Sharon Arbuthnot, editors (<span class="None" lang="und">2019), “catt”, in <cite>eDIL: Electronic Dictionary of the Irish Language</cite></span></span>"#
            ),
            (
                "Gregory Toner, Sharon Arbuthnot, editors (2019), eDIL: Electronic Dictionary of the Irish Language".into(),
                Some(2019),
                Some("catt".into())
            )
        );
        assert_eq!(
            read(
                r#"<span class="cited-source">Paul Rycaut (<span class="None" lang="und">1678), <cite>Monarchia Tvrecka</cite> (in Polish), page 190</span></span>"#
            ),
            (
                "Paul Rycaut (1678), Monarchia Tvrecka (in Polish)".into(),
                Some(1678),
                Some("page 190".into())
            )
        );
        assert_eq!(
            // with the word joiner Wiktionary puts after a title
            read(&format!(
                r#"<span class="cited-source">“absolutely, adv.”, in <cite>OED Online {}</cite>, Oxford: Oxford University Press, launched 2000.</span>"#,
                '\u{2060}'
            )),
            (
                "OED Online, Oxford: Oxford University Press, launched 2000".into(),
                None,
                Some("absolutely, adv.".into())
            )
        );
    }
}
