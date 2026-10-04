import SearchBox from "./search/SearchBox";
import ViewBar from "./search/ViewBar";
import { HomographBar, Unresolved } from "./search/Homographs";
import { TreeKind } from "./search/types";
import OutlineTree from "./ety/OutlineTree";
import ChartTree from "./ety/ChartTree";
import NodePopover from "./ety/NodePopover";
import { forest, lastRequest, loadFromPath } from "./state";

import { Show, createEffect, on } from "solid-js";
import { Link, useLocation, useRouter } from "@tanstack/solid-router";

export default function App() {
  const location = useLocation();
  const router = useRouter();

  // `on` keeps the loader untracked: it reads other signals synchronously
  // (e.g. the descendant-language set) and later writes them, which would
  // otherwise re-run this effect and load the same path forever.
  createEffect(
    on(
      () => location().pathname + location().searchStr,
      (path) => {
        if (path === "/") return;
        // Adopt the word's canonical URL (current language code, displayed
        // term) without adding a history entry.
        loadFromPath(path).then((canonical) => {
          if (canonical && location().pathname + location().searchStr === path)
            router.history.replace(canonical);
        });
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
        <HomographBar />
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
        <Unresolved />
        <Show when={lastRequest() && !forest()}>
          <p class="empty">Nothing to show for this word.</p>
        </Show>
      </main>
      <NodePopover />
    </>
  );
}
