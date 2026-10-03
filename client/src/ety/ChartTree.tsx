import "./tree.css";
import { Forest, TreeNode } from "./model";
import NodeLabel from "./NodeLabel";
import { langColor } from "./langColor";
import { etyModeRep } from "./etyMode";
import { hoverEnter, hoverLeave, pin } from "./popover";

import { For, Show } from "solid-js";

interface ChartTreeProps {
  forest: Forest;
}

// Etymology as a top-down chart: the word at the top, its sources beneath it,
// theirs beneath them. Layout and connectors are pure CSS (flex rows plus
// border pseudo-elements); the derivation mode sits on the connector.
export default function ChartTree(props: ChartTreeProps) {
  let rootEl!: HTMLDivElement;

  const nodeFromEvent = (e: Event): [TreeNode, HTMLElement] | null => {
    const label = (e.target as Element).closest<HTMLElement>(".label");
    if (!label || !rootEl.contains(label)) return null;
    const node = props.forest.byKey.get(Number(label.dataset.key));
    return node ? [node, label] : null;
  };

  return (
    <div class="tree-view">
      <div
        ref={rootEl}
        class="chart-scroll"
        on:click={(e) => {
          const hit = nodeFromEvent(e);
          if (hit) pin(hit[0], hit[1]);
        }}
        on:pointerover={(e) => {
          const hit = nodeFromEvent(e);
          if (hit) hoverEnter(hit[0], hit[1]);
        }}
        on:pointerout={(e) => {
          const label = (e.target as Element).closest(".label");
          if (label && !label.contains(e.relatedTarget as Node)) hoverLeave();
        }}
      >
        <ul class="chart" role="tree">
          <For each={props.forest.roots}>{(n) => <ChartNode node={n} />}</For>
        </ul>
      </div>
    </div>
  );
}

function ChartNode(props: { node: TreeNode }) {
  const n = props.node;
  return (
    <li
      class="chart-node"
      classList={{ selected: n.isSelected }}
      style={{ "--lang-color": langColor(n.langDistance) }}
      role="treeitem"
    >
      <div class="chart-box">
        <NodeLabel node={n} />
      </div>
      <Show when={n.children.length > 0}>
        <div class="chart-mode">
          <Show when={n.etyMode}>{etyModeRep(n.etyMode!)}</Show>
        </div>
        <ul role="group">
          <For each={n.children}>{(c) => <ChartNode node={c} />}</For>
        </ul>
      </Show>
    </li>
  );
}
