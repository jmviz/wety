//! A hand-written page in Parsoid's shape: two numbered Latin etymologies.

use serde_json::{Value, json};
use wety_wiktionary_entries::{
    model::Counts,
    page::{Languages, PageInfo, entries, entry_names},
};

const PAGE: &str = r##"<!DOCTYPE html><html><head></head><body>
<section data-mw-section-id="0"><p>lead</p></section>
<section data-mw-section-id="1"><h2 id="Latin">Latin</h2>
<section data-mw-section-id="2"><h3 id="Etymology_1">Etymology 1</h3>
<ul class="etymonid" data-ety-tree-json='{"term":"canis","lang":"la","children":[]}' about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"ety","href":"./Template:ety"},"params":{"1":{"wt":"la"},"2":{"wt":":inh"},"3":{"wt":"itc-pro:*kwon"}},"i":0}}]}'></ul>
<p about="#mwt1">Inherited.</p>
<p>From <span class="etyl" about="#mwt2" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"inh","href":"./Template:inh"},"params":{"1":{"wt":"la"},"2":{"wt":"itc-pro"},"3":{"wt":"*kwon"}},"i":0}}]}'>Proto-Italic</span><span about="#mwt2"> </span><i about="#mwt2"><a rel="mw:WikiLink" href="./Reconstruction:Proto-Italic/kwon" class="new">*kwon</a></i>, perhaps.<sup typeof="mw:Extension/ref" data-mw='{"name":"ref"}'><a href="./canis#cite_note-1">[1]</a></sup> Compare <i class="Latn mention" lang="grc" about="#mwt3" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"Cog","href":"./Template:cog"},"params":{"1":{"wt":"grc"},"2":{"wt":"κύων"}},"i":0}}]}'><a rel="mw:WikiLink" href="./%CE%BA%CF%8D%CF%89%CE%BD#Ancient_Greek">κύων</a></i>.</p>
<section data-mw-section-id="3"><h4 id="Noun">Noun</h4>
<p><span class="headword-line" about="#mwt4" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"la-noun","href":"./Template:la-noun"},"params":{"1":{"wt":"canis&lt;3&gt;"}},"i":0}}]}'><strong class="Latn headword" lang="la">ca<ruby>nis<rp>(</rp><rt>x</rt><rp>)</rp></ruby></strong> (<span class="headword-tr tr">kanis</span>)</span><link rel="mw:PageProp/Category" href="./Category:Latin_nouns#CANIS"/></p>
<ol><li>dog<ol><li>hound</li></ol></li><li class="senseid" data-id="Q1" about="#mwt5" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"senseid","href":"./Template:senseid"},"params":{"1":{"wt":"la"},"2":{"wt":"Q1"}},"i":0}}," the Dog Star"]}'>the Dog Star</li></ol>
<section data-mw-section-id="4"><h5 id="Descendants">Descendants</h5>
<div about="#mwt6" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"top2","href":"./Template:top2"},"params":{},"i":0}},"\n* ",{"template":{"target":{"wt":"desc","href":"./Template:desc"},"params":{"1":{"wt":"it"},"2":{"wt":"cane"}},"i":1}},"\n* ",{"template":{"target":{"wt":"desctree","href":"./Template:desctree"},"params":{"1":{"wt":"fro"},"2":{"wt":"chien"}},"i":2}},"\n"]}'><ul><li>Italian: <a rel="mw:WikiLink" href="./cane#Italian">cane</a></li><li>Old French: <a rel="mw:WikiLink" href="./chien#Old_French">chien</a><ul><li>French: <a rel="mw:WikiLink" href="./chien#French">chien</a></li></ul></li></ul></div>
</section></section></section>
<section data-mw-section-id="5"><h3 id="Etymology_2">Etymology 2</h3>
<p>Unknown.<link rel="mw:PageProp/Category" href="./Category:Latin_terms_derived_from_the_PIE_root_*kan-" about="#mwt9" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"root","href":"./Template:root"},"params":{"1":{"wt":"la"},"2":{"wt":"ine-pro"},"3":{"wt":"*kan-"}},"i":0}}]}'/></p>
<section data-mw-section-id="6"><h4 id="Pronunciation">Pronunciation</h4>
<section data-mw-section-id="10"><h5 id="Verb">Verb</h5><p><strong class="headword">canis</strong></p><ol><li><span class="form-of-definition use-with-mention">second-person singular of <span class="form-of-definition-link"><i class="Latn mention" lang="la"><a rel="mw:WikiLink" href="./cano#Latin">canō</a></i> <span class="mention-gloss-paren">(</span><span class="mention-gloss">to sing</span><span class="mention-gloss-paren">)</span></span></span></li><li><span class="use-with-mention">imperative of <i class="Latn mention" lang="la"><a rel="mw:WikiLink" href="./cano#Latin">canō</a></i></span></li></ol></section></section>
</section>
<section data-mw-section-id="7"><h3 id="References">References</h3>
<ol class="mw-references references"><li id="cite_note-1"><span class="mw-reference-text">A. Author (2000) <i>A Book</i></span></li></ol>
</section></section>
<section data-mw-section-id="8"><h2 id="Klingon_Unlisted">Klingon Unlisted</h2><section data-mw-section-id="9"><h3 id="Noun_2">Noun</h3><p><strong class="headword">canis</strong></p></section></section>
</body></html>"##;

