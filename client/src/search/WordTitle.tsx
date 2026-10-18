import { Item, TreeKind, term } from "./types";
import { useGoTo } from "./navigate";
import { forest, homographs, lastRequest, treeSize } from "../state";

import { For, Show, createSignal, onCleanup, onMount } from "solid-js";

// Wiktionary's number for the etymology, or ours where it has none.
const label = (item: Item) => item.ety ?? String(item.etyNum);

// The word being shown, on one line: term, its etymology number when it has
// homographs (a switch between them), part of speech and first sense, then
// how big the tree is.
export default function WordTitle() {
  const goTo = useGoTo();
  const [open, setOpen] = createSignal(false);
  let switchEl: HTMLSpanElement | undefined;

  onMount(() => {
    const away = (e: PointerEvent) => {
      if (open() && switchEl && !switchEl.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", esc);
    onCleanup(() => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", esc);
    });
  });

  const words = (n: number) => `${n.toLocaleString()} ${n === 1 ? "word" : "words"}`;
  // The server counts the words below the root; the forest counts the root too.
  const total = () => treeSize()!.total + 1;

  return (
    <Show when={lastRequest()}>
      {(r) => (
        <div class="word-title">
          <h1 class="word">
            <span class="term">{term(r().item)}</span>
            <Show when={homographs().length > 1}>
              <span class="homograph-switch" ref={switchEl}>
                <button
                  type="button"
                  class="view-link"
                  aria-expanded={open()}
                  aria-label={`etymology ${label(r().item)} of ${homographs().length}`}
                  onClick={() => setOpen(!open())}
                >
                  <sup>{label(r().item)}</sup>
                  <span class="disclosure" aria-hidden="true" />
                </button>
                <Show when={open()}>
                  <ul class="menu homograph-menu" aria-label="etymologies of this word">
                    <For each={homographs()}>
                      {(h) => (
                        <li>
                          <button
                            type="button"
                            classList={{ current: h.id === r().item.id }}
                            aria-current={h.id === r().item.id ? "page" : undefined}
                            onClick={() => {
                              setOpen(false);
                              if (h.id !== r().item.id) goTo(r().kind, h);
                            }}
                          >
                            <span class="ety-label">{label(h)}</span>{" "}
                            <Show when={h.pos?.[0]}>
                              <i>{h.pos![0]}</i>{" "}
                            </Show>
                            {h.gloss?.[0]}
                          </button>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </span>
            </Show>
          </h1>
          <Show when={r().item.pos?.[0] || r().item.gloss?.[0]}>
            <p class="sense">
              <Show when={r().item.pos?.[0]}>
                <i>{r().item.pos![0]}</i>{" "}
              </Show>
              {r().item.gloss?.[0]}
            </p>
          </Show>
          <Show when={r().kind !== TreeKind.Etymology && forest()}>
            {(f) => (
              <p class="tree-stats">
                <Show
                  when={treeSize() && !treeSize()!.allLangs && total() > f().nodeCount}
                  fallback={words(f().nodeCount)}
                >
                  {f().nodeCount.toLocaleString()} of {words(total())}
                  {treeSize()!.total >= 100_000 ? "+" : ""}
                </Show>
                {" · "}
                {f().langs.length} {f().langs.length === 1 ? "language" : "languages"}
                <Show when={treeSize() && !treeSize()!.allLangs}>
                  {" · "}
                  <button
                    type="button"
                    class="link"
                    onClick={() => goTo(r().kind, r().item, { all: true }, r().root)}
                  >
                    show all
                  </button>
                </Show>
              </p>
            )}
          </Show>
        </div>
      )}
    </Show>
  );
}
