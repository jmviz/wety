import { Item, linkHref, term } from "./types";
import { useGoTo } from "./navigate";
import { homographs, lastRequest, unresolved } from "../state";

import { For, Show } from "solid-js";
import { useRouter } from "@tanstack/solid-router";

// Wiktionary's number for the etymology, or ours where it has none.
const label = (item: Item) => item.ety ?? String(item.etyNum);

// Keep the full sense below a compact selector, instead of making every
// definition compete for space on the view bar.
export function HomographBar() {
  const goTo = useGoTo();
  const current = () => lastRequest()?.item;
  return (
    <Show when={lastRequest() && homographs().length > 1}>
      <div class="homographs">
        <label class="homograph-picker">
          Etymology
          <select aria-label="etymology of this word" value={String(current()?.id)} onChange={(e) => {
            const h = homographs().find((item) => item.id === Number(e.currentTarget.value));
            if (h && h.id !== current()?.id) goTo(lastRequest()!.kind, h);
          }}>
            <For each={homographs()}>{(h) => (
              <option value={String(h.id)}>{label(h)} · {h.pos?.[0]} · {h.gloss?.[0] || "No definition recorded"}</option>
            )}</For>
          </select>
        </label>
        <p class="current-sense"><i>{current()?.pos?.[0]}</i> {current()?.gloss?.[0]}</p>
      </div>
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

