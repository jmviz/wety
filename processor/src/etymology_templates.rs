use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString, IntoStaticStr};

#[derive(
    Hash,
    Eq,
    PartialEq,
    Debug,
    Copy,
    Clone,
    AsRefStr,
    IntoStaticStr,
    EnumString,
    Serialize,
    Deserialize,
)]
#[strum(use_phf)]
pub(crate) enum EtyMode {
    // start derived-kind modes
    #[strum(
        to_string = "derived", // https://en.wiktionary.org/wiki/Template:derived
        serialize = "der", // shortcut for "derived"
        // serialize = "der+", // https://en.wiktionary.org/wiki/Template:der%2B
        serialize = "der-lite", // https://en.wiktionary.org/wiki/Template:der-lite
    )]
    Derived,
    #[strum(
        to_string = "inherited", // https://en.wiktionary.org/wiki/Template:inherited
        serialize = "inh", // shortcut for "inherited"
        // serialize = "inh+", // https://en.wiktionary.org/wiki/Template:inh%2B
        serialize = "inh-lite", // https://en.wiktionary.org/wiki/Template:inh-lite
    )]
    Inherited,
    #[strum(
        to_string = "borrowed", // https://en.wiktionary.org/wiki/Template:borrowed
        serialize = "bor", // shortcut for "borrowed"
        // serialize = "bor+", // https://en.wiktionary.org/wiki/Template:bor%2B
    )]
    // The commented-out (der|inh|bor)+ templates above are so because their
    // expansions generate (der|inh|bor) templates. So in the list of ety
    // templates, there will be two templates in succession e.g. bor, bor+. This
    // confuses the ety template processing. Because the + versions never appear
    // without their normal versions, we just ignore them. We keep the
    // commented-out lines to remind us of this in case the way the + templates
    // get expanded/processed by wiktextract changes at some point for some
    // reason.
    Borrowed,
    #[strum(
        to_string = "learned borrowing", // https://en.wiktionary.org/wiki/Template:learned_borrowing
        serialize = "lbor", // shortcut for "learned borrowing"
    )]
    LearnedBorrowing,
    #[strum(
        to_string = "semi-learned borrowing", // https://en.wiktionary.org/wiki/Template:semi-learned_borrowing
        serialize = "slbor", // shortcut for "semi-learned borrowing"
        serialize = "slb", // not a template shortcut, but an arg used in {{desc}}
    )]
    SemiLearnedBorrowing,
    #[strum(
        to_string = "unadapted borrowing", // https://en.wiktionary.org/wiki/Template:unadapted_borrowing
        serialize = "ubor", // shortcut for "unadapted borrowing"
    )]
    UnadaptedBorrowing,
    #[strum(
        to_string = "orthographic borrowing", // https://en.wiktionary.org/wiki/Template:orthographic_borrowing
        serialize = "obor", // shortcut for "orthographic borrowing"
    )]
    OrthographicBorrowing,
    #[strum(
        to_string = "semantic loan", // https://en.wiktionary.org/wiki/Template:semantic_loan
        serialize = "sl", // shortcut for "semantic loan"
        serialize = "sml", // not a template shortcut, but an arg used in {{desc}}
    )]
    SemanticLoan,
    #[strum(
        to_string = "calque", // https://en.wiktionary.org/wiki/Template:calque
        serialize = "cal", // shortcut for "calque"
        serialize = "clq", // shortcut for "calque"
    )]
    Calque,
    #[strum(
        to_string = "partial calque", // https://en.wiktionary.org/wiki/Template:partial_calque
        serialize = "pcal", // shortcut for "partial calque"
        serialize = "pclq", // shortcut for "partial calque"
    )]
    PartialCalque,
    #[strum(
        to_string = "phono-semantic matching", // https://en.wiktionary.org/wiki/Template:phono-semantic_matching
        serialize = "psm", // shortcut for "phono-semantic matching"
    )]
    PhonoSemanticMatching,
    #[strum(
        to_string = "undefined derivation", // https://en.wiktionary.org/wiki/Template:undefined_derivation
        serialize = "uder", // shortcut for "undefined derivation"
        serialize = "der?", // shortcut for "undefined derivation"
    )]
    UndefinedDerivation,
    #[strum(
        to_string = "transliteration", // https://en.wiktionary.org/wiki/Template:transliteration
        serialize = "translit", // shortcut for "transliteration"
    )]
    Transliteration,
    // start abbreviation-kind modes
    #[strum(
        to_string = "abbreviation", // this is not a wiktionary template
        serialize = "abbrev", // https://en.wiktionary.org/wiki/Template:abbrev
    )]
    Abbreviation,
    #[strum(
        to_string = "adverbial accusative", // https://en.wiktionary.org/wiki/Template:adverbial_accusative
    )]
    AdverbialAccusative,
    #[strum(
        to_string = "contraction", // https://en.wiktionary.org/wiki/Template:contraction
        serialize = "contr", // shortcut for "contraction"
    )]
    Contraction,
    #[strum(
        to_string = "reduplication", // https://en.wiktionary.org/wiki/Template:reduplication
        serialize = "rdp", // shortcut for "reduplication"
    )]
    Reduplication,
    #[strum(
        to_string = "syncopic form", // https://en.wiktionary.org/wiki/Template:syncopic_form
        serialize = "sync", // shortcut for "syncopic form"
    )]
    SyncopicForm,
    #[strum(
        to_string = "rebracketing", // https://en.wiktionary.org/wiki/Template:rebracketing
    )]
    Rebracketing,
    #[strum(
        to_string = "nominalization", // https://en.wiktionary.org/wiki/Template:nominalization
        serialize = "nom", // shortcut for "nominalization"
    )]
    Nominalization,
    #[strum(
        to_string = "ellipsis", // https://en.wiktionary.org/wiki/Template:ellipsis
    )]
    Ellipsis,
    #[strum(
        to_string = "acronym", // https://en.wiktionary.org/wiki/Template:acronym
    )]
    Acronym,
    #[strum(
        to_string = "initialism", // https://en.wiktionary.org/wiki/Template:initialism
    )]
    Initialism,
    #[strum(
        to_string = "conversion", // https://en.wiktionary.org/wiki/Template:conversion
    )]
    Conversion,
    #[strum(
        to_string = "clipping", // https://en.wiktionary.org/wiki/Template:clipping
    )]
    Clipping,
    #[strum(
        to_string = "causative", // https://en.wiktionary.org/wiki/Template:causative
    )]
    Causative,
    #[strum(
        to_string = "back-formation", // https://en.wiktionary.org/wiki/Template:back-formation
        serialize = "back-form", // shortcut for "back-formation"
        serialize = "bf", // shortcut for "back-formation"
    )]
    BackFormation,
    #[strum(
        to_string = "deverbal", // https://en.wiktionary.org/wiki/Template:deverbal
    )]
    Deverbal,
    #[strum(
        to_string = "apocopic form", // https://en.wiktionary.org/wiki/Template:apocopic_form
    )]
    ApocopicForm,
    #[strum(
        to_string = "aphetic form", // https://en.wiktionary.org/wiki/Template:aphetic_form
    )]
    ApheticForm,
    // start compound-kind modes
    #[strum(
        to_string = "compound", // https://en.wiktionary.org/wiki/Template:compound
        serialize = "com", // shortcut for "compound"
        // serialize = "com+", // https://en.wiktionary.org/wiki/Template:com%2B
    )]
    // For the commented-out + variant above, see the comment further above
    // about (der|inh|bor)+.
    Compound,
    #[strum(
        to_string = "univerbation", // https://en.wiktionary.org/wiki/Template:univerbation
        serialize = "univ", // shortcut for "univerbation"
    )]
    Univerbation,
    #[strum(
        to_string = "transfix", // https://en.wiktionary.org/wiki/Template:transfix
    )]
    Transfix,
    #[strum(
        to_string = "surface analysis", // https://en.wiktionary.org/wiki/Template:surface_analysis
        serialize = "surf", // shortcut for "surface analysis"
    )]
    SurfaceAnalysis,
    #[strum(
        to_string = "suffix", // https://en.wiktionary.org/wiki/Template:suffix
        serialize = "suf", // shortcut for "suffix" (undocumented, but used)
    )]
    Suffix,
    #[strum(
        to_string = "prefix", // https://en.wiktionary.org/wiki/Template:prefix
        serialize = "pre", // shortcut for "prefix"
    )]
    Prefix,
    #[strum(
        to_string = "infix", // https://en.wiktionary.org/wiki/Template:infix
    )]
    Infix,
    #[strum(
        to_string = "confix", // https://en.wiktionary.org/wiki/Template:confix
        serialize = "con", // shortcut for "confix"
    )]
    Confix,
    #[strum(
        to_string = "circumfix", // https://en.wiktionary.org/wiki/Template:circumfix
    )]
    Circumfix,
    #[strum(
        to_string = "blend", // https://en.wiktionary.org/wiki/Template:blend
    )]
    Blend,
    #[strum(
        to_string = "affix", // https://en.wiktionary.org/wiki/Template:affix
        serialize = "af", // shortcut for "affix"
    )]
    Affix,
    // start vrddhi-kind modes
    #[strum(
        to_string = "vṛddhi", // https://en.wiktionary.org/wiki/Template:vrddhi
        serialize = "vrddhi", // the actual template name (the above is for writing)
        serialize = "vrd", // shortcut
    )]
    Vrddhi,
    #[strum(
        to_string = "vṛddhi-ya", // https://en.wiktionary.org/wiki/Template:vrddhi-ya
        serialize = "vrddhi-ya", // the actual template name (the above is for writing)
        serialize = "vrd-ya", // shortcut
    )]
    VrddhiYa,
    // Start special cases. All of the above are handled in
    // process_json_ety_template. The below are more ad-hoc ones handled/used in
    // various other places.
    #[strum(
        to_string = "root", // this is a wiktionary template, but this is only used for writing
    )]
    Root, // ad-hoc mode used when imputing root source for an item
    #[strum(
        to_string = "form", // not a wiktionary template, only used for writing
    )]
    Form, // ad-hoc mode used when term is wiktextract alt or form of another
    #[strum(
        to_string = "morphological derivation", // not a wiktionary template, only used for writing
    )]
    // ad-hoc mode used for terms listed in descendants trees of proto-languages
    // which are morphologically derived within the language, e.g. from a root
    // to a noun
    MorphologicalDerivation,
    #[strum(
        to_string = "mention", // https://en.wiktionary.org/wiki/Template:mention
        serialize = "m", // shortcut for "mention"
    )]
    // This is decidedly not an ety mode. But it is a template that is
    // commonly used in ety sections, which often indicates ety relations.
    // Sometimes this is an improper use where a real ety mode specifying
    // template could have been used, and sometimes it is used when no common
    // ety mode template applies. The latter case can be seen e.g. here:
    // https://en.wiktionary.org/wiki/fortuitus#Latin. The ety section contains
    // a single {{m}} template linking to Latin "fors". Presumably this is done
    // because "fortuitus" is a morphological derivation of "fors" and not
    // ~derived~ in the wiktionary ety template sense of descending-from.
    Mention,
}

