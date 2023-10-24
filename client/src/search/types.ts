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

  url(): string {
    switch (this.kind) {
      case TreeKind.Etymology:
        return `${process.env.REACT_APP_API_BASE_URL}/etymology/${this.item.id}`;
      case TreeKind.Descendants:
        return `${process.env.REACT_APP_API_BASE_URL}/descendants/${
          this.item.id
        }?distLang=${this.item.lang.id}&${this.descLangs
          .map((lang) => `descLang=${lang.id}`)
          .join("&")}`;
    }
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
