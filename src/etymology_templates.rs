use phf::{phf_ordered_map, phf_ordered_set, OrderedMap, OrderedSet};
use strum::{AsRefStr, EnumString};

// For each of the three maps below, the key is the name of the template as it
// appears in a given etymology section for a given word on wiktionary, as read
// from the wiktextract json. The value is the canonical name used by wety. We
// first define the set MODES that contains all of the unique values from all 3
// maps. REMEMBER, IF EVER ADDING/REMOVING TO/FROM A MAP, YOU NEED TO ALSO
// ADD/REMOVE THE CANONICAL NAME TO/FROM MODES.

// This is the set of canonical names for all templates listed below
pub(crate) static MODE: OrderedSet<&'static str> = phf_ordered_set! {
    "derived", // start derived-type modes
    "inherited",
    "borrowed",
    "learned borrowing",
    "semi-learned borrowing",
    "unadapted borrowing",
    "orthographic borrowing",
    "semantic loan",
    "calque",
    "partial calque",
    "phono-semantic matching",
    "undefined derivation",
    "transliteration",
    "abbreviation", // start abbrev-type modes
    "adverbial accusative",
    "contraction",
    "reduplication",
    "syncopic form",
    "rebracketing",
    "nominalization",
    "ellipsis",
    "acronym",
    "initialism",
    "conversion",
    "clipping",
    "causative",
    "back-formation",
    "deverbal",
    "apocopic form",
    "aphetic form",
    "compound", // start compound-type modes
    "univerbation",
    "transfix",
    "surface analysis",
    "suffix",
    "prefix",
    "infix",
    "confix",
    "circumfix",
    "blend",
    "affix",
    "form", // ad-hoc mode used when term is wiktextract alt or form of another
    "root", // ad-hoc mode used when imputing root source for an item
};

//     "partial calque" => "partial calque", // https://en.wiktionary.org/wiki/Template:partial_calque
//     "pcal" => "partial calque", // shortcut for "partial calque"
//     "pclq" => "partial calque", // shortcut for "partial calque"
//     "phono-semantic matching" => "phono-semantic matching", // https://en.wiktionary.org/wiki/Template:phono-semantic_matching
//     "psm" => "phono-semantic matching", // shortcut for "phono-semantic matching"
//     "undefined derivation" => "undefined derivation", // https://en.wiktionary.org/wiki/Template:undefined_derivation
//     "uder" => "undefined derivation", // shortcut for "undefined derivation"
//     "der?" => "undefined derivation", // shortcut for "undefined derivation"
//     "transliteration" => "transliteration", // https://en.wiktionary.org/wiki/Template:transliteration
//     "translit" => "transliteration", // shortcut for "transliteration"

#[derive(Clone, AsRefStr, EnumString)]
#[strum(use_phf)]
pub(crate) enum EtyMode {
    // start derived-type modes
    #[strum(
        to_string = "derived", // https://en.wiktionary.org/wiki/Template:derived
        serialize = "der", // shortcut for "derived"
        serialize = "der+", // https://en.wiktionary.org/wiki/Template:der%2B
        serialize = "der-lite", // https://en.wiktionary.org/wiki/Template:der-lite
    )]
    Derived,
    #[strum(
        to_string = "inherited", // https://en.wiktionary.org/wiki/Template:inherited
        serialize = "inh", // shortcut for "inherited"
        serialize = "inh+", // https://en.wiktionary.org/wiki/Template:inh%2B
        serialize = "inh-lite", // https://en.wiktionary.org/wiki/Template:inh-lite
    )]
    Inherited,
    #[strum(
        to_string = "borrowed", // https://en.wiktionary.org/wiki/Template:borrowed
        serialize = "bor", // shortcut for "borrowed"
        serialize = "bor+", // https://en.wiktionary.org/wiki/Template:bor%2B
    )]
    Borrowed,
    #[strum(
        to_string = "learned borrowing", // https://en.wiktionary.org/wiki/Template:learned_borrowing
        serialize = "lbor", // shortcut for "learned borrowing"
    )]
    LearnedBorrowing,
    #[strum(
        to_string = "semi-learned borrowing", // https://en.wiktionary.org/wiki/Template:semi-learned_borrowing
        serialize = "slbor", // shortcut for "semi-learned borrowing"
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
    // start abbreviation-type modes
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
    // start compound-type modes

    // "compound" => "compound", // https://en.wiktionary.org/wiki/Template:compound
    // "com" => "compound", // shortcut for "compound"
    // "com+" => "compound", // https://en.wiktionary.org/wiki/Template:com%2B
    // "univerbation" => "univerbation", // https://en.wiktionary.org/wiki/Template:univerbation
    // "univ" => "univerbation", // shortcut for "univerbation"
    // "transfix" => "transfix", // https://en.wiktionary.org/wiki/Template:transfix
    // "surface analysis" => "surface analysis", // https://en.wiktionary.org/wiki/Template:surface_analysis
    // "surf" => "surface analysis", // shortcut for "surface analysis"
    // "suffix" => "suffix", // https://en.wiktionary.org/wiki/Template:suffix
    // "prefix" => "prefix", // https://en.wiktionary.org/wiki/Template:prefix
    // "pre" => "prefix", // shortcut for "prefix"
    // "infix" => "infix", // https://en.wiktionary.org/wiki/Template:infix
    // "confix" => "confix", // https://en.wiktionary.org/wiki/Template:confix
    // "con" => "confix", // shortcut for "confix"
    // "circumfix" => "circumfix", // https://en.wiktionary.org/wiki/Template:circumfix
    // "blend" => "blend", // https://en.wiktionary.org/wiki/Template:blend
    // "affix" => "affix", // https://en.wiktionary.org/wiki/Template:affix
    // "af" => "affix", // shortcut for "affix"
    #[strum(
        to_string = "compound", // https://en.wiktionary.org/wiki/Template:compound
        serialize = "com", // shortcut for "compound"
        serialize = "com+", // https://en.wiktionary.org/wiki/Template:com%2B
    )]
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
    Form, // ad-hoc mode used when term is wiktextract alt or form of another
    Root, // ad-hoc mode used when imputing root source for an item
}

