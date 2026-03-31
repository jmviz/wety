export interface Lang {
  id: number;
  name: string;
}

export interface Item {
  id: number;
  etyNum: number;
  lang: Lang;
  term: string;
  imputed: boolean;
  reconstructed: boolean;
  url: string | null;
  pos: string[] | null;
  gloss: string[] | null;
  romanization: string | null;
}

export function term(item: Item): string {
  return item.reconstructed ? `*${item.term}` : item.term;
}

export interface Etymology {
  item: Item;
  etyMode: string | null;
  etyOrder: number;
  parents: Etymology[];
  langDistance: number;
}

export interface OtherParent {
  item: Item;
  etyOrder: number;
  langDistance: number;
}

export interface Descendants {
  item: Item;
  children: Descendants[];
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  parentEtyOrder: number | null;
}

export interface AncestralLine {
  item: Item;
  ancestralLine: AncestralLine | null;
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  etyOrder: number | null;
}

export interface InterLangDescendants {
  item: Item;
  parent: AncestralLine | null;
  children: InterLangDescendants[];
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  parentEtyOrder: number | null;
}

export enum TreeKind {
  Cognates = "Cognates",
  Etymology = "Etymology",
  Descendants = "Descendants",
}

export class TreeRequest {
  lang: Lang;
  item: Item;
  descLangs: Lang[];
  kind: TreeKind;

  constructor(lang: Lang, item: Item, descLangs: Lang[], kind: TreeKind) {
    this.lang = lang;
    this.item = item;
    this.descLangs = descLangs;
    this.descLangs.sort((a, b) => a.id - b.id);
    this.kind = kind;
  }

  apiPath(): string {
    switch (this.kind) {
      case TreeKind.Cognates:
        return `/cognates/${this.item.id}?distLang=${
          this.item.lang.id
        }&${this.descLangs.map((lang) => `descLang=${lang.id}`).join("&")}`;
      case TreeKind.Etymology:
        return `/etymology/${this.item.id}`;
      case TreeKind.Descendants:
        return `/descendants/${this.item.id}?distLang=${
          this.item.lang.id
        }&${this.descLangs.map((lang) => `descLang=${lang.id}`).join("&")}`;
    }
  }

  url(): string {
    return `${process.env.REACT_APP_API_BASE_URL}${this.apiPath()}`;
  }

  static parsePath(
    path: string
  ): {
    kind: TreeKind;
    itemId: number;
    distLangId: number;
    descLangIds: number[];
  } | null {
    const qIdx = path.indexOf("?");
    const pathname = qIdx >= 0 ? path.slice(0, qIdx) : path;
    const search = qIdx >= 0 ? path.slice(qIdx) : "";
    const parts = pathname.split("/").filter(Boolean);
    if (parts.length < 2) return null;
    const [kindStr, itemIdStr] = parts;
    const itemId = parseInt(itemIdStr, 10);
    if (isNaN(itemId)) return null;
    let kind: TreeKind;
    switch (kindStr) {
      case "etymology":
        kind = TreeKind.Etymology;
        break;
      case "cognates":
        kind = TreeKind.Cognates;
        break;
      case "descendants":
        kind = TreeKind.Descendants;
        break;
      default:
        return null;
    }
    const params = new URLSearchParams(search);
    const distLangStr = params.get("distLang");
    const distLangId = distLangStr ? parseInt(distLangStr, 10) : 0;
    const descLangIds = params.getAll("descLang").map((s) => parseInt(s, 10));
    return { kind, itemId, distLangId, descLangIds };
  }

  equals(other: TreeRequest): boolean {
    return (
      this.lang.id === other.lang.id &&
      this.item.id === other.item.id &&
      this.descLangs.length === other.descLangs.length &&
      this.descLangs.every((lang, i) => lang.id === other.descLangs[i].id) &&
      this.kind === other.kind
    );
  }
}
