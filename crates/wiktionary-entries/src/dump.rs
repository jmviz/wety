//! The pages-articles dump: what the Enterprise snapshot leaves out (redirects).

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::Result;
use bzip2::read::MultiBzDecoder;

/// A redirect page: its namespace, title and target.
#[derive(Debug, PartialEq)]
pub struct Redirect {
    pub ns: i64,
    pub title: String,
    pub target: String,
}

fn tag<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = line.find(&open)? + open.len();
    let end = line[start..].find(&close)? + start;
    Some(&line[start..end])
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Redirects in the namespaces given, read from a pages-articles dump (`.xml.bz2`) or
/// any reader of its XML.
pub fn redirects(reader: impl BufRead, namespaces: &[i64]) -> Result<Vec<Redirect>> {
    let mut out = vec![];
    let (mut title, mut ns, mut target) = (None, None, None);
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim_start();
        if trimmed.starts_with("<page>") {
            (title, ns, target) = (None, None, None);
        } else if trimmed.starts_with("<title>") {
            title = tag(trimmed, "title").map(unescape);
        } else if trimmed.starts_with("<ns>") {
            ns = tag(trimmed, "ns").and_then(|n| n.parse().ok());
        } else if let Some(rest) = trimmed.strip_prefix("<redirect title=\"") {
            target = rest.split('"').next().map(unescape);
        } else if trimmed.starts_with("</page>")
            && let (Some(t), Some(n), Some(to)) = (title.take(), ns, target.take())
            && namespaces.contains(&n)
        {
            out.push(Redirect {
                ns: n,
                title: t,
                target: to,
            });
        }
    }
    Ok(out)
}

pub fn redirects_in(path: &Path, namespaces: &[i64]) -> Result<Vec<Redirect>> {
    let file = BufReader::with_capacity(1 << 20, File::open(path)?);
    redirects(
        BufReader::with_capacity(1 << 20, MultiBzDecoder::new(file)),
        namespaces,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_pages() {
        let xml = r#"<mediawiki>
  <page>
    <title>Reconstruction:Proto-Germanic/frijô</title>
    <ns>118</ns>
    <id>91</id>
    <redirect title="Reconstruction:Proto-Germanic/frijaz &amp; co" />
  </page>
  <page>
    <title>Template:x</title>
    <ns>10</ns>
    <redirect title="Template:y" />
  </page>
  <page>
    <title>free</title>
    <ns>0</ns>
  </page>
</mediawiki>
"#;
        assert_eq!(
            redirects(xml.as_bytes(), &[0, 118]).unwrap(),
            vec![Redirect {
                ns: 118,
                title: "Reconstruction:Proto-Germanic/frijô".into(),
                target: "Reconstruction:Proto-Germanic/frijaz & co".into(),
            }]
        );
    }
}
