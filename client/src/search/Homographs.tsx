import { Item, linkHref, term } from "./types";
import { useGoTo } from "./navigate";
import { homographs, lastRequest, selectedLang, unresolved } from "../state";

import { For, Show } from "solid-js";
import { useRouter } from "@tanstack/solid-router";

// Wiktionary's number for the etymology, or ours where it has none.
const label = (item: Item) => item.ety ?? String(item.etyNum);

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
              No {selectedLang().code === u().link.lang ? selectedLang().name : `“${u().link.lang}”`} word
              “{u().link.term}” is known. Try searching for it.
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

