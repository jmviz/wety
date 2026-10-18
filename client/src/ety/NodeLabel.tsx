import { term } from "../search/types";
import { TreeNode, outlineSources } from "./model";
import { etyModeShort, etyPrep } from "./etyMode";
import { showGlosses } from "../state";
import { BASIS_NAME, periodOf } from "./periods";

import { For, Show } from "solid-js";

interface NodeLabelProps {
  node: TreeNode;
  // Show the derivation mode inline (outline rows); the chart shows it on the
  // connector instead.
  inlineMode?: boolean;
  // Outline rows take one Tab stop between them; arrows move within.
  roving?: boolean;
}

// The clickable part of a row: language, term, romanization. Everything else
// on the row (mode mark, gloss, badge) is outside the button so it doesn't
// bloat the hit target.
export default function NodeLabel(props: NodeLabelProps) {
  const item = () => props.node.item;
  // Parents to name inline: the ones not already visible as the row's tree
  // parent (all of them for a root, whose parent isn't in the tree).
  const inlineSources = () =>
    props.node.parent
      ? props.node.otherParents
          .slice()
          .sort((a, b) => a.etyOrder - b.etyOrder)
          .map((p) => ({ item: p.item, langDistance: p.langDistance }))
      : outlineSources(props.node);
  const gloss = () => item().gloss?.[0];
  // Dates belong to languages: show them where the language changes.
  const datedHere = () => props.node.parent?.item.lang.id !== item().lang.id;
  // Inheritance is the unmarked case: say nothing unless there is a source
  // to name.
  const showMode = () =>
    props.inlineMode &&
    props.node.etyMode &&
    !(props.node.etyMode === "inherited" && props.node.parent && inlineSources().length === 0);

  return (
    <>
      <button
        type="button"
        class="label"
        data-key={props.node.key}
        tabIndex={props.roving ? -1 : undefined}
        aria-haspopup="dialog"
      >
        <span class="lang">{item().lang.name}</span>{" "}
        <span class="term">{term(item())}</span>
        <Show when={item().romanization}>
          {" "}
          <span class="romanization">{item().romanization}</span>
        </Show>
      </button>
      <Show when={datedHere() && periodOf(item().lang.id)}>
        {(p) => (
          <span
            class="period"
            title={`${item().lang.name}: ${BASIS_NAME[p().basis]} (${p()
              .evidence.map((e) => e.source)
              .join("; ")})`}
          >
            {p().label}
            <Show when={p().revival?.length}>, being revived</Show>
          </span>
        )}
      </Show>
      <Show when={showMode()}>
        <span class="mode">
          <span class="mode-name">{etyModeShort(props.node.etyMode!)}</span>
          <For each={inlineSources()}>
            {(s, i) => (
              <>
                {i() === 0 && !props.node.parent
                  ? etyPrep(props.node.etyMode!)
                  : " + "}
                <span class="mode-src">
                  <Show when={s.item.lang.id !== item().lang.id}>
                    <span class="lang">{s.item.lang.name}</span>{" "}
                  </Show>
                  {term(s.item)}
                </span>
              </>
            )}
          </For>
        </span>
      </Show>
      <Show when={showGlosses() && gloss()}>
        <span class="gloss">{gloss()}</span>
      </Show>
    </>
  );
}
