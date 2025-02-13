export interface Language {
  name: string;
}

export interface TreeItem {
  lang: Language;
  term: string;
  romanization?: string;
}

export interface TreeNodeData {
  item: TreeItem;
  langDistance: number | null;
  children?: TreeNodeData[];
}

export interface ResponseData extends Array<TreeNodeData> {}

export interface ErrorResponse {
  error: string;
}