impl EtyMode {
    pub(crate) fn template_type(&self) -> TemplateType {
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
            | EtyMode::Transliteration => TemplateType::Derived,
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
            | EtyMode::ApheticForm => TemplateType::Abbreviation,
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
            | EtyMode::Affix => TemplateType::Compound,
            EtyMode::Root => TemplateType::Root,
            EtyMode::Form => TemplateType::None,
        }
    }
}

pub(crate) enum TemplateType {
    Derived,
    Abbreviation,
    Compound,
    Root,
    None,
}

// Wiktionary etymology template names that will be considered to represent
// the concept "derived from", in a broad sense. They have 3 main parameters:
// "1": lang code of term being described
// "2": lang code of source language
// "3": term in source language (can be optional; sometimes present but = "" or "-")
// "4" or "alt": alternative display form to show for the source term (optional)
// "5" or "t": gloss/translation for the source term (optional)
// "tr": transliteration for the source term (optional)
// "pos": part of speech of the source term (optional)
pub(crate) static DERIVED_TYPE_TEMPLATES: OrderedMap<&'static str, &'static str> = phf_ordered_map! {
    "derived" => "derived", // https://en.wiktionary.org/wiki/Template:derived
    "der" => "derived", // shortcut for "derived"
    "der+" => "derived", // https://en.wiktionary.org/wiki/Template:der%2B
    "der-lite" => "derived", // https://en.wiktionary.org/wiki/Template:der-lite
    "inherited" => "inherited", // https://en.wiktionary.org/wiki/Template:inherited
    "inh" => "inherited", // shortcut for "inherited"
    "inh+" => "inherited", // https://en.wiktionary.org/wiki/Template:inh%2B
    "inh-lite" => "inherited", // https://en.wiktionary.org/wiki/Template:inh-lite
    "borrowed" => "borrowed", // https://en.wiktionary.org/wiki/Template:borrowed
    "bor" => "borrowed", // shortcut for "borrowed"
    "bor+" => "borrowed", // https://en.wiktionary.org/wiki/Template:bor%2B
    "learned borrowing" => "learned borrowing", // https://en.wiktionary.org/wiki/Template:learned_borrowing
    "lbor" => "learned borrowing", // shortcut for "learned borrowing"
    "semi-learned borrowing" => "semi-learned borrowing", // https://en.wiktionary.org/wiki/Template:semi-learned_borrowing
    "slbor" => "semi-learned borrowing", // shortcut for "semi-learned borrowing"
    "unadapted borrowing" => "unadapted borrowing", // https://en.wiktionary.org/wiki/Template:unadapted_borrowing
    "ubor" => "unadapted borrowing", // shortcut for "unadapted borrowing"
    "orthographic borrowing" => "orthographic borrowing", // https://en.wiktionary.org/wiki/Template:orthographic_borrowing
    "obor" => "orthographic borrowing", // shortcut for "orthographic borrowing"
    "semantic loan" => "semantic loan", // https://en.wiktionary.org/wiki/Template:semantic_loan
    "sl" => "semantic loan", // shortcut for "semantic loan"
    "calque" => "calque", // https://en.wiktionary.org/wiki/Template:calque
    "cal" => "calque", // shortcut for "calque"
    "clq" => "calque", // shortcut for "calque"
    "partial calque" => "partial calque", // https://en.wiktionary.org/wiki/Template:partial_calque
    "pcal" => "partial calque", // shortcut for "partial calque"
    "pclq" => "partial calque", // shortcut for "partial calque"
    "phono-semantic matching" => "phono-semantic matching", // https://en.wiktionary.org/wiki/Template:phono-semantic_matching
    "psm" => "phono-semantic matching", // shortcut for "phono-semantic matching"
    "undefined derivation" => "undefined derivation", // https://en.wiktionary.org/wiki/Template:undefined_derivation
    "uder" => "undefined derivation", // shortcut for "undefined derivation"
    "der?" => "undefined derivation", // shortcut for "undefined derivation"
    "transliteration" => "transliteration", // https://en.wiktionary.org/wiki/Template:transliteration
    "translit" => "transliteration", // shortcut for "transliteration"
};

