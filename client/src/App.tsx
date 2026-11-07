import SearchBox from "./search/SearchBox";
import ViewBar from "./search/ViewBar";
import WordTitle from "./search/WordTitle";
import { Unresolved } from "./search/Homographs";
import { TreeKind, term } from "./search/types";
import OutlineTree from "./ety/OutlineTree";
import ChartTree from "./ety/ChartTree";
import NodePopover from "./ety/NodePopover";
import About, { GITHUB } from "./About";
import { clearView, forest, lastRequest, loadFromPath, loadError, loading } from "./state";

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
        if (path === "/" || path === "/about") return clearView();
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
  const about = () => location().pathname === "/about";

  // The tab names the word, so history and bookmarks tell words apart.
  createEffect(() => {
    const r = lastRequest();
    document.title = r ? `${term(r.item)} · ${r.item.lang.name} · wety` : about() ? "about · wety" : "wety";
  });

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
          <p class="hint">Type a language name first to search in it.</p>
        </Show>
        <Show when={!home() && !about()}>
          <nav class="masthead-links" aria-label="site">
            <Link to="/about">about</Link>
          </nav>
        </Show>
      </header>
      <Show when={about()}>
        <About />
      </Show>
      <Show when={!home() && !about()}>
        <section class="view" aria-label="word">
          <WordTitle />
          <ViewBar />
        </section>
      </Show>
      <main aria-busy={loading()}>
        <Show when={loading()}>
          <p class="empty" role="status">Loading word…</p>
        </Show>
        <Show when={loadError()}>
          <p class="empty" role="alert">
            {loadError()}{" "}
            <button type="button" class="link" onClick={() => window.location.reload()}>
              Retry
            </button>
          </p>
        </Show>
        <Show when={forest()} keyed>
          {(f) =>
            f.roots.length === 0 ? (
              <EmptyTree />
            ) : f.kind === TreeKind.Etymology ? (
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
      <Show when={home()}>
        <footer class="colophon">
          <Link to="/about">about</Link>
          <a href={GITHUB} target="_blank" rel="noopener noreferrer">
            GitHub
          </a>
        </footer>
      </Show>
      <NodePopover />
    </>
  );
}

// A tree with nothing to draw: filters that hide every word.
function EmptyTree() {
  return <p class="empty">The filters hide every word in this tree.</p>;
}
