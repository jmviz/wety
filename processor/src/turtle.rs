use crate::{items::Item, processed_data::ProcessedData, progress_bar};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Ok, Result};
use urlencoding::encode;

const WIKTIONARY_PRE: &str = "k:";
const WIKTIONARY_URL: &str = "https://en.wiktionary.org/wiki/";
const WIKTIONARY_RECONSTRUCTION_PRE: &str = "r:";
const WIKTIONARY_RECONSTRUCTION_URL: &str = "https://en.wiktionary.org/wiki/Reconstruction:";

const PRED_PRE: &str = "p:";

const ITEM_PRE: &str = "w:";
const PRED_IS_IMPUTED: &str = "p:isImputed";
const PRED_IS_RECONSTRUCTED: &str = "p:isReconstructed";
const PRED_TERM: &str = "p:term";
const PRED_LANG: &str = "p:lang";
const PRED_URL: &str = "p:url";
const PRED_POS: &str = "p:pos";
const PRED_GLOSS: &str = "p:gloss";
const PRED_ETY_NUM: &str = "p:etyNum";
const PRED_SOURCE: &str = "p:source";
const PRED_MODE: &str = "p:mode";
const PRED_HEAD: &str = "p:head";
const PRED_HEAD_PROGENITOR: &str = "p:headProgenitor";
const PRED_PROGENITOR: &str = "p:progenitor";

// These two are used in every blank node defining a source.
const PRED_ITEM: &str = "p:item";
const PRED_ORDER: &str = "p:order";

fn write_prefix(f: &mut BufWriter<File>, prefix: &str, iri: &str) -> Result<()> {
    writeln!(f, "@prefix {prefix} <{iri}> .")?;
    Ok(())
}
fn write_prefixes(f: &mut BufWriter<File>) -> Result<()> {
    write_prefix(f, WIKTIONARY_PRE, WIKTIONARY_URL)?;
    write_prefix(
        f,
        WIKTIONARY_RECONSTRUCTION_PRE,
        WIKTIONARY_RECONSTRUCTION_URL,
    )?;
    write_prefix(f, PRED_PRE, PRED_PRE)?;
    write_prefix(f, ITEM_PRE, ITEM_PRE)?;
    Ok(())
}
// cf. https://www.w3.org/TR/turtle/#turtle-literals
fn write_quoted_str(f: &mut BufWriter<File>, s: &str) -> Result<()> {
    write!(f, "\"")?;
    for c in s.chars() {
        match c {
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            _ => write!(f, "{}", c.encode_utf8(&mut [0; 4]))?,
        };
    }
    write!(f, "\"")?;
    Ok(())
}
fn write_item_quoted_prop(f: &mut BufWriter<File>, pred: &str, obj: &str) -> Result<()> {
    write!(f, "  {pred} ")?;
    write_quoted_str(f, obj)?;
    writeln!(f, " ;")?;
    Ok(())
}
// cf. https://www.w3.org/TR/turtle/#sec-escapes
fn write_local_name(f: &mut BufWriter<File>, s: &str) -> Result<()> {
    for c in s.chars() {
        let mut buf = [0; 4];
        let c_str = c.encode_utf8(&mut buf);
        match c {
            '~' | '.' | '-' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
            | '/' | '?' | '#' | '@' | '%' | '_' => write!(f, "\\{c_str}")?,
            _ => write!(f, "{c_str}")?,
        };
    }
    Ok(())
}

fn write_list_delim(f: &mut BufWriter<File>, i: usize, len: usize) -> Result<()> {
    if i + 1 < len {
        write!(f, ", ")?;
    } else {
        writeln!(f, " ;")?;
    }
    Ok(())
}

impl ProcessedData {
    fn write_turtle_item(&self, f: &mut BufWriter<File>, item: &Item) -> Result<()> {
        writeln!(f, "{ITEM_PRE}{}", item.id)?;
        let term = item.term.resolve(&self.string_pool);
        write_item_quoted_prop(f, PRED_TERM, term)?;
        write_item_quoted_prop(f, PRED_LANG, item.lang.name())?;
        if let Some(page_term) = item.page_term {
            let page_title = page_term.resolve(&self.string_pool);
            let page_lang = item.lang.ety2main();
            let page_lang_name = page_lang.name();
            let (pre, title) = if page_lang.is_reconstructed() {
                (
                    WIKTIONARY_RECONSTRUCTION_PRE,
                    format!("{page_lang_name}/{page_title}"),
                )
            } else {
                (WIKTIONARY_PRE, format!("{page_title}#{page_lang_name}"))
            };
            let title = encode(&title);
            write!(f, "  {PRED_URL} {pre}")?;
            write_local_name(f, &title)?;
            writeln!(f, " ;")?;
        };

        writeln!(f, "  {PRED_ETY_NUM} {} ;", item.ety_num)?;

        if item.is_imputed {
            writeln!(f, "  {PRED_IS_IMPUTED} true ;")?;
        }
        if item.lang.is_reconstructed() {
            writeln!(f, "  {PRED_IS_RECONSTRUCTED} true ;")?;
        }
        if let Some(pos) = &item.pos {
            write!(f, "  {PRED_POS} ")?;
            for (p_i, p) in pos.iter().map(|p| p.name()).enumerate() {
                write!(f, "\"{p}\"")?;
                write_list_delim(f, p_i, pos.len())?;
            }
        };
        if let Some(gloss) = &item.gloss {
            write!(f, "  {PRED_GLOSS} ")?;
            for (g_i, g) in gloss.iter().enumerate() {
                write!(f, "\"{}\"", g.to_string(&self.string_pool))?;
                write_list_delim(f, g_i, gloss.len())?;
            }
        }

        if let Some(immediate_ety) = self.ety_graph.get_immediate_ety(item.id) {
            let mode = immediate_ety.mode.as_ref();
            write_item_quoted_prop(f, PRED_MODE, mode)?;
            writeln!(f, "  {PRED_HEAD} {} ;", immediate_ety.head)?;
            write!(f, "  {PRED_SOURCE} ")?;
            for (e_i, ety_item) in immediate_ety.items.iter().enumerate() {
                write!(
                    f,
                    "[ {PRED_ITEM} {ITEM_PRE}{ety_item}; {PRED_ORDER} {e_i} ]"
                )?;
                write_list_delim(f, e_i, immediate_ety.items.len())?;
            }
        }
        if let Some(progenitors) = self.ety_graph.get_progenitors(item.id) {
            let head = progenitors.head;
            writeln!(f, "  {PRED_HEAD_PROGENITOR} {ITEM_PRE}{head} ;")?;
            write!(f, "  {PRED_PROGENITOR} ")?;
            for (p_i, progenitor) in progenitors.items.iter().enumerate() {
                write!(f, "{ITEM_PRE}{progenitor}")?;
                write_list_delim(f, p_i, progenitors.items.len())?;
            }
        }
        writeln!(f, ".")?;
        Ok(())
    }

    pub(crate) fn write_turtle_file(&self, path: &Path) -> Result<()> {
        let mut f = BufWriter::new(File::create(path)?);
        write_prefixes(&mut f)?;
        let n = self.items.len() + self.ety_graph.imputed_items.len();
        let pb = progress_bar(n, "Writing RDF to Turtle file data/wety.ttl")?;
        for item in self.items.iter() {
            self.write_turtle_item(&mut f, item)?;
            pb.inc(1);
        }
        for item in self.ety_graph.imputed_items.iter() {
            self.write_turtle_item(&mut f, item)?;
            pb.inc(1);
        }
        f.flush()?;
        pb.finish();
        Ok(())
    }
}
