use crate::{
    etylang2lang, is_reconstructed_lang,
    lang_phf::LANG_CODE2NAME,
    phf_ext::{OrderedMapExt, OrderedSetExt},
    pos_phf::POS,
    progress_bar,
    raw_items::RawItem,
    ProcessedData,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
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
const PRED_GLOSS_NUM: &str = "p:glossNum";
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
fn write_item(
    f: &mut BufWriter<File>,
    data: &ProcessedData,
    item: &Rc<RawItem>,
    has_multi_gloss: bool,
) -> Result<()> {
    writeln!(f, "{ITEM_PRE}{}", item.i)?;
    let term = data.string_pool.resolve(item.term);
    write_item_quoted_prop(f, PRED_TERM, term)?;
    let language = LANG_CODE2NAME.get_expected_index_value(item.lang)?;
    write_item_quoted_prop(f, PRED_LANG, language)?;
    if let Some(page_title) = item.page_title {
        let page_title = data.string_pool.resolve(page_title);
        let page_lang_index = etylang2lang(item.lang);
        let page_lang = LANG_CODE2NAME.get_expected_index_value(page_lang_index)?;
        let (pre, title) = if is_reconstructed_lang(page_lang_index) {
            (
                WIKTIONARY_RECONSTRUCTION_PRE,
                format!("{page_lang}/{page_title}"),
            )
        } else {
            (WIKTIONARY_PRE, format!("{page_title}#{page_lang}"))
        };
        let title = encode(&title);
        write!(f, "  {PRED_URL} {pre}")?;
        write_local_name(f, &title)?;
        writeln!(f, " ;")?;
    };
    if let Some(pos) = item.pos {
        let pos = POS.get_expected_index_key(pos)?;
        write_item_quoted_prop(f, PRED_POS, pos)?;
    };
    if item.is_imputed {
        writeln!(f, "  {PRED_IS_IMPUTED} true ;")?;
    }
    if item.is_reconstructed {
        writeln!(f, "  {PRED_IS_RECONSTRUCTED} true ;")?;
    }
    if let Some(gloss) = item.gloss {
        let gloss = data.string_pool.resolve(gloss);
        write_item_quoted_prop(f, PRED_GLOSS, gloss)?;
    }
    if let Some(ety_num) = item.ety_num {
        writeln!(f, "  {PRED_ETY_NUM} {ety_num} ;")?;
    }
    if has_multi_gloss {
        writeln!(f, "  {PRED_GLOSS_NUM} {} ;", item.gloss_num)?;
    }
    if let Some(immediate_ety) = data.ety_graph.get_immediate_ety(item) {
        let mode = immediate_ety.mode.as_ref();
        write_item_quoted_prop(f, PRED_MODE, mode)?;
        writeln!(f, "  {PRED_HEAD} {} ;", immediate_ety.head)?;
        write!(f, "  {PRED_SOURCE} ")?;
        for (e_i, ety_item) in immediate_ety.items.iter().enumerate() {
            write!(
                f,
                "[ {PRED_ITEM} {ITEM_PRE}{}; {PRED_ORDER} {} ]",
                ety_item.i, e_i
            )?;
            if e_i + 1 < immediate_ety.items.len() {
                write!(f, ", ")?;
            } else {
                writeln!(f, " ;")?;
            }
        }
    }
    if let Some(progenitors) = data.ety_graph.get_progenitors(item) {
        let head = progenitors.head.i;
        writeln!(f, "  {PRED_HEAD_PROGENITOR} {ITEM_PRE}{head} ;")?;
        write!(f, "  {PRED_PROGENITOR} ")?;
        for (p_i, progenitor) in progenitors.items.iter().enumerate() {
            write!(f, "{ITEM_PRE}{}", progenitor.i)?;
            if p_i + 1 < progenitors.items.len() {
                write!(f, ", ")?;
            } else {
                writeln!(f, " ;")?;
            }
        }
    }
    writeln!(f, ".")?;
    Ok(())
}

pub(crate) fn write_turtle_file(data: &ProcessedData, path: &Path) -> Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    write_prefixes(&mut f)?;
    let n = data.items.n + data.ety_graph.imputed_items.n;
    let pb = progress_bar(n, "Writing RDF to Turtle file data/wety.ttl")?;
    for lang_map in data.items.term_map.values() {
        for ety_map in lang_map.values() {
            for pos_map in ety_map.values() {
                for gloss_map in pos_map.values() {
                    for item in gloss_map.values() {
                        write_item(&mut f, data, item, gloss_map.len() > 1)?;
                        pb.inc(1);
                    }
                }
            }
        }
    }
    for lang_map in data.ety_graph.imputed_items.term_map.values() {
        for item in lang_map.values() {
            write_item(&mut f, data, item, false)?;
            pb.inc(1);
        }
    }
    f.flush()?;
    pb.finish();
    Ok(())
}
