import SearchBox from "./search/SearchBox";
import ViewBar from "./search/ViewBar";
import { TreeKind } from "./search/types";
import OutlineTree from "./ety/OutlineTree";
import ChartTree from "./ety/ChartTree";
import NodePopover from "./ety/NodePopover";
import { forest, lastRequest, loadFromPath } from "./state";

import { Show, createEffect, on } from "solid-js";
import { Link, useLocation } from "@tanstack/solid-router";

export default function App() {
  const location = useLocation();

  // `on` keeps the loader untracked: it reads other signals synchronously
  // (e.g. the descendant-language set) and later writes them, which would
  // otherwise re-run this effect and load the same path forever.
  createEffect(
    on(
      () => location().pathname + location().searchStr,
      (path) => {
        if (path === "/") return;
        loadFromPath(path);
      }
    )
  );

  const home = () => location().pathname === "/";

  return (
    <>
      <header class="masthead" classList={{ home: home() }}>
        <Link to="/" class="wordmark" aria-label="wety home">
          wety
        </Link>
        <Show when={home()}>
          <p class="tagline">(w)iktionary (ety)mologies</p>
        </Show>
        <SearchBox />
        <Show when={home()}>
          <p class="hint">
            Type a word, or a language then a word: <kbd>latin</kbd> <kbd>status</kbd>.
            Press <kbd>/</kbd> to search from anywhere.
          </p>
        </Show>
        <ViewBar />
      </header>
      <main>
        <Show when={forest()} keyed>
          {(f) =>
            f.kind === TreeKind.Etymology ? (
              <ChartTree forest={f} />
            ) : (
              <OutlineTree forest={f} />
            )
          }
        </Show>
        <Show when={lastRequest() && !forest()}>
          <p class="empty">Nothing to show for this word.</p>
        </Show>
      </main>
      <NodePopover />
    </>
  );
}
