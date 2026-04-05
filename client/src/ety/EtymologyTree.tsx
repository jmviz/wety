import "./EtymologyTree.css";
import {
  Etymology,
  InterLangDescendants,
  Item,
  term,
} from "../search/types";
import { xMeanClusterLayout } from "./treeCluster";
import EtymologyTooltip, {
  setEtymologyTooltipListeners,
} from "./EtymologyTooltip";
import { PositionKind, hideTooltip, TooltipRefs } from "./tooltip";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import { lastRequest } from "../state";

import { select, Selection } from "d3-selection";
import { link, curveStepAfter } from "d3-shape";
import {
  hierarchy,
  HierarchyPointLink,
  HierarchyPointNode,
} from "d3-hierarchy";
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
      <EtymologyTooltip
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

  const svg = select(svgElement)
    .attr("version", "1.1")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("xmlns:xlink", "http://www.w3.org/1999/xlink")
    .attr("xmlns:xhtml", "http://www.w3.org/1999/xhtml")
    .attr("viewBox", viewBox)
    .attr("width", width)
    .attr("height", height)
    .attr(
      "style",
      `min-width: ${width}px; max-width: ${width}px; height: auto; height: intrinsic;`
    )
    .attr("shape-rendering", "crispEdges")
    .attr("vector-effect", "non-scaling-stroke")
    .attr("text-anchor", "middle")
    .attr("text-rendering", "optimizeLegibility")
    .on("touchstart", () => {});

  svg
    .append("g")
    .attr("fill", "none")
    .attr("stroke", "#555")
    .attr("stroke-opacity", 1.0)
    .attr("stroke-linecap", "butt")
    .attr("stroke-linejoin", "miter")
    .attr("stroke-width", 1.0)
    .selectAll("path")
    .data(pointRoot.links())
    .join("path")
    .attr(
      "d",
      link<HierarchyPointLink<Etymology>, HierarchyPointNode<Etymology>>(
        curveStepAfter
      )
        .x((d) => d.x)
        .y((d) => d.y)
    );

  const ancestors: BoundedHierarchyPointNode<Etymology>[] = pointRoot
    .descendants()
    .map((d) => ({ node: d, bbox: new DOMRect(0, 0, 0, 0) }));

  const nodeBackground = svg
    .append("g")
    .selectAll<SVGRectElement, unknown>("rect")
    .data(ancestors)
    .join("rect")
    .attr("fill", "white");

  const node = svg
    .append("g")
    .selectAll<SVGTextElement, unknown>("g")
    .data(ancestors)
    .join("g")
    .attr("font-weight", (d) =>
      d.node.data.item.id === treeRootItem.id ? "bold" : null
    )
    .attr("transform", (d) => `translate(${d.node.x},${d.node.y})`);

  node
    .append("text")
    .attr("class", "lang")
    .attr("y", "-1em")
    .attr("fill", (d) => langColor(d.node.data.langDistance))
    .text((d) => d.node.data.item.lang.name);

  node
    .append("text")
    .attr("class", "term")
    .attr("y", "0.25em")
    .text((d) => term(d.node.data.item));

  node
    .append("text")
    .attr("class", "romanization")
    .attr("y", "1.5em")
    .text((d) =>
      d.node.data.item.romanization ? `(${d.node.data.item.romanization})` : ""
    );

  setEtymologyTooltipListeners(
    node,
    setShowTooltip,
    setTooltipTreeNode,
    setTooltipSVGElement,
    setTooltipPositionKind,
    tooltipRefs
  );

  addSVGTextBackgrounds(node, nodeBackground);
}

function addSVGTextBackgrounds(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<Etymology>,
    SVGGElement,
    undefined
  >,
  nodeBackground: Selection<
    SVGRectElement,
    BoundedHierarchyPointNode<Etymology>,
    SVGGElement,
    undefined
  >
) {
  node.each(function (d) {
    d.bbox = this.getBBox();
  });

  const xMargin = 3;
  const yMargin = 3;

  nodeBackground
    .attr("width", (d) => d.bbox.width + 2 * xMargin)
    .attr("height", (d) => d.bbox.height + 2 * yMargin)
    .attr("transform", (d) => {
      const x = d.node.x - xMargin;
      const y = d.node.y - yMargin;
      return `translate(${x},${y})`;
    })
    .attr("x", (d) => d.bbox.x)
    .attr("y", (d) => d.bbox.y);
}