fn run() -> (Vec<Value>, Counts) {
    let info = PageInfo {
        title: "canis",
        page_id: 7,
        rev: 9,
        ns: 0,
    };
    let mut counts = Counts::default();
    let languages = Languages::from_pairs([("Latin", "la")]);
    let out = entries(&info, PAGE, &languages, &mut counts)
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();
    (out, counts)
}

#[test]
fn numbered_etymologies_are_entries() {
    let (e, counts) = run();
    assert_eq!(e.len(), 3);
    assert_eq!(e[0]["lang"], "la");
    assert_eq!(e[0]["ety_label"], "1");
    assert_eq!(e[1]["ety_label"], "2");
    // Under a Pronunciation heading.
    assert_eq!(e[1]["pos"][0]["pos"], "verb");
    assert_eq!(
        e[1]["pos"][0]["senses"][0]["form_of"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // a template marked only as a use-with-mention ({{eo-form of}})
    assert_eq!(e[1]["pos"][0]["senses"][1]["form_of"][0]["term"], "canō");
    assert_eq!(
        e[1]["pos"][0]["senses"][0]["form_of"][0],
        json!({"lang": "la", "term": "canō", "link": {"page": "cano", "anchor": "Latin"}, "gloss": "to sing"})
    );
    // {{root}} renders only a category; its call is a zero-width span.
    let span = &e[1]["etymology"]["blocks"][0]["spans"][0];
    assert_eq!(
        (span["start"].as_u64(), span["end"].as_u64()),
        (Some(8), Some(8))
    );
    assert_eq!(span["calls"][0]["name"], "root");
    assert_eq!(e[2]["lang"], Value::Null);
    assert_eq!(counts.0["language headings unknown"], 1);
    // The References beside both etymologies cover each.
    assert_eq!(
        e[0]["language_references"]["blocks"][0]["text"],
        "A. Author (2000) A Book"
    );
    assert_eq!(
        e[1]["language_references"]["blocks"][0]["text"],
        "A. Author (2000) A Book"
    );
}

#[test]
fn etymology_prose_keeps_templates_footnotes_and_trees() {
    let (e, _) = run();
    let ety = &e[0]["etymology"];
    assert_eq!(ety["trees"][0]["term"], "canis");
    // The tree's block keeps its call; the prose it writes continues the transclusion.
    assert_eq!(ety["blocks"][0]["kind"], "tree");
    assert_eq!(ety["blocks"][0]["spans"][0]["calls"][0]["name"], "ety");
    assert_eq!(ety["blocks"][1]["text"], "Inherited.");
    assert_eq!(ety["blocks"][1]["spans"][0]["continued"], true);
    let block = &ety["blocks"][2];
    assert_eq!(
        block["text"],
        "From Proto-Italic *kwon, perhaps. Compare κύων."
    );
    let spans = block["spans"].as_array().unwrap();
    // The transclusion continues over its siblings that share its `about`.
    assert_eq!(spans[0]["start"], 5);
    assert_eq!(spans[0]["end"], 23);
    assert_eq!(
        spans[0]["calls"][0],
        json!({"name": "inh", "args": {"1": "la", "2": "itc-pro", "3": "*kwon"}})
    );
    assert_eq!(
        spans[0]["links"][0],
        json!({"page": "Reconstruction:Proto-Italic/kwon", "missing": true})
    );
    assert_eq!(
        spans[1],
        json!({"kind": "ref", "start": 33, "note": "cite_note-1"})
    );
    assert_eq!(spans[2]["calls"][0]["name"], "cog");
    assert_eq!(
        spans[2]["links"][0],
        json!({"page": "κύων", "anchor": "Ancient Greek"})
    );
    assert_eq!(
        spans[2]["mentions"][0],
        json!({"lang": "grc", "term": "κύων", "link": {"page": "κύων", "anchor": "Ancient Greek"}})
    );
    assert_eq!(ety["notes"][0]["text"], "A. Author (2000) A Book");
    assert_eq!(ety["digest"].as_str().unwrap().len(), 32);
}

#[test]
fn senses_headwords_and_categories() {
    let (e, _) = run();
    let noun = &e[0]["pos"][0];
    assert_eq!(noun["headword"], json!(["canis"]));
    assert_eq!(noun["head"][0]["name"], "la-noun");
    assert_eq!(noun["headword_tr"], json!(["kanis"]));
    let senses = noun["senses"].as_array().unwrap();
    assert_eq!(senses.len(), 3);
    assert_eq!(
        (senses[0]["gloss"].as_str(), senses[0]["depth"].as_u64()),
        (Some("dog"), Some(0))
    );
    assert_eq!(
        (senses[1]["gloss"].as_str(), senses[1]["depth"].as_u64()),
        (Some("hound"), Some(1))
    );
    assert_eq!(senses[2]["senseids"], json!(["Q1"]));
    assert!(
        e[0]["categories"]
            .as_array()
            .unwrap()
            .contains(&json!("Latin nouns"))
    );
    // A numbered etymology has its own categories only.
    let second = e[1]["categories"].as_array().unwrap();
    assert!(!second.contains(&json!("Latin nouns")));
    assert!(second.contains(&json!("Latin terms derived from the PIE root *kan-")));
}

#[test]
fn wrapped_descendants_get_their_lines_calls() {
    let (e, _) = run();
    let d = e[0]["pos"][0]["descendants"].as_array().unwrap();
    assert_eq!(d[0]["calls"][0]["args"]["2"], "cane");
    assert_eq!(d[1]["calls"][0]["name"], "desctree");
    // Rendered from another page: text and links, no calls, and what drew it.
    assert_eq!(d[1]["children"][0]["calls"], json!([]));
    assert_eq!(d[1]["children"][0]["via"], "desctree");
    assert_eq!(d[0].get("via"), None);
    assert_eq!(d[1]["children"][0]["links"][0]["page"], "chien");
}

#[test]
fn a_list_whose_nesting_does_not_pair_is_paired_in_order() {
    // "* → Norwegian:" then ":* Bokmål: {{l|nb|jurist}}" nests otherwise in the HTML
    let page = r##"<!DOCTYPE html><html><body>
<section><h2 id="Latin">Latin</h2><section><h3 id="Noun">Noun</h3><p><strong class="headword">iurista</strong></p><ol><li>jurist</li></ol>
<section><h4 id="Descendants">Descendants</h4>
<div about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"col-top","href":"./Template:col-top"},"params":{},"i":0}},"\n* ",{"template":{"target":{"wt":"desc","href":"./Template:desc"},"params":{"1":{"wt":"ca"},"2":{"wt":"jurista"}},"i":1}},"\n* → Norwegian:\n:* Bokmål: ",{"template":{"target":{"wt":"l","href":"./Template:l"},"params":{"1":{"wt":"nb"},"2":{"wt":"jurist"}},"i":2}},"\n* ",{"template":{"target":{"wt":"desc","href":"./Template:desc"},"params":{"1":{"wt":"ru"},"2":{"wt":"юри́ст"},"bor":{"wt":"1"}},"i":3}},"\n"]}'><ul><li>Catalan: jurista</li><li>→ Norwegian:</li></ul><dl><dd><ul><li>Bokmål: jurist</li></ul></dd></dl><ul><li>→ Russian: юри́ст</li></ul></div>
</section></section></section></body></html>"##;
    let info = PageInfo {
        title: "iurista",
        page_id: 1,
        rev: 1,
        ns: 0,
    };
    let languages = Languages::from_pairs([("Latin", "la")]);
    let mut counts = Counts::default();
    let e = entries(&info, page, &languages, &mut counts);
    let d = serde_json::to_value(&e[0].pos[0].descendants).unwrap();
    assert_eq!(d[0]["calls"][0]["args"]["2"], "jurista");
    assert_eq!(d[1]["calls"], json!([]));
    assert_eq!(d[2]["calls"][0]["args"]["2"], "jurist");
    assert_eq!(d[3]["calls"][0]["args"]["2"], "юри́ст");
}

