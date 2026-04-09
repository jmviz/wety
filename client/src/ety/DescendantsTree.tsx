import "./Tree.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  TreeKind,
} from "../search/types";
import { xMinClusterLayout } from "./treeCluster";
import TreeTooltip from "./TreeTooltip";
import { PositionKind, hideTooltip, TooltipRefs } from "./tooltip";
import {
  configureSVG,
  renderTreeLinks,
  renderTreeNodes,
  addSVGTextBackgrounds,
  setTooltipListeners,
} from "./tree";
import { lastRequest } from "../state";

import { select } from "d3-selection";
import { curveStepBefore } from "d3-shape";
import { hierarchy, HierarchyPointNode } from "d3-hierarchy";
import { createSignal, createEffect, createMemo, onCleanup, For, Setter } from "solid-js";

interface DescendantsTreeProps {
  tree: Etymology | InterLangDescendants[] | null;
}

export default function DescendantsTree(props: DescendantsTreeProps) {
  const [showTooltip, setShowTooltip] = createSignal(false);
  const [tooltipTreeNode, setTooltipTreeNode] =
    createSignal<HierarchyPointNode<InterLangDescendants> | null>(null);
  const [tooltipSVGElement, setTooltipSVGElement] =
    createSignal<SVGElement | null>(null);
  const [tooltipPositionKind, setTooltipPositionKind] =
    createSignal<PositionKind>(PositionKind.Hover);

  const tooltipRefs: TooltipRefs = {
    el: undefined,
    showTimeout: null,
    hideTimeout: null,
  };

  const svgEls = createMemo(() => {
    const t = props.tree;
    if (!Array.isArray(t)) return [];
    return t.map(() => ({ current: null as SVGSVGElement | null }));
  });

  createEffect(() => {
    const t = props.tree;
    const request = lastRequest();
    const refs = svgEls();
    if (!Array.isArray(t) || !request) return;

    const cleanupFns: (() => void)[] = [];

    for (let index = 0; index < t.length; index++) {
      const svg = refs[index]?.current;
      if (!svg || !t[index]) return;

      descendantsTreeSVG(
        svg,
        t[index] as InterLangDescendants,
        request.item,
        setShowTooltip,
        setTooltipTreeNode,
        setTooltipSVGElement,
        setTooltipPositionKind,
        tooltipRefs
      );

      cleanupFns.push(() => select(svg).selectAll("*").remove());
    }

    onCleanup(() => {
      cleanupFns.forEach((fn) => fn());
      hideTooltip(tooltipRefs, setShowTooltip);
      setShowTooltip(false);
      setTooltipTreeNode(null);
      setTooltipSVGElement(null);
      setTooltipPositionKind(PositionKind.Hover);
    });
  });

  return (
    <div class="tree-container">
      <For each={svgEls()}>
        {(ref, index) => (
          <svg
            class="tree"
            ref={(el) => {
              ref.current = el;
            }}
          />
        )}
      </For>
      <TreeTooltip
        treeKind={TreeKind.Descendants}
        showTooltip={showTooltip}
        setShowTooltip={setShowTooltip}
        treeNode={tooltipTreeNode}
        svgElement={tooltipSVGElement}
        positionKind={tooltipPositionKind}
        tooltipRefs={tooltipRefs}
      />
    </div>
  );
}

function descendantsTreeSVG(
  svgElement: SVGSVGElement,
  tree: InterLangDescendants,
  treeRootItem: Item,
  setShowTooltip: Setter<boolean>,
  setTooltipTreeNode: Setter<HierarchyPointNode<InterLangDescendants> | null>,
  setTooltipSVGElement: Setter<SVGElement | null>,
  setTooltipPositionKind: Setter<PositionKind>,
  tooltipRefs: TooltipRefs
) {
  const root = hierarchy<InterLangDescendants>(
    tree,
    (d: InterLangDescendants) => d.children
  );

  const selectedItemNode = root.find(
    (d) => d.data.item.id === treeRootItem.id
  );
  const selectedItemNodeAncestors = selectedItemNode?.ancestors() ?? [];

  root
    .count()
    .sort(
      (a, b) =>
        +selectedItemNodeAncestors.includes(a) -
          +selectedItemNodeAncestors.includes(b) ||
        a.height - b.height ||
        (a.value ?? 0) - (b.value ?? 0) ||
        +(a.data.item.term < b.data.item.term) * 2 - 1
    );

  const fontSize = svgElement
    ? parseFloat(window.getComputedStyle(svgElement).fontSize)
    : 13;
  const dx = 10 * fontSize;
  const dy = fontSize;
  const sep = Math.floor(0.25 * fontSize);
  const layout = xMinClusterLayout<InterLangDescendants>()
    .nodeSize([dy, dx])
    .separation((a, b) => {
      const aAncestors = a.ancestors();
      const bAncestors = b.ancestors();
      for (
        let i = 0, j = 0;
        i < aAncestors.length &&
        j < bAncestors.length &&
        aAncestors[i].data.item.id !== bAncestors[j].data.item.id &&
        aAncestors[i].height === bAncestors[j].height;
        i++, j++
      ) {
        if (aAncestors[i].data.item.romanization) {
          return sep + 1;
        }
      }
      return sep;
    });

  const pointRoot = layout(root);

  let y0 = Infinity;
  let y1 = -y0;
  pointRoot.each((d) => {
    if (d.x > y1) y1 = d.x;
    if (d.x < y0) y0 = d.x;
  });

  const width = (root.height + 1) * dx;
  const height = y1 - y0 + dy * 4;
  const viewBox = [-dx / 2, y0 - dy * 2, width, height];

  const svg = configureSVG(svgElement, viewBox, width, height);

  renderTreeLinks(svg, pointRoot, curveStepBefore, {
    x: (d) => d.y,
    y: (d) => d.x,
  });

  const { node, nodeBackground } = renderTreeNodes(
    svg,
    pointRoot,
    treeRootItem,
    (d) => [d.node.y, d.node.x]
  );

  setTooltipListeners(
    node,
    setShowTooltip,
    setTooltipTreeNode,
    setTooltipSVGElement,
    setTooltipPositionKind,
    tooltipRefs
  );

  addSVGTextBackgrounds(node, nodeBackground, (d) => [d.node.y, d.node.x]);
}

function interLangDescendantsInner(
  root: InterLangDescendants
): InterLangDescendants[] {
  const children = [];
  for (const child of root.children) {
    if (child.item.lang.id === root.item.lang.id || root.parent) {
      child.parent = {
        item: root.item,
        ancestralLine: root.parent,
        langDistance: root.langDistance,
        etyMode: child.etyMode,
        otherParents: child.otherParents,
        etyOrder: child.parentEtyOrder,
      };
    }
    children.push(...interLangDescendantsInner(child));
  }
  if (
    root.parent &&
    root.parent.item.lang.id === root.item.lang.id &&
    root.children
  ) {
    return children;
  } else {
    root.children = children;
    return [root];
  }
}

export function interLangDescendants(
  fullRoot: Descendants
): InterLangDescendants {
  const root = fullRoot as InterLangDescendants;
  root.parent = null;
  return interLangDescendantsInner(root)[0];
}
