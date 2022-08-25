use crate::{
    etymology_templates::MODE, lang::LANG_CODE2NAME, pos::POS, Item, OrderedMapExt, OrderedSetExt,
    Processor,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
};

use anyhow::{Ok, Result};
use indicatif::{ProgressBar, ProgressStyle};

const PRED_PRE: &str = "p:";

const ITEM_PRE: &str = "w:";
const PRED_IS_IMPUTED: &str = "p:isImputed";
const PRED_TERM: &str = "p:term";
const PRED_LANG: &str = "p:lang";
const PRED_POS: &str = "p:pos";
const PRED_GLOSS: &str = "p:gloss";
const PRED_ETY_NUM: &str = "p:etyNum";
const PRED_GLOSS_NUM: &str = "p:glossNum";
const PRED_SOURCE: &str = "p:source";
const PRED_MODE: &str = "p:mode";
const PRED_HEAD: &str = "p:head";

const SOURCE_NODE_PRE: &str = "s:";
const PRED_ITEM: &str = "p:item";
const PRED_ORDER: &str = "p:order";

fn write_prefix(f: &mut BufWriter<File>, prefix: &str, iri: &str) -> Result<()> {
    writeln!(f, "@prefix {} <{}> .", prefix, iri)?;
    Ok(())
}
fn write_prefixes(f: &mut BufWriter<File>) -> Result<()> {
    write_prefix(f, PRED_PRE, PRED_PRE)?;
    write_prefix(f, ITEM_PRE, ITEM_PRE)?;
    write_prefix(f, SOURCE_NODE_PRE, SOURCE_NODE_PRE)?;
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
fn write_item(
    f: &mut BufWriter<File>,
    data: &Processor,
    item: &Item,
    has_multi_ety: bool,
    has_multi_gloss: bool,
) -> Result<()> {
    writeln!(f, "{ITEM_PRE}{}", item.i)?;
    let term = data.string_pool.resolve(item.term);
    write_item_quoted_prop(f, PRED_TERM, term)?;
    let language = LANG_CODE2NAME.get_expected_index_value(item.lang)?;
    write_item_quoted_prop(f, PRED_LANG, language)?;
    let pos = POS.get_expected_index_key(item.pos)?;
    write_item_quoted_prop(f, PRED_POS, pos)?;
    if item.is_imputed {
        writeln!(f, "  {PRED_IS_IMPUTED} true ;")?;
    }
    if let Some(gloss) = item.gloss {
        let gloss = data.string_pool.resolve(gloss);
        write_item_quoted_prop(f, PRED_GLOSS, gloss)?;
    }
    if has_multi_ety {
        writeln!(f, "  {PRED_ETY_NUM} {} ;", item.ety_num)?;
    }
    if has_multi_gloss {
        writeln!(f, "  {PRED_GLOSS_NUM} {} ;", item.gloss_num)?;
    }
    if let Some(source) = data.sources.get(item) {
        let mode = MODE.get_expected_index_key(source.mode)?;
        write_item_quoted_prop(f, PRED_MODE, mode)?;
        writeln!(f, "  {PRED_HEAD} {} ;", source.head)?;
        write!(f, "  {PRED_SOURCE} ")?;
        for (s_i, source_item) in source.items.iter().enumerate() {
            write!(
                f,
                "[ {PRED_ITEM} {ITEM_PRE}{}; {PRED_ORDER} {} ]",
                source_item.i, s_i
            )?;
            if s_i + 1 < source.items.len() {
                write!(f, ", ")?;
            } else {
                writeln!(f, " ;")?;
            }
        }
    }
    writeln!(f, ".")?;
    Ok(())
}

pub(crate) fn write_turtle_file(data: &Processor, path: &str) -> Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    write_prefixes(&mut f)?;
    let n = u64::try_from(data.items.n + data.sources.imputed_items.n)?;
    let pb = ProgressBar::new(n);
    pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
    for lang_map in data.items.term_map.values() {
        for ety_map in lang_map.values() {
            for (_, pos_map) in ety_map.values() {
                for gloss_map in pos_map.values() {
                    for item in gloss_map.values() {
                        write_item(&mut f, data, item, ety_map.len() > 1, gloss_map.len() > 1)?;
                        pb.inc(1);
                    }
                }
            }
        }
    }
    for lang_map in data.sources.imputed_items.term_map.values() {
        for item in lang_map.values() {
            write_item(&mut f, data, item, false, false)?;
            pb.inc(1);
        }
    }
    f.flush()?;
    pb.finish();
    Ok(())
}