#[test]
fn a_roots_derived_terms_with_descendants_are_read_as_a_tree() {
    let page = r##"<!DOCTYPE html><html><body>
<section><h2 id="Proto-Indo-European">Proto-Indo-European</h2><section><h3 id="Root">Root</h3><p><strong class="headword">*wed-</strong></p><ol><li>water</li></ol>
<section><h4 id="Derived_terms">Derived terms</h4>
<ul><li><i class="mention" lang="ine-pro" about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"l","href":"./Template:l"},"params":{"1":{"wt":"ine-pro"},"2":{"wt":""},"3":{"wt":"*wēd-o-s"}},"i":0}}]}'>*wēd-o-s</i><ul><li><span about="#mwt2" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"desc","href":"./Template:desc"},"params":{"1":{"wt":"gem-pro"},"2":{"wt":"*wētaz"}},"i":0}}]}'>Proto-Germanic: *wētaz</span></li></ul></li></ul>
</section></section></section></body></html>"##;
    let info = PageInfo {
        title: "Reconstruction:Proto-Indo-European/wed-",
        page_id: 1,
        rev: 1,
        ns: 118,
    };
    let languages = Languages::from_pairs([("Proto-Indo-European", "ine-pro")]);
    let mut counts = Counts::default();
    let e = entries(&info, page, &languages, &mut counts);
    let d = serde_json::to_value(&e[0].pos[0].descendants).unwrap();
    assert_eq!(d[0]["calls"][0]["name"], "l");
    assert_eq!(d[0]["children"][0]["calls"][0]["args"]["2"], "*wētaz");
}

