import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import solid from "vite-plugin-solid";

// `virtual:lang-tree`: each language's parent language and the names of the
// languages that have daughters, from the Wiktionary export the graph was
// built from (`data/registry/inputs.json`). The constellation view groups
// words by branch with it; the API sends no family for a language.
function langTree(): Plugin {
  const id = "virtual:lang-tree";
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const inputs = resolve(repo, "data/registry/inputs.json");
  return {
    name: "wety-lang-tree",
    resolveId: (source) => (source === id ? "\0" + id : undefined),
    load(resolved) {
      if (resolved !== "\0" + id) return;
      const languages = resolve(repo, JSON.parse(readFileSync(inputs, "utf8")).wiktionary_languages);
      this.addWatchFile(inputs);
      this.addWatchFile(languages);
      const table = JSON.parse(readFileSync(languages, "utf8")) as Record<
        string,
        { ancestors?: string[]; canonicalName: string }
      >;
      const parents: Record<string, string> = {};
      const names: Record<string, string> = {};
      for (const [code, lang] of Object.entries(table)) {
        const ancestors = lang.ancestors ?? [];
        if (!ancestors.length) continue;
        parents[code] = ancestors[ancestors.length - 1];
        for (const a of ancestors) if (table[a]) names[a] = table[a].canonicalName;
      }
      return `export const parents = ${JSON.stringify(parents)};\nexport const names = ${JSON.stringify(names)};\n`;
    },
  };
}

export default defineConfig({
  plugins: [solid(), langTree()],
  server: {
    port: 8000,
  },
  css: {
    modules: {
      localsConvention: "camelCaseOnly",
    },
  },
});