// Wiktionary etymology template names for templates that deal with
// within-language derivation but are not generally of a compounding
// or affixing type. They have only 2 main parameters, the lang code
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
pub(crate) static ABBREV_TYPE_TEMPLATES: OrderedMap<&'static str, &'static str> = phf_ordered_map! {
    "abbrev" => "abbreviation", // https://en.wiktionary.org/wiki/Template:abbrev
    "adverbial accusative" => "adverbial accusative", // https://en.wiktionary.org/wiki/Template:adverbial_accusative
    "contraction" => "contraction", // https://en.wiktionary.org/wiki/Template:contraction
    "contr" => "contraction", // shortcut for "contraction"
    "reduplication" => "reduplication", // https://en.wiktionary.org/wiki/Template:reduplication
    "rdp" => "reduplication", // shortcut for "reduplication"
    "syncopic form" => "syncopic form", // https://en.wiktionary.org/wiki/Template:syncopic_form
    "sync" => "syncopic form", // shortcut for "syncopic form"
    "rebracketing" => "rebracketing", // https://en.wiktionary.org/wiki/Template:rebracketing
    "nom" => "nominalization", // https://en.wiktionary.org/wiki/Template:nom
    "ellipsis" => "ellipsis", // https://en.wiktionary.org/wiki/Template:ellipsis
    "acronym" => "acronym", // https://en.wiktionary.org/wiki/Template:acronym
    "initialism" => "initialism", // https://en.wiktionary.org/wiki/Template:initialism
    "conversion" => "conversion", // https://en.wiktionary.org/wiki/Template:conversion
    "clipping" => "clipping", // https://en.wiktionary.org/wiki/Template:clipping
    "causative" => "causative", // https://en.wiktionary.org/wiki/Template:causative
    "back-formation" => "back-formation", // https://en.wiktionary.org/wiki/Template:back-formation
    "back-form" => "back-formation", // shortcut for "back-formation"
    "bf" => "back-formation", // shortcut for "back-formation"
    "deverbal" => "deverbal", // https://en.wiktionary.org/wiki/Template:deverbal
    "apocopic form" => "apocopic form", // https://en.wiktionary.org/wiki/Template:apocopic_form
    "aphetic form" => "aphetic form", // https://en.wiktionary.org/wiki/Template:aphetic_form
};

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
pub(crate) static COMPOUND_TYPE_TEMPLATES: OrderedMap<&'static str, &'static str> = phf_ordered_map! {
    "compound" => "compound", // https://en.wiktionary.org/wiki/Template:compound
    "com" => "compound", // shortcut for "compound"
    "com+" => "compound", // https://en.wiktionary.org/wiki/Template:com%2B
    "univerbation" => "univerbation", // https://en.wiktionary.org/wiki/Template:univerbation
    "univ" => "univerbation", // shortcut for "univerbation"
    "transfix" => "transfix", // https://en.wiktionary.org/wiki/Template:transfix
    "surface analysis" => "surface analysis", // https://en.wiktionary.org/wiki/Template:surface_analysis
    "surf" => "surface analysis", // shortcut for "surface analysis"
    "suffix" => "suffix", // https://en.wiktionary.org/wiki/Template:suffix
    "prefix" => "prefix", // https://en.wiktionary.org/wiki/Template:prefix
    "pre" => "prefix", // shortcut for "prefix"
    "infix" => "infix", // https://en.wiktionary.org/wiki/Template:infix
    "confix" => "confix", // https://en.wiktionary.org/wiki/Template:confix
    "con" => "confix", // shortcut for "confix"
    "circumfix" => "circumfix", // https://en.wiktionary.org/wiki/Template:circumfix
    "blend" => "blend", // https://en.wiktionary.org/wiki/Template:blend
    "affix" => "affix", // https://en.wiktionary.org/wiki/Template:affix
    "af" => "affix", // shortcut for "affix"
};

// $$ Should {{cognate}} and the like be treated at all?
// https://en.wiktionary.org/wiki/Template:cognate
// https://en.wiktionary.org/wiki/Template:doublet
// https://en.wiktionary.org/wiki/Template:noncognate
// https://en.wiktionary.org/wiki/Template:piecewise_doublet

// $$ What about {{root}} and {{PIE word}}?
// https://en.wiktionary.org/wiki/Template:root
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

// $$ It may turn out that we need to deal specifically with some/many of these:
// https://en.wiktionary.org/wiki/Category:Language-specific_morphology_templates
// https://en.wiktionary.org/wiki/Category:Etymology_templates_by_language