/// Used to determine how to handle an ety mode template within `process_json_ety_template`
#[derive(PartialEq)]
pub(crate) enum TemplateKind {
    // Wiktionary etymology template names that will be considered to represent
    // the concept "derived from", in a broad sense. They have 3 main parameters:
    // "1": lang code of term being described
    // "2": lang code of source language
    // "3": term in source language (can be optional; sometimes present but = "" or "-")
    // "4" or "alt": alternative display form to show for the source term (optional)
    // "5" or "t": gloss/translation for the source term (optional)
    // "tr": transliteration for the source term (optional)
    // "pos": part of speech of the source term (optional)
    Derived,
    // Wiktionary etymology template names for templates that deal with
    // within-language derivation but are not generally of a compounding
    // or affixing kind. They have only 2 main parameters, the lang code
    // and the source term:
    // "1": lang code of term being described
    // "2": source term (optional)
    // "3" or "alt": alternative display form to show for the source term (optional)
    // "4" or "t": gloss/translation for the source term (optional)
    // "tr": transliteration for the source term (optional)
    // "pos": part of speech for the source term (optional)
    // $$ A number of these (e.g. contraction, rebracketing, ellipsis,
    // $$ acronym, initialism)
    // $$ have source "term" that is often multiple individual terms
    // $$ that together do not have a term entry.
    Abbreviation,
    // Wiktionary etymology template names for templates that deal with
    // with compounding/affixing etc. They have up to N main parameters, the first
    // being the lang code, and the rest being the source terms:
    // "1": lang code of term being described
    // "2"--"N": N-1 source terms (optional)
    // "altn": alternative display form to show for source term given in arg n+1 (optional)
    // "tn": gloss/translation for source term given in arg n+1 (optional)
    // "trn": transliteration for source term given in arg n+1 (optional)
    // "posn": part of speech for source term given in arg n+1 (optional)
    // Some of these templates have optional "lang1", "lang2", etc. arguments,
    // which are the lang codes of the source terms. We handle this.
    Compound,
    // Wiktionary etymology template names for templates that deal with with
    // vrddhi derivates. These templates are unusual in that the "1" arg is not
    // the lang code of the term being described. They have two main parameters:
    // "1": lang code of source language
    // "2": term in source language
    Vrddhi,
}

