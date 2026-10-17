# wety-client

The frontend for [`wety.org`](https://www.wety.org/).

It's a [Vite](https://vite.dev/) app using [SolidJS](https://www.solidjs.com/) and [TanStack Router](https://tanstack.com/router); there is no UI library. Everything is set in [Brill](https://brill.com/page/resources_fontsdownloads) (free for non-commercial use), served from `public/fonts`.

## Search

There is one input (`src/search/SearchBox.tsx`), which reads like a dictionary citation: a small-caps language chip and a word. Typing offers words in the chip's language, with matching languages in a second group; typing an exact language name followed by a space swaps the chip, and Backspace on an empty word (or clicking the chip) edits it. `/` focuses it from anywhere. The chip, the descendant-language set and the current view are remembered in `localStorage` (`src/prefs.ts`).

The view controls (`src/search/ViewBar.tsx`) select etymology, descendants or cognates,
target languages, dates and filters. Target languages include the word's own language;
connecting languages stay visible along the paths. Suggestions use unique words
reachable from the current word or cognate origins, ranked by count, with the browser's
language first when it actually occurs. Counts precede target-language and display
filtering; there is no fixed list of favoured languages.

Cognates default to **Base lineage**, following the processor's recorded single-base
edges in both directions. This excludes relationships through affixes alone but still
includes loans and word formations. It is not a claim of strict inherited cognacy or
semantic/syntactic headedness. A compound with no single recorded base offers
**All components**, which preserves the broader view through every ultimate ancestor.
The server follows the stored edges directly: older cached head summaries can stop at
an ambiguous intermediate compound. No graph rebuild is needed.

Filters offer presets plus collapsed groups containing only connection types present
in the fetched tree. Active exclusions remain visible with a reset control. Languages
can be searched within a bounded list. These filters never re-fetch.

## Trees

Both tree kinds are plain HTML; there is no SVG or layout library.

- **Descendants / cognates** (`src/ety/OutlineTree.tsx`) are an indented outline: nested `<ul>`/`<li>`, a `<details>` on every node that has children (leaves are plain rows), and connectors drawn with CSS borders. Subtrees use `content-visibility: auto` with a size hint from the number of visible rows, so very large trees stay cheap to scroll. A branch's children are built the first time it is opened.
- **Etymology** (`src/ety/ChartTree.tsx`) is a top-down chart laid out entirely with flexbox and border pseudo-elements; the derivation mode sits on the connector.
- `src/ety/model.ts` turns the API's JSON into the display model: stable keys, subtree counts and language summaries, display order (the searched item's ancestry first, then closest languages, then largest subtrees), the default collapse policy (path to the searched item plus a visible-row budget), and the client-side filters (connection types, hidden languages) from the settings drawer. Filters never re-fetch.
- Hover gives a brief word preview; clicking opens full details (`src/ety/NodePopover.tsx`), with date evidence, varieties, alternative estimates and full citations behind disclosures. On touch devices it is a bottom sheet. It uses the Popover API and CSS anchor positioning, with JS placement as a fallback.
- Outline controls stay visible while scrolling. “Jump to searched word” reopens its path and focuses the word, including after “Collapse all.” `src/ety/TreeGuide.tsx` explains direction, relationships, connectors and date labels.

## Links and API

URLs name a word, not a position in the server's data: `/<view>/<language code>/<term>`, plus `?ety=N` when Wiktionary numbers the word's etymologies and `lang=<code>` for each descendant language (spaces in terms are written as underscores, as on Wiktionary). `src/search/types.ts` builds and parses them; `src/state.ts` loads one by asking the server's `/resolve` for the word and its homographs, then fetching the tree by the word's numeric ID. Numeric IDs only mean something within one graph build, so the client sends the build's version (`graph=`) along with them; if the server answers 409, it has been redeployed and the page reloads, which resolves the URL afresh. When a word has several etymologies, a line under the view bar switches between them (`src/search/Homographs.tsx`); a link that names several without a number shows a chooser.

Homographs use a compact selector with the selected sense below it; search suggestions
show numbered etymologies and definitions on separate lines.

Cognate links use `scope=all` for All components; omission means Base lineage in the
client. The API keeps its older default of all components for compatibility, so the
client always sends `/cognates/:id?scope=head|all` explicitly. Scope is part of the
server's tree-cache key. `/explore/:id?cognates=true|false&scope=head|all&graph=…`
provides the recorded base and unique reachable-word counts by language, independent
of selected targets. It shares the bounded response cache and graph-version checks.

The client fetches an item lazily (`/item/:id`) when its glosses are absent, and a language (`/lang/:code`) when a link names one that doesn't occur in the tree.

## Local development

You'll need the `wety` API server running first — see the root README for that.

```bash
npm install
npm run dev
```

The dev server starts at `http://127.0.0.1:8000`. Changes are hot-reloaded.