/// An unnumbered etymology whose References sit under it and under the part of
/// speech (a list a template rendered whole), and Further reading under the part of
/// speech.
const NESTED: &str = r##"<!DOCTYPE html><html><head></head><body>
<section data-mw-section-id="1"><h2 id="English">English</h2>
<section data-mw-section-id="2"><h3 id="Etymology">Etymology</h3><p>From Welsh.</p>
<section data-mw-section-id="3"><h4 id="References">References</h4><ul><li>Ety Ref</li></ul></section>
</section>
<section data-mw-section-id="4"><h3 id="Noun">Noun</h3><p><strong class="headword">keldyshite</strong></p><ol><li>a mineral</li></ol>
<section data-mw-section-id="5"><h4 id="References_2">References</h4><ul about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"R:mak:Cense:1979","href":"./Template:R:mak:Cense:1979"},"params":{"page":{"wt":"859"}},"i":0}}]}'><li>Noun Ref</li></ul></section>
<section data-mw-section-id="6"><h4 id="Further_reading">Further reading</h4><ul><li>More</li></ul></section>
</section></section>
</body></html>"##;

#[test]
fn references_under_a_part_of_speech_or_an_etymology_are_the_entrys() {
    let info = PageInfo {
        title: "keldyshite",
        page_id: 8,
        rev: 10,
        ns: 0,
    };
    let mut counts = Counts::default();
    let languages = Languages::from_pairs([("English", "en")]);
    let e: Vec<Value> = entries(&info, NESTED, &languages, &mut counts)
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();
    assert_eq!(e.len(), 1);
    let texts: Vec<&str> = e[0]["references"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["Ety Ref", "Noun Ref"]);
    // a list the template rendered whole: its call is the first item's
    assert_eq!(
        e[0]["references"]["blocks"][1]["spans"][0]["calls"][0]["name"],
        "r:mak:Cense:1979"
    );
    assert_eq!(e[0]["further_reading"]["blocks"][0]["text"], "More");
}

