import "./Tree.css";
import {
  Etymology,
  InterLangDescendants,
  Item,
  TreeKind,
} from "../search/types";
import { xMeanClusterLayout } from "./treeCluster";
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
import { curveStepAfter } from "d3-shape";
import { hierarchy, HierarchyPointNode } from "d3-hierarchy";
import { createSignal, createEffect, onCleanup, Setter } from "solid-js";

interface EtymologyTreeProps {
  tree: Etymology | InterLangDescendants[] | null;
}

export default function EtymologyTree(props: EtymologyTreeProps) {
  const [showTooltip, setShowTooltip] = createSignal(false);
  const [tooltipTreeNode, setTooltipTreeNode] =
    createSignal<HierarchyPointNode<Etymology> | null>(null);
  const [tooltipSVGElement, setTooltipSVGElement] =
    createSignal<SVGElement | null>(null);
  const [tooltipPositionKind, setTooltipPositionKind] =
    createSignal<PositionKind>(PositionKind.Hover);

  let svgEl: SVGSVGElement | undefined;
  const tooltipRefs: TooltipRefs = {
    el: undefined,
    showTimeout: null,
    hideTimeout: null,
    justDismissed: false,
  };

  createEffect(() => {
    const t = props.tree;
    const request = lastRequest();
    if (!svgEl || t === null || request === null) return;

    etymologyTreeSVG(
      svgEl,
      t as Etymology,
      request.item,
      setShowTooltip,
      setTooltipTreeNode,
      setTooltipSVGElement,
      setTooltipPositionKind,
      tooltipRefs
    );

    onCleanup(() => {
      select(svgEl!).selectAll("*").remove();
      hideTooltip(tooltipRefs, setShowTooltip);
      setShowTooltip(false);
      setTooltipTreeNode(null);
      setTooltipSVGElement(null);
      setTooltipPositionKind(PositionKind.Hover);
    });
  });

  return (
    <div class="tree-container">
      <svg class="tree" ref={svgEl} />
      <TreeTooltip
        treeKind={TreeKind.Etymology}
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

function etymologyTreeSVG(
  svgElement: SVGSVGElement,
  tree: Etymology,
  treeRootItem: Item,
  setShowTooltip: Setter<boolean>,
  setTooltipTreeNode: Setter<HierarchyPointNode<Etymology> | null>,
  setTooltipSVGElement: Setter<SVGElement | null>,
  setTooltipPositionKind: Setter<PositionKind>,
  tooltipRefs: TooltipRefs
) {
  const root = hierarchy<Etymology>(tree, (d: Etymology) => d.parents);

  root.sort((a, b) => b.data.etyOrder - a.data.etyOrder);

  const fontSize = svgElement
    ? parseFloat(window.getComputedStyle(svgElement).fontSize)
    : 13;
  const dx = 15 * fontSize;
  const dy = 5 * fontSize;
  const sep = Math.floor(0.1 * fontSize);
  const layout = xMeanClusterLayout<Etymology>()
    .nodeSize([dx, dy])
    .separation(() => sep);

  const pointRoot = layout(root);

  let x0 = Infinity;
  let x1 = -x0;
  pointRoot.each((d) => {
    if (d.x > x1) x1 = d.x;
    if (d.x < x0) x0 = d.x;
  });

  const width = x1 - x0 + dx;
  const height = (root.height + 1) * dy;
  const viewBox = [x0 - dx / 2, -dy / 2, width, height];

  const svg = configureSVG(svgElement, viewBox, width, height);

  renderTreeLinks(svg, pointRoot, curveStepAfter, {
    x: (d) => d.x,
    y: (d) => d.y,
  });

  const { node, nodeBackground } = renderTreeNodes(
    svg,
    pointRoot,
    treeRootItem,
    (d) => [d.node.x, d.node.y]
  );

  setTooltipListeners(
    node,
    setShowTooltip,
    setTooltipTreeNode,
    setTooltipSVGElement,
    setTooltipPositionKind,
    tooltipRefs
  );

  addSVGTextBackgrounds(node, nodeBackground, (d) => [d.node.x, d.node.y]);
}
