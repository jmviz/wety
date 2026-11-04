import "./tree.css";
import { Alternative, Forest, TreeNode, isUncertain } from "./model";
import NodeLabel from "./NodeLabel";
import { etyModeRep } from "./etyMode";
import { hoverEnter, hoverLeave, pin } from "./popover";

import { For, Show, createSignal, onMount } from "solid-js";

interface ChartTreeProps {
  forest: Forest;
}

// Etymology as a chart read like the outline, oldest at the top: the word at
// the bottom, its sources above it, theirs above them. Layout and connectors
// are pure CSS (flex rows plus border pseudo-elements); the derivation mode
// sits on the connector. A word's alternative etymologies ("from A or B") are
// branches of their own, each with its mode and "or"; an uncertain link is
// dashed.
export default function ChartTree(props: ChartTreeProps) {
  let rootEl!: HTMLDivElement;

  const nodeFromEvent = (e: Event): [TreeNode, HTMLElement] | null => {
    const label = (e.target as Element).closest<HTMLElement>(".label");
    if (!label || !rootEl.contains(label)) return null;
    const node = props.forest.byKey.get(Number(label.dataset.key));
    return node ? [node, label] : null;
  };

  // A chart wider than the screen would open at its left edge; centre the
  // searched word (the bottom of the chart) instead.
  onMount(() => {
    requestAnimationFrame(() => {
      const box = rootEl.querySelector<HTMLElement>(".chart-node.selected > .chart-box");
      if (!box || rootEl.scrollWidth <= rootEl.clientWidth) return;
      const view = rootEl.getBoundingClientRect();
      const r = box.getBoundingClientRect();
      rootEl.scrollLeft += r.left + r.width / 2 - (view.left + view.width / 2);
    });
  });

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
  const shown = n.primary ?? 0;
  const count = n.alternatives.length;
  // With none preferred every alternative is drawn; otherwise the others
  // wait behind a control.
  const [all, setAll] = createSignal(count > 1 && n.primary === null);
  const parentsIn = (i: number) => n.children.filter((c) => (c.alt ?? 0) === i);
  const above = n.children.length > 0 || n.alternatives.some((a) => a.kind === "unknown");
  return (
    <li
      class="chart-node"
      classList={{ selected: n.isSelected, uncertain: isUncertain(n) }}
      role="treeitem"
    >
      <div class="chart-box">
        <NodeLabel node={n} />
      </div>
      <Show when={above}>
        <div class="chart-mode">
          <Show when={n.reading?.disputed}>
            <span class="disputed-badge" title="independent sources disagree">
              disputed
            </span>{" "}
          </Show>
          <Show
            when={count > 1 && all()}
            fallback={
              <>
                <AltMode alt={n.alternatives[shown]} />
                <Show when={count > 1}>
                  <button class="chart-alts-toggle" onClick={() => setAll(true)}>
                    {count === 2 ? "or 1 more" : `or ${count - 1} more`}
                  </button>
                </Show>
              </>
            }
          >
            <button class="chart-alts-toggle" onClick={() => setAll(false)}>
              {n.primary === null ? `${count} alternatives, none preferred` : `${count} alternatives`}
            </button>
          </Show>
        </div>
        <Show when={count > 1 && all()} fallback={<Sources nodes={parentsIn(shown)} />}>
          <ul role="group" class="chart-alts">
            <For each={n.alternatives}>
              {(a, i) => (
                <li class="chart-alt" classList={{ preferred: i() === n.primary }}>
                  <div class="chart-mode">
                    <Show when={i() > 0}>
                      <span class="chart-or">or</span>{" "}
                    </Show>
                    <AltMode alt={a} />
                  </div>
                  <Sources nodes={parentsIn(i())} />
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Show>
    </li>
  );
}

function Sources(props: { nodes: TreeNode[] }) {
  return (
    <Show when={props.nodes.length > 0}>
      <ul role="group">
        <For each={props.nodes}>{(c) => <ChartNode node={c} />}</For>
      </ul>
    </Show>
  );
}

// Inheritance is the unmarked case; a rule's reading says so.
function AltMode(props: { alt: Alternative | undefined }) {
  return (
    <Show when={props.alt}>
      {(a) => (
        <Show
          when={a().kind !== "unknown"}
          fallback={<span class="mode-unknown">origin unknown</span>}
        >
          <Show when={a().mode && (a().mode !== "inherited" || a().inferred)}>
            <span
              classList={{ inferred: a().inferred, "from-source": a().source }}
              title={
                a().inferred
                  ? "inferred by wety, not stated by a source"
                  : a().source
                    ? "given by another source, not Wiktionary"
                    : undefined
              }
            >
              {etyModeRep(a().mode!)}
            </span>
          </Show>
        </Show>
      )}
    </Show>
  );
}
