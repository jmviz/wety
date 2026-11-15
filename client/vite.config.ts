import { readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import solid from "vite-plugin-solid";
import type { LayoutPeriod } from "./src/constellation/chronology";

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

// `virtual:common-words`: English words by how common they are, from SCOWL
// (the `wordlist-english` package: American, British, Canadian and Australian
// spellings, sizes 10 to 70, the last including obscure words). Each word
// is under the smallest size that lists it. The constellation view leaves
// defaults to sizes through 50 and lists the commonest first; the graph
// has no word frequencies.
function commonWords(): Plugin {
  const id = "virtual:common-words";
  const dir = resolve(dirname(fileURLToPath(import.meta.url)), "node_modules/wordlist-english");
  return {
    name: "wety-common-words",
    resolveId: (source) => (source === id ? "\0" + id : undefined),
    load(resolved) {
      if (resolved !== "\0" + id) return;
      const sizes = new Map<string, number>();
      for (const file of readdirSync(dir)) {
        const size = Number(/-words-(\d+)\.json$/.exec(file)?.[1]);
        if (!size) continue;
        for (const word of JSON.parse(readFileSync(resolve(dir, file), "utf8")) as string[]) {
          const w = word.toLowerCase();
          if (size < (sizes.get(w) ?? Infinity)) sizes.set(w, size);
        }
      }
      const levels: Record<string, string[]> = {};
      for (const [w, size] of sizes) (levels[size] ??= []).push(w);
      const joined = Object.fromEntries(Object.entries(levels).map(([size, words]) => [size, words.sort().join("\n")]));
      return `export const levels = ${JSON.stringify(joined)};\n`;
    },
  };
}

// Layout hints reuse the shipped dating registry without changing its claims.
function layoutPeriods(): Plugin {
  const id = "virtual:layout-periods";
  const path = resolve(dirname(fileURLToPath(import.meta.url)), "../data/products/dating/language_periods.json");
  return {
    name: "wety-layout-periods",
    resolveId: (source) => source === id ? "\0" + id : undefined,
    load(resolved) {
      if (resolved !== "\0" + id) return;
      this.addWatchFile(path);
      const { languages } = JSON.parse(readFileSync(path, "utf8")) as { languages: LayoutPeriod[] };
      const periods = languages.map(({ code, spoken, estimate }) => ({ code, spoken, estimate }));
      return `export const periods = ${JSON.stringify(periods)};`;
    },
  };
}

export default defineConfig({
  plugins: [solid(), langTree(), commonWords(), layoutPeriods()],
  server: {
    port: 8000,
  },
  css: {
    modules: {
      localsConvention: "camelCaseOnly",
    },
  },
});
