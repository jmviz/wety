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