/// {{ety}} with no tree drawn: an empty list carries the call, the paragraph after it
/// the text.
const EMPTY_LIST: &str = r##"<!DOCTYPE html><html><head></head><body>
<section data-mw-section-id="1"><h2 id="Latin">Latin</h2>
<section data-mw-section-id="2"><h3 id="Etymology">Etymology</h3>
<ul about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"ety","href":"./Template:ety"},"params":{"1":{"wt":"la"},"2":{"wt":":suf"},"3":{"wt":"pācō"},"4":{"wt":"-tor"}},"i":0}}]}'></ul><p about="#mwt1">From pācō + -tor.</p>
</section>
<section data-mw-section-id="3"><h3 id="Noun">Noun</h3><p><strong class="headword">pācātor</strong></p><ol><li>pacifier</li></ol></section>
</section>
</body></html>"##;

#[test]
fn an_empty_list_leaves_its_call_to_the_text_after_it() {
    let info = PageInfo {
        title: "pacator",
        page_id: 9,
        rev: 11,
        ns: 0,
    };
    let mut counts = Counts::default();
    let languages = Languages::from_pairs([("Latin", "la")]);
    let e: Vec<Value> = entries(&info, EMPTY_LIST, &languages, &mut counts)
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();
    let span = &e[0]["etymology"]["blocks"][0]["spans"][0];
    assert_eq!(span["calls"][0]["name"], "ety");
    assert_eq!(span["continued"], Value::Null);
}

/// {{inh}} showing another form than its term: Spanish anudar from Latin innōdāre.
const SHOWN: &str = r##"<!DOCTYPE html><html><head></head><body>
<section data-mw-section-id="1"><h2 id="Spanish">Spanish</h2>
<section data-mw-section-id="2"><h3 id="Etymology">Etymology</h3>
<p>From <span about="#mwt1" typeof="mw:Transclusion" data-mw='{"parts":[{"template":{"target":{"wt":"inh","href":"./Template:inh"},"params":{"1":{"wt":"es"},"2":{"wt":"la"},"3":{"wt":"innodō"},"4":{"wt":"innōdāre"}},"i":0}}]}'>Latin </span><i class="Latn mention" lang="la" about="#mwt1"><a rel="mw:WikiLink" href="./innodo#Latin">innōdāre</a></i>.</p>
</section>
<section data-mw-section-id="3"><h3 id="Verb">Verb</h3><p><strong class="headword">anudar</strong></p><ol><li>to knot</li></ol></section>
</section>
</body></html>"##;

#[test]
fn a_templates_own_term_names_the_page_it_links() {
    let info = PageInfo {
        title: "anudar",
        page_id: 10,
        rev: 12,
        ns: 0,
    };
    let mut counts = Counts::default();
    let languages = Languages::from_pairs([("Spanish", "es")]);
    let found = entries(&info, SHOWN, &languages, &mut counts);
    let names: Vec<_> = found.iter().flat_map(entry_names).collect();
    assert!(names.contains(&("la".into(), "innōdāre".into(), "innodo".into())));
    assert!(names.contains(&("la".into(), "innodō".into(), "innodo".into())));
}
