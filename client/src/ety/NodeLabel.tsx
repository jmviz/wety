import { term } from "../search/types";
import { TreeNode, outlineSources } from "./model";
import { langColor } from "./langColor";
import { etyModeShort, etyPrep } from "./etyMode";
import { showGlosses } from "../state";
import { BASIS_MARK, BASIS_NAME, periodOf } from "./periods";

import { For, Show } from "solid-js";

interface NodeLabelProps {
  node: TreeNode;
  // Show the derivation mode inline (outline rows); the chart shows it on the
  // connector instead.
  inlineMode?: boolean;
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

  return (
    <>
      <button
        type="button"
        class="label"
        data-key={props.node.key}
        aria-haspopup="dialog"
      >
        <span class="lang" style={{ color: langColor(props.node.langDistance) }}>
          {item().lang.name}
        </span>{" "}
        <span class="term">{term(item())}</span>
        <Show when={item().romanization}>
          {" "}
          <span class="romanization">{item().romanization}</span>
        </Show>
      </button>
      <Show when={periodOf(item().lang.id)}>
        {(p) => (
          <span
            class="period"
            title={`${item().lang.name}: ${BASIS_NAME[p().basis]} (${p()
              .evidence.map((e) => e.source)
              .join("; ")})`}
          >
            {p().label}
            <span class={`basis basis-${p().basis}`}>{BASIS_MARK[p().basis]}</span>
          </span>
        )}
      </Show>
      <Show when={props.inlineMode && props.node.etyMode}>
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
                    <span class="lang" style={{ color: langColor(s.langDistance) }}>
                      {s.item.lang.name}
                    </span>{" "}
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
