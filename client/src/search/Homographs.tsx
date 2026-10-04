import { Item, linkHref, term } from "./types";
import { useGoTo } from "./navigate";
import { homographs, lastRequest, unresolved } from "../state";

import { For, Show } from "solid-js";
import { useRouter } from "@tanstack/solid-router";

// Wiktionary's number for the etymology, or ours where it has none.
const label = (item: Item) => item.ety ?? String(item.etyNum);

function sense(item: Item): string {
  const gloss = item.gloss?.[0] ?? "";
  const short = gloss.length > 40 ? `${gloss.slice(0, 38).trimEnd()}…` : gloss;
  return [item.pos?.[0], short].filter(Boolean).join(" ");
}

// The word's other etymologies, one click away. Links name an etymology by
// Wiktionary's number, which shifts when editors insert one, so a shared link
// can land on a sibling of the word meant.
export function HomographBar() {
  const goTo = useGoTo();
  const current = () => lastRequest()?.item.id;
  return (
    <Show when={lastRequest() && homographs().length > 1}>
      <nav class="view-bar homographs" aria-label="etymologies of this word">
        <For each={homographs()}>
          {(h) => (
            <button
              type="button"
              class="view-link"
              classList={{ current: h.id === current() }}
              aria-current={h.id === current() ? "page" : undefined}
              title={h.gloss?.[0] ?? undefined}
              onClick={() => h.id !== current() && goTo(lastRequest()!.kind, h)}
            >
              <span class="ety-label">{label(h)}</span> {sense(h)}
            </button>
          )}
        </For>
      </nav>
    </Show>
  );
}

// A link that names no single word: offer the homographs, or say so.
export function Unresolved() {
  const router = useRouter();
  return (
    <Show when={unresolved()}>
      {(u) => (
        <Show
          when={u().reason === "ambiguous"}
          fallback={
            <p class="empty">
              Nothing called “{u().link.term}” in language “{u().link.lang}”. Try searching
              for it.
            </p>
          }
        >
          <div class="choose">
            <p>
              <span class="lang">{homographs()[0].lang.name}</span>{" "}
              <span class="term">{term(homographs()[0])}</span> has {homographs().length}{" "}
              etymologies. Which one?
            </p>
            <ul>
              <For each={homographs()}>
                {(h) => (
                  <li>
                    <a
                      href={linkHref({ ...u().link, ety: label(h) })}
                      onClick={(e) => {
                        e.preventDefault();
                        router.history.push(linkHref({ ...u().link, ety: label(h) }));
                      }}
                    >
                      <span class="ety-label">{label(h)}</span>{" "}
                      <i>{h.pos?.join(", ")}</i> {h.gloss?.[0]}
                    </a>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </Show>
      )}
    </Show>
  );
}

