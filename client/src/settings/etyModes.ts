export interface EtyModeGroup {
  label: string;
  modes: string[];
}

export const etyModeGroups: EtyModeGroup[] = [
  {
    label: "Derivation",
    modes: [
      "derived",
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
    ],
  },
  {
    label: "Morphological",
    modes: [
      "abbreviation",
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
    ],
  },
  {
    label: "Compounding",
    modes: [
      "compound",
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
    ],
  },
  {
    label: "Vrddhi",
    modes: ["vṛddhi", "vṛddhi-ya"],
  },
  {
    label: "Other",
    modes: ["root", "form", "morphological derivation", "mention"],
  },
];

export const allEtyModes: string[] = etyModeGroups.flatMap((g) => g.modes);

// The few kinds of connection a reader filters by. Every mode belongs to one;
// the fine-grained modes stay one disclosure away.
export interface EtyModeCategory {
  label: string;
  modes: string[];
}

const grouped = (label: string) => etyModeGroups.find((g) => g.label === label)!.modes;
const loanModes = [
  "borrowed",
  "learned borrowing",
  "semi-learned borrowing",
  "unadapted borrowing",
  "orthographic borrowing",
  "semantic loan",
  "calque",
  "partial calque",
  "phono-semantic matching",
  "transliteration",
];
const compoundModes = ["compound", "univerbation", "blend", "surface analysis"];

export const etyModeCategories: EtyModeCategory[] = [
  { label: "inherited", modes: ["inherited"] },
  { label: "borrowed", modes: loanModes },
  {
    label: "formed",
    modes: [
      ...grouped("Morphological"),
      ...grouped("Compounding").filter((m) => !compoundModes.includes(m)),
      ...grouped("Vrddhi"),
      "morphological derivation",
      "root",
    ],
  },
  { label: "compounds", modes: compoundModes },
  // Wiktionary's unspecified "derived from" and the like.
  {
    label: "other",
    modes: allEtyModes.filter(
      (m) =>
        m !== "inherited" &&
        !loanModes.includes(m) &&
        !compoundModes.includes(m) &&
        !grouped("Morphological").includes(m) &&
        !grouped("Compounding").includes(m) &&
        !grouped("Vrddhi").includes(m) &&
        m !== "morphological derivation" &&
        m !== "root"
    ),
  },
];