impl EtyMode {
    pub(crate) fn template_kind(self) -> Option<TemplateKind> {
        match self {
            EtyMode::Derived
            | EtyMode::Inherited
            | EtyMode::Borrowed
            | EtyMode::LearnedBorrowing
            | EtyMode::SemiLearnedBorrowing
            | EtyMode::UnadaptedBorrowing
            | EtyMode::OrthographicBorrowing
            | EtyMode::SemanticLoan
            | EtyMode::Calque
            | EtyMode::PartialCalque
            | EtyMode::PhonoSemanticMatching
            | EtyMode::UndefinedDerivation
            | EtyMode::Transliteration => Some(TemplateKind::Derived),
            EtyMode::Abbreviation
            | EtyMode::AdverbialAccusative
            | EtyMode::Contraction
            | EtyMode::Reduplication
            | EtyMode::SyncopicForm
            | EtyMode::Rebracketing
            | EtyMode::Nominalization
            | EtyMode::Ellipsis
            | EtyMode::Acronym
            | EtyMode::Initialism
            | EtyMode::Conversion
            | EtyMode::Clipping
            | EtyMode::Causative
            | EtyMode::BackFormation
            | EtyMode::Deverbal
            | EtyMode::ApocopicForm
            | EtyMode::ApheticForm => Some(TemplateKind::Abbreviation),
            EtyMode::Compound
            | EtyMode::Univerbation
            | EtyMode::Transfix
            | EtyMode::SurfaceAnalysis
            | EtyMode::Suffix
            | EtyMode::Prefix
            | EtyMode::Infix
            | EtyMode::Confix
            | EtyMode::Circumfix
            | EtyMode::Blend
            | EtyMode::Affix => Some(TemplateKind::Compound),
            EtyMode::Vrddhi | EtyMode::VrddhiYa => Some(TemplateKind::Vrddhi),
            // the other EtyMode variants are special cases that are not handled
            // in process_json_ety_template
            _ => None,
        }
    }

