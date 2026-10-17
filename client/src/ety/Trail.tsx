import { Item, TreeKind, term } from "../search/types";
import { useGoTo } from "../search/navigate";
import { cognates, lastRequest, selectedDescLangs } from "../state";
import { etyModeShort } from "./etyMode";

import { For, Show, createSignal } from "solid-js";

// Longer trails show the word, an expandable gap and the last two steps;
// the tree below draws the whole path anyway.
const SHORT_TRAIL = 4;

// The cognates view's line of descent: the word back to its oldest form that
// is the same word, then that form's sources. The tree below is the
// descendants of the bold one; any other starts it there instead.
export default function Trail() {
  const goTo = useGoTo();
  const info = () => cognates();
  const top = () => info()!.trail[info()!.trail.length - 1].item;
  // Expanded for one word only; the next word starts short again.
  const [expandedFor, setExpandedFor] = createSignal<number | null>(null);
  // How many steps after the word hide behind the gap.
  const hidden = () => {
    const n = info()!.trail.length;
    return expandedFor() === lastRequest()?.item.id || n <= SHORT_TRAIL ? 0 : n - 3;
  };
  const shown = (i: number) => i === 0 || i > hidden();
  const choose = (item: Item) => {
    const r = lastRequest()!;
    goTo(TreeKind.Cognates, r.item, selectedDescLangs(), item.id === top().id ? null : item);
  };

  // A language name only where it changes, so a run within one language
  // reads as a run.
  const Word = (props: { item: Item; prev?: Item }) => (
    <button
      type="button"
      class="trail-word"
      classList={{ current: props.item.id === info()!.root }}
      aria-pressed={props.item.id === info()!.root}
      onClick={() => props.item.id !== info()!.root && choose(props.item)}
    >
      <Show when={props.prev?.lang.id !== props.item.lang.id}>
        <span class="lang">{props.item.lang.name}</span>{" "}
      </Show>
      <span class="term">{term(props.item)}</span>
    </button>
  );

  // How each step came from the next one ("inherited", "borrowed"…).
  const stepMode = (i: number) => {
    const mode = info()!.trail[i].etyMode;
    return mode ? etyModeShort(mode) : undefined;
  };

  // Each arrow stays with the word after it when the line wraps.
  return (
    <Show when={info() && (info()!.trail.length > 1 || info()!.sources.length > 0)}>
      <p class="trail" role="group" aria-label="start the tree from">
        <For each={info()!.trail}>
          {(step, i) => (
            <Show when={shown(i())}>
              <Show when={hidden() > 0 && i() === hidden() + 1}>
                {" "}
                <span class="trail-link">
                  <span class="trail-step" aria-hidden="true">
                    {"← "}
                  </span>
                  <button
                    type="button"
                    class="trail-word trail-gap"
                    aria-label={`show ${hidden()} more steps`}
                    onClick={() => setExpandedFor(lastRequest()!.item.id)}
                  >
                    …
                  </button>
                </span>
              </Show>
              <Show when={i() > 0}>{" "}</Show>
              <span class="trail-link">
                <Show when={i() > 0}>
                  <span class="trail-step" title={stepMode(i() - 1)} aria-hidden="true">
                    {"← "}
                  </span>
                </Show>
                <Word item={step.item} prev={shown(i() - 1) ? info()!.trail[i() - 1]?.item : undefined} />
              </span>
            </Show>
          )}
        </For>
        <Show when={info()!.sources.length}>
          {" "}
          <span class="trail-link">
            <span class="trail-from">· from </span>
            <For each={info()!.sources}>
              {(source, i) => (
                <>
                  <Show when={i() > 0}>{" + "}</Show>
                  <Word item={source} prev={top()} />
                </>
              )}
            </For>
          </span>
        </Show>
      </p>
    </Show>
  );
}
