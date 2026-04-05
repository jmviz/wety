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
