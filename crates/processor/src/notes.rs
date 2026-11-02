// Footnotes of an etymology section, as credits. wiktextract keeps each <ref>
// of the section as a "ref" template whose argument is the rendered citation,
// and an {{etydate}}'s reference as its "ref" argument. Citations inside the
// prose ("Kroonen (2013)") are not templates; reading them is model work.

use crate::wiktextract_json::{WiktextractJsonItem, WiktextractJsonValidStr};

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

impl WiktextractJsonItem<'_> {
    /// The section's footnotes: a "ref" cites a work for the etymology, an
    /// {{etydate}}'s ref for a date.
    pub(crate) fn get_notes(&self) -> Box<[RawNote]> {
        let Some(templates) = self.json.get_array("etymology_templates") else {
            return Box::default();
        };
        let mut notes: Vec<RawNote> = vec![];
        for template in templates {
            let Some(args) = template.get("args") else {
                continue;
            };
            let found = match template.get_valid_str("name") {
                Some("ref") => args
                    .get_valid_str("1")
                    .and_then(|c| parse_citation(c, Role::Follows)),
                Some("etydate") => args
                    .get_valid_str("ref")
                    .and_then(|c| parse_citation(c, Role::Data)),
                _ => None,
            };
            if let Some(note) = found
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
