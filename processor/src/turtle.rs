use crate::{items::Item, processed::Data, progress_bar, ItemId};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Ok, Result};

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

fn write_list_delim(f: &mut BufWriter<File>, i: usize, len: usize) -> Result<()> {
    if i + 1 < len {
        write!(f, ", ")?;
    } else {
        writeln!(f, " ;")?;
    }
    Ok(())
}

impl Data {
    fn write_turtle_item(&self, f: &mut BufWriter<File>, id: ItemId, item: &Item) -> Result<()> {
        writeln!(f, "{ITEM_PRE}{}", id.index())?;
        let term = item.term().resolve(&self.string_pool);
        write_item_quoted_prop(f, PRED_TERM, term)?;
        write_item_quoted_prop(f, PRED_LANG, item.lang().name())?;

        if let Some(url) = item.url(&self.string_pool) {
            write_item_quoted_prop(f, PRED_URL, &url)?;
        };

        writeln!(f, "  {PRED_ETY_NUM} {} ;", item.ety_num())?;

        if item.is_imputed() {
            writeln!(f, "  {PRED_IS_IMPUTED} true ;")?;
        }
        if item.is_reconstructed() {
            writeln!(f, "  {PRED_IS_RECONSTRUCTED} true ;")?;
        }
        if let Some(pos) = &item.pos() {
            write!(f, "  {PRED_POS} ")?;
            for (p_i, p) in pos.iter().map(|p| p.name()).enumerate() {
                write_quoted_str(f, p)?;
                write_list_delim(f, p_i, pos.len())?;
            }
        };
        if let Some(gloss) = &item.gloss() {
            write!(f, "  {PRED_GLOSS} ")?;
            for (g_i, g) in gloss.iter().enumerate() {
                write_quoted_str(f, &g.to_string(&self.string_pool))?;
                write_list_delim(f, g_i, gloss.len())?;
            }
        }

        if let Some(immediate_ety) = self.graph.get_immediate_ety(id) {
            let mode = immediate_ety.mode.as_ref();
            write_item_quoted_prop(f, PRED_MODE, mode)?;
            if let Some(head) = immediate_ety.head {
                writeln!(f, "  {PRED_HEAD} {head} ;",)?;
            }
            write!(f, "  {PRED_SOURCE} ")?;
            for (e_i, ety_item) in immediate_ety.items.iter().enumerate() {
                write!(
                    f,
                    "[ {PRED_ITEM} {ITEM_PRE}{}; {PRED_ORDER} {e_i} ]",
                    ety_item.index()
                )?;
                write_list_delim(f, e_i, immediate_ety.items.len())?;
            }
        }
        if let Some(progenitors) = self.progenitors.get(&id) {
            if let Some(head) = progenitors.head {
                writeln!(f, "  {PRED_HEAD_PROGENITOR} {ITEM_PRE}{} ;", head.index())?;
            }
            write!(f, "  {PRED_PROGENITOR} ")?;
            for (p_i, progenitor) in progenitors.items.iter().enumerate() {
                write!(f, "{ITEM_PRE}{}", progenitor.index())?;
                write_list_delim(f, p_i, progenitors.items.len())?;
            }
        }
        writeln!(f, ".")?;
        Ok(())
    }

    pub(crate) fn write_turtle(&self, path: &Path) -> Result<()> {
        let mut f = BufWriter::new(File::create(path)?);
        write_prefixes(&mut f)?;
        let n = self.graph.len();
        let pb = progress_bar(n, "Writing RDF to Turtle file data/wety.ttl")?;
        for (id, item) in self.graph.iter() {
            self.write_turtle_item(&mut f, id, item)?;
            pb.inc(1);
        }
        f.flush()?;
        pb.finish();
        Ok(())
    }
}
