/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

// Built by `vite.config.ts` from the Wiktionary language export.
declare module "virtual:lang-tree" {
  // A language's code to its parent language's code.
  export const parents: Record<string, string>;
  // Names of the languages that have daughters.
  export const names: Record<string, string>;
}

// Built by `vite.config.ts` from the SCOWL word lists (`wordlist-english`).
declare module "virtual:common-words" {
  // A SCOWL size (10, the commonest words, to 70) to the English words first
  // listed at that size, lower-case, one to a line.
  export const levels: Record<string, string>;
}
