# wety-client

The frontend for [`wety.org`](https://www.wety.org/).

It's a [Vite](https://vite.dev/) app using [SolidJS](https://www.solidjs.com/) and [TanStack Router](https://tanstack.com/router); there is no UI library. Everything is set in [Brill](https://brill.com/page/resources_fontsdownloads) (free for non-commercial use), served from `public/fonts`.

## Search

There is one input (`src/search/SearchBox.tsx`), which reads like a dictionary citation: a small-caps language chip and a word. Typing offers words in the chip's language, with matching languages in a second group; typing an exact language name followed by a space swaps the chip, and Backspace on an empty word (or clicking the chip) edits it. `/` focuses it from anywhere. The chip, the reader's languages and the current view are remembered in `localStorage` (`src/prefs.ts`).

On a word's page the search sits beside the wordmark on one line; the dotted frame is the home page's. Below it, the word's title line (`src/search/WordTitle.tsx`) gives the term, a switch between its etymologies when it has several, its first sense and the tree's size. Then one row of controls (`src/search/ViewBar.tsx`): the three views of the word (etymology, descendants, cognates) on the left; on the right, the languages a tree is drawn in, the connections it follows (`src/settings/ConnectionsPanel.tsx`) and how rows are shown (`src/settings/DisplayPanel.tsx`), each opening a panel. Trees are drawn in the reader's languages (remembered, and always including the word's own), with "show all" (`all=1` in the URL) beside the count; naming languages in the panel puts them in the URL (`lang=`). Words formed from a word of the same language (limelight under lime) are left out unless the panel asks for them (`within=1`). Cognates add the word's trail below the row (`src/ety/Trail.tsx`): the word back to its oldest form that is the same word, then that form's sources, then "ultimately from" the forms at the ends of its etymology (roots, with affixes and the rest one click away). The tree is the descendants of the bold one: that oldest form, or the root it is built on; any other step or source starts it there instead (`root=` in the URL).

## Trees

Both tree kinds are plain HTML; there is no SVG or layout library.

- **Descendants / cognates** (`src/ety/OutlineTree.tsx`) are an indented outline: nested `<ul>`/`<li>`, a `<details>` on every node that has children (leaves are plain rows), and connectors drawn with CSS borders. Subtrees use `content-visibility: auto` with a size hint from the number of visible rows, so very large trees stay cheap to scroll. A branch's children are built the first time it is opened.
- **Etymology** (`src/ety/ChartTree.tsx`) is a chart read like the outline, oldest at the top and the word at the bottom, laid out entirely with flexbox and border pseudo-elements; the derivation mode sits on the connector. A word's alternative etymologies ("from A or B") are branches of their own, each with its mode and "or": all are drawn when none is preferred, otherwise the preferred one with an "or N more" control. A possible or doubtful link is dashed and its word marked "?"; a mode a wety rule inferred (a form-of sense, a root category) is underlined; "origin unknown" stands where the source says so.
- `src/ety/model.ts` turns the API's JSON into the display model: stable keys, subtree counts and language summaries, display order (the searched item's ancestry first, then closest languages, then largest subtrees), the default collapse policy (path to the searched item plus a visible-row budget, spent on branches that pass the same word on before new formations), and the client-side filters (connection types, hidden languages) from the panels. Filters never re-fetch.
- Word details open in a popover (`src/ety/NodePopover.tsx`) on hover or click; on touch devices it is a bottom sheet. It uses the Popover API and CSS anchor positioning, with JS placement as a fallback. Its Origin list gives each of the word's etymologies with who asserts it (Wiktionary's editors, on which page and section, or the wety rule that inferred it) and the works it follows (footnotes, References), from `/origin/:id`.

## Links and API

URLs name a word, not a position in the server's data: `/<view>/<language code>/<term>`, plus `?ety=N` when Wiktionary numbers the word's etymologies and `lang=<code>` for each descendant language (spaces in terms are written as underscores, as on Wiktionary). `src/search/types.ts` builds and parses them; `src/state.ts` loads one by asking the server's `/resolve` for the word and its homographs, then fetching the tree by the word's numeric ID. Numeric IDs only mean something within one graph build, so the client sends the build's version (`graph=`) along with them; if the server answers 409, it has been redeployed and the page reloads, which resolves the URL afresh. When a word has several etymologies, a line under the view bar switches between them (`src/search/Homographs.tsx`); a link that names several without a number shows a chooser.

The client fetches an item lazily (`/item/:id`) when its glosses are absent, a word's origin (`/origin/:id`) when its details open, and a language (`/lang/:code`) when a link names one that doesn't occur in the tree.

## Local development

You'll need the `wety` API server running first — see the root README for that.

```bash
npm install
npm run dev
```

The dev server starts at `http://127.0.0.1:8000`. Changes are hot-reloaded.
