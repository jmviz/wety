import "./tree.css";
import { Forest, TreeNode, openPathTo } from "./model";
import NodeLabel from "./NodeLabel";
import { hoverEnter, hoverLeave, pin, popover } from "./popover";
import { term } from "../search/types";

import { For, Show, createEffect, createMemo, createSignal, on, onCleanup, onMount } from "solid-js";

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

  // The page opens at the top, with the word's path already open. While the
  // searched word is off screen, a button brings it back.
  const [away, setAway] = createSignal<"above" | "below" | null>(null);
  const selectedLabel = () =>
    props.forest.selected &&
    rootEl.querySelector<HTMLElement>(`.label[data-key="${props.forest.selected.key}"]`);
  const checkAway = () => {
    const label = selectedLabel();
    const r = label?.getBoundingClientRect();
    // A label inside a closed branch has no box: treat it as below.
    if (!r || (r.width === 0 && r.height === 0)) return setAway(label ? "below" : null);
    setAway(r.bottom <= 0 ? "above" : r.top >= window.innerHeight ? "below" : null);
  };
  onMount(() => {
    window.addEventListener("scroll", checkAway, { passive: true });
    window.addEventListener("resize", checkAway);
    onCleanup(() => {
      window.removeEventListener("scroll", checkAway);
      window.removeEventListener("resize", checkAway);
    });
  });
  createEffect(on(() => props.forest, () => queueMicrotask(checkAway)));
  // Keyboard: the tree is one Tab stop (the searched word, else the first
  // row); arrows move between visible rows, right and left open and close.
  const visibleLabels = () =>
    [...rootEl.querySelectorAll<HTMLElement>(".label")].filter((l) => l.getClientRects().length > 0);
  const moveTo = (from: HTMLElement | null, to: HTMLElement | undefined) => {
    if (!to) return;
    from?.setAttribute("tabindex", "-1");
    to.setAttribute("tabindex", "0");
    to.focus();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    const label = (e.target as Element).closest<HTMLElement>(".label");
    if (!label || !rootEl.contains(label)) return;
    const node = props.forest.byKey.get(Number(label.dataset.key));
    const labels = visibleLabels();
    const i = labels.indexOf(label);
    const byKey = (n: TreeNode | null | undefined) =>
      n ? rootEl.querySelector<HTMLElement>(`.label[data-key="${n.key}"]`) ?? undefined : undefined;
    const keys: Record<string, () => void> = {
      ArrowDown: () => moveTo(label, labels[i + 1]),
      ArrowUp: () => moveTo(label, labels[i - 1]),
      Home: () => moveTo(label, labels[0]),
      End: () => moveTo(label, labels[labels.length - 1]),
      ArrowRight: () => {
        if (!node?.children.length) return;
        if (!node.open()) node.setOpen(true);
        else queueMicrotask(() => moveTo(label, byKey(node.children[0])));
      },
      ArrowLeft: () => {
        if (node?.children.length && node.open()) node.setOpen(false);
        else moveTo(label, byKey(node?.parent));
      },
    };
    const act = keys[e.key];
    if (!act) return;
    e.preventDefault();
    act();
  };
  createEffect(
    on(
      () => props.forest,
      () =>
        queueMicrotask(() => {
          const first = selectedLabel() ?? rootEl.querySelector<HTMLElement>(".label");
          first?.setAttribute("tabindex", "0");
        })
    )
  );

  // While a word's details show, mark its rows back up to the root. Marked
  // in place, so a hover re-renders nothing.
  let marked: HTMLElement[] = [];
  createEffect(() => {
    const state = popover();
    for (const li of marked) li.classList.remove("lineage");
    marked = [];
    if (!state || props.forest.byKey.get(state.node.key) !== state.node) return;
    for (let n: TreeNode | null = state.node; n; n = n.parent) {
      const li = rootEl.querySelector(`.label[data-key="${n.key}"]`)?.closest<HTMLElement>("li.node");
      if (li) marked.push(li);
    }
    for (const li of marked) li.classList.add("lineage");
  });
  onCleanup(() => marked.forEach((li) => li.classList.remove("lineage")));

  const jump = () => {
    openPathTo(props.forest.selected!);
    // Branches mount their children when opened; wait for the label.
    queueMicrotask(() => {
      const label = selectedLabel();
      label?.scrollIntoView({ block: "center" });
      label?.focus({ preventScroll: true });
      checkAway();
    });
  };

  return (
    <div class="tree-view">
      <div
        ref={rootEl}
        class="outline"
        on:click={onClick}
        on:keydown={onKeyDown}
        on:pointerover={onPointerOver}
        on:pointerout={onPointerOut}
      >
        <ul class="tree" role="tree">
          <For each={props.forest.roots}>{(n) => <Node node={n} />}</For>
        </ul>
      </div>
      <Show when={away()}>
        <button type="button" class="jump-to-word" onClick={jump}>
          {away() === "above" ? "↑" : "↓"} {term(props.forest.selected!.item)}
        </button>
      </Show>
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
      style={{ "--rows": n.visibleRows() }}
      role="treeitem"
      aria-expanded={branch ? n.open() : undefined}
      aria-selected={n.isSelected ? true : undefined}
    >
      <Show when={branch} fallback={
        <div class="row">
          <span class="caret" aria-hidden="true" />
          <NodeLabel node={n} inlineMode roving />
        </div>
      }>
        <details open={n.open()} on:toggle={(e) => n.setOpen((e.currentTarget as HTMLDetailsElement).open)}>
          <summary class="row" tabIndex={-1}>
            <span class="caret" aria-hidden="true" />
            <NodeLabel node={n} inlineMode roving />
            <span class="badge" title={langSummary(n)}>
              +{n.descendantCount.toLocaleString()}
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