    // pub(crate) fn has_ambiguous_head(self) -> bool {
    //     matches!(
    //         self,
    //         EtyMode::Compound | EtyMode::Univerbation | EtyMode::SurfaceAnalysis | EtyMode::Blend
    //     )
    // }

    pub(crate) fn as_str(self) -> &'static str {
        self.into()
    }
}

// $$ Should {{cognate}} and the like be treated at all?
// https://en.wiktionary.org/wiki/Template:cognate
// https://en.wiktionary.org/wiki/Template:doublet
// https://en.wiktionary.org/wiki/Template:noncognate
// https://en.wiktionary.org/wiki/Template:piecewise_doublet

// $$ What about {{PIE word}}?
// https://en.wiktionary.org/wiki/Template:PIE_word
// https://en.wiktionary.org/wiki/Template:word

// $$ What about any of these ety templates? They have different params and/or
// $$ would require additional logic to handle:
// https://en.wiktionary.org/wiki/Template:hyperthesis
// https://en.wiktionary.org/wiki/Template:metathesis
// https://en.wiktionary.org/wiki/Template:pseudo-loan
// https://en.wiktionary.org/wiki/Template:onomatopoeic
// https://en.wiktionary.org/wiki/Template:named-after
// https://en.wiktionary.org/wiki/Template:internationalism
// https://en.wiktionary.org/wiki/Template:coinage

// $$ What about these form-of templates? We handle a couple, are any of the
// others used often in ety sections?
// https://en.wiktionary.org/wiki/Category:Form-of_templates

// $$ It may turn out that we need to deal specifically with some/many of these:
// https://en.wiktionary.org/wiki/Category:Language-specific_morphology_templates
// https://en.wiktionary.org/wiki/Category:Etymology_templates_by_language
