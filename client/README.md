# wety-client

The frontend for [`wety.org`](https://www.wety.org/).

It's a [Vite](https://vite.dev/) app using [SolidJS](https://www.solidjs.com/) and [TanStack Router](https://tanstack.com/router); there is no UI library. Everything is set in [Brill](https://brill.com/page/resources_fontsdownloads) (free for non-commercial use), served from `public/fonts`.

## Search

There is one input (`src/search/SearchBox.tsx`), which reads like a dictionary citation: a small-caps language chip and a word. Typing offers words in the chip's language, with matching languages in a second group; typing an exact language name followed by a space swaps the chip, and Backspace on an empty word (or clicking the chip) edits it. `/` focuses it from anywhere. The chip, the descendant-language set and the current view are remembered in `localStorage` (`src/prefs.ts`).

Everything else is a property of the view rather than the search, and lives on one line under the box (`src/search/ViewBar.tsx`): the three views of the word (etymology, descendants, cognates), the descendant-language set (a chip editor; the word's own language is always included), and the client-side filters (`src/settings/FiltersPanel.tsx`).

## Trees

Both tree kinds are plain HTML; there is no SVG or layout library.

- **Descendants / cognates** (`src/ety/OutlineTree.tsx`) are an indented outline: nested `<ul>`/`<li>`, a `<details>` on every node that has children (leaves are plain rows), and connectors drawn with CSS borders. Subtrees use `content-visibility: auto` with a size hint from the number of visible rows, so very large trees stay cheap to scroll. A branch's children are built the first time it is opened.
- **Etymology** (`src/ety/ChartTree.tsx`) is a top-down chart laid out entirely with flexbox and border pseudo-elements; the derivation mode sits on the connector.
- `src/ety/model.ts` turns the API's JSON into the display model: stable keys, subtree counts and language summaries, display order (the searched item's ancestry first, then closest languages, then largest subtrees), the default collapse policy (path to the searched item plus a visible-row budget), and the client-side filters (connection types, hidden languages) from the settings drawer. Filters never re-fetch.
- Word details open in a popover (`src/ety/NodePopover.tsx`) on hover or click; on touch devices it is a bottom sheet. It uses the Popover API and CSS anchor positioning, with JS placement as a fallback.

## API

The client reads the tree endpoints of the `wety` server unchanged. Newer servers also send `descendantCount` per node, accept `lean=true` on the tree endpoints to omit glosses, and serve `/item/:id` and `/lang/:id`; the client fetches an item lazily when its glosses are absent, and a language name when a link names one that doesn't occur in the tree.

## Local development

You'll need the `wety` API server running first — see the root README for that.

```bash
npm install
npm run dev
```

The dev server starts at `http://127.0.0.1:8000`. Changes are hot-reloaded.
