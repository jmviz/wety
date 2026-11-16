import { CARD_WIDTH } from "./layout.ts";

// Intrinsic content measurements, never the widths assigned by this function.
// That distinction lets a lane shrink again when its widest content is closed.
export interface LaneContent {
  language: string;
  root: number;
  source?: number;
  words?: number;
}

export interface LaneWidth { width: number; source: number }

export function languageWidths(content: LaneContent[], available: number): Map<string, LaneWidth> {
  const groups = new Map<string, { root: number; source: number; words: number; tray: boolean }>();
  for (const c of content) {
    const g = groups.get(c.language) ?? { root: 0, source: 0, words: 0, tray: false };
    g.root = Math.max(g.root, c.root);
    g.source = Math.max(g.source, c.source ?? 0);
    g.words = Math.max(g.words, c.words ?? 0);
    g.tray ||= c.source !== undefined;
    groups.set(c.language, g);
  }
  return new Map([...groups].map(([language, g]) => {
    const limit = Math.max(240, Math.min(520, available));
    const source = Math.min(220, Math.max(60, g.source));
    const words = Math.min(360, Math.max(100, g.words));
    const width = Math.min(limit, Math.max(100, Math.min(CARD_WIDTH, g.root + 40), g.tray ? source + words + 80 : 0));
    return [language, { width: Math.ceil(width), source: Math.max(0, Math.floor(Math.min(source, width - 180))) }];
  }));
}
