// Human-readable forms of the raw wiktionary etymology template modes.

export function etyModeRep(etyMode: string): string {
  switch (etyMode) {
    case "undefined derivation":
    case "mention":
      return "derived";
    case "morphological derivation":
      return "derivation";
    case "transfix":
      return "transfixation";
    case "suffix":
      return "suffixation";
    case "prefix":
      return "prefixation";
    case "infix":
      return "infixation";
    case "confix":
      return "confixation";
    case "circumfix":
      return "circumfixation";
    case "affix":
      return "affixation";
    default:
      return etyMode;
  }
}

export function etyPrep(etyMode: string): string {
  switch (etyMode) {
    case "derived":
    case "inherited":
    case "borrowed":
    case "back-formation":
    case "undefined derivation":
    case "mention":
      return " from ";
    case "surface analysis":
      return ": ";
    case "vṛddhi":
    case "vṛddhi-ya":
      return " derivative of ";
    case "root":
      return " reflex of ";
    default:
      return " of ";
  }
}

// Short form used inline on tree rows, where "from"/"of" is implied by the
// connector to the parent.
export function etyModeShort(etyMode: string): string {
  switch (etyMode) {
    case "undefined derivation":
    case "mention":
    case "morphological derivation":
      return "derived";
    case "learned borrowing":
      return "learned";
    case "semi-learned borrowing":
      return "semi-learned";
    case "unadapted borrowing":
      return "unadapted";
    case "orthographic borrowing":
      return "orthographic";
    case "surface analysis":
      return "analysable as";
    default:
      return etyMode;
  }
}
