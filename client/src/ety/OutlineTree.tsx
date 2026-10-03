import "./tree.css";
import { Forest, TreeNode, setAllOpen, openLang } from "./model";
import NodeLabel from "./NodeLabel";
import { langColor } from "./langColor";
import { hoverEnter, hoverLeave, pin } from "./popover";
import { showGlosses, setShowGlosses } from "../state";

import { For, Show, createEffect, createMemo, on } from "solid-js";

interface OutlineTreeProps {
  forest: Forest;
}

// Descendants / cognates as a semantic indented outline: nested lists,
// `<details>` on branch nodes only, connectors drawn with CSS borders.
export default function OutlineTree(props: OutlineTreeProps) {
  let rootEl!: HTMLDivElement;

  const nodeFromEvent = (e: Event): [TreeNode, HTMLElement] | null => {
    const label = (e.target as Element).closest<HTMLElement>(".label");
    if (!label || !rootEl.contains(label)) return null;
    const node = props.forest.byKey.get(Number(label.dataset.key));
    return node ? [node, label] : null;
  };

  // One set of listeners for the whole tree rather than one per row.
  const onClick = (e: MouseEvent) => {
    const hit = nodeFromEvent(e);
    if (!hit) return;
    e.preventDefault(); // don't toggle the enclosing <details>
    pin(hit[0], hit[1]);
  };
  const onPointerOver = (e: PointerEvent) => {
    const hit = nodeFromEvent(e);
    if (hit) hoverEnter(hit[0], hit[1]);
  };
  const onPointerOut = (e: PointerEvent) => {
    const label = (e.target as Element).closest(".label");
    if (!label) return;
    if (label.contains(e.relatedTarget as Node)) return;
    hoverLeave();
  };

  // Bring the searched item into view once per tree.
  createEffect(
    on(
      () => props.forest,
      (f) => {
        if (!f.selected) return;
        requestAnimationFrame(() => {
          rootEl
            .querySelector(`.label[data-key="${f.selected!.key}"]`)
            ?.scrollIntoView({ block: "center" });
        });
      }
    )
  );

  return (
    <div class="tree-view">
      <Toolbar forest={props.forest} />
      <div
        ref={rootEl}
        class="outline"
        on:click={onClick}
        on:pointerover={onPointerOver}
        on:pointerout={onPointerOut}
      >
        <ul class="tree" role="tree">
          <For each={props.forest.roots}>{(n) => <Node node={n} />}</For>
        </ul>
      </div>
    </div>
  );
}

function Node(props: { node: TreeNode }) {
  const n = props.node;
  const branch = n.children.length > 0;
  // Children are built the first time the branch opens and kept afterwards,
  // so the initial render costs only what is visible.
  const mounted = createMemo<boolean>((prev) => prev || n.open(), false);

  return (
    <li
      class="node"
      classList={{
        branch,
        leaf: !branch,
        "same-lang": n.sameLangAsParent,
        hoisted: n.hoisted,
        selected: n.isSelected,
        "on-path": n.onSelectedPath,
      }}
      style={{ "--lang-color": langColor(n.langDistance), "--rows": n.visibleRows() }}
      role="treeitem"
      aria-expanded={branch ? n.open() : undefined}
      aria-selected={n.isSelected ? true : undefined}
    >
      <Show when={branch} fallback={
        <div class="row">
          <span class="caret" aria-hidden="true" />
          <NodeLabel node={n} inlineMode />
        </div>
      }>
        <details open={n.open()} on:toggle={(e) => n.setOpen((e.currentTarget as HTMLDetailsElement).open)}>
          <summary class="row">
            <span class="caret" aria-hidden="true" />
            <NodeLabel node={n} inlineMode />
            <span class="badge">
              {n.descendantCount}
              <span class="badge-langs"> · {langSummary(n)}</span>
            </span>
          </summary>
          <Show when={mounted()}>
            <ul role="group">
              <For each={n.children}>{(c) => <Node node={c} />}</For>
            </ul>
          </Show>
        </details>
      </Show>
    </li>
  );
}

function langSummary(n: TreeNode): string {
  const top = n.langCounts.slice(0, 3).map((lc) => lc.lang.name);
  const more = n.langCounts.length - top.length;
  return top.join(", ") + (more > 0 ? ` +${more}` : "");
}

function Toolbar(props: { forest: Forest }) {
  let langSelect!: HTMLSelectElement;
  return (
    <div class="tree-toolbar" role="toolbar" aria-label="tree controls">
      <span class="tree-stats">
        {props.forest.nodeCount.toLocaleString()} words · {props.forest.langs.length} languages
      </span>
      <button type="button" onClick={() => setAllOpen(props.forest, true)}>
        Expand all
      </button>
      <button type="button" onClick={() => setAllOpen(props.forest, false)}>
        Collapse all
      </button>
      <label class="tree-lang-expand">
        Expand language
        <select
          ref={langSelect}
          onChange={(e) => {
            const id = Number(e.currentTarget.value);
            if (id) openLang(props.forest, id);
            e.currentTarget.value = "";
          }}
        >
          <option value="">…</option>
          <For each={props.forest.langs}>
            {(lc) => (
              <option value={lc.lang.id}>
                {lc.lang.name} ({lc.count})
              </option>
            )}
          </For>
        </select>
      </label>
      <label class="tree-toggle">
        <input
          type="checkbox"
          checked={showGlosses()}
          onChange={(e) => setShowGlosses(e.currentTarget.checked)}
        />
        glosses
      </label>
    </div>
  );
}
