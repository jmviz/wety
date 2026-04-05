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
import { PositionKind, hideTooltip } from "./tooltip";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import { lastRequest } from "../signals";

import { select, Selection } from "d3-selection";
import { link, curveStepAfter } from "d3-shape";
import {
  hierarchy,
  HierarchyPointLink,
  HierarchyPointNode,
} from "d3-hierarchy";
import { useSignal } from "@preact/signals";
import { useRef, useEffect } from "preact/hooks";

interface EtymologyTreeProps {
  tree: Etymology | InterLangDescendants[] | null;
}

export default function EtymologyTree({ tree }: EtymologyTreeProps) {
  const showTooltip = useSignal(false);
  const tooltipTreeNode =
    useSignal<HierarchyPointNode<Etymology> | null>(null);
  const tooltipSVGElement = useSignal<SVGElement | null>(null);
  const tooltipPositionKind = useSignal<PositionKind>(PositionKind.Hover);
  const svgRef = useRef<SVGSVGElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  const request = lastRequest.value;

  useEffect(() => {
    const svg = svgRef.current;

    if (svg === null || tree === null || request === null) {
      return;
    }

    etymologyTreeSVG(
      svg,
      tree as Etymology,
      request.item,
      showTooltip,
      tooltipTreeNode,
      tooltipSVGElement,
      tooltipPositionKind,
      tooltipRef,
      tooltipShowTimeout,
      tooltipHideTimeout
    );

    return () => {
      select(svg).selectAll("*").remove();
      hideTooltip(tooltipRef, showTooltip);
      showTooltip.value = false;
      tooltipTreeNode.value = null;
      tooltipSVGElement.value = null;
      tooltipPositionKind.value = PositionKind.Hover;
    };
  }, [tree, request]);

  return (
    <div class="tree-container">
      <svg class="tree" ref={svgRef} />
      <EtymologyTooltip
        showTooltip={showTooltip}
        treeNode={tooltipTreeNode}
        svgElement={tooltipSVGElement}
        positionKind={tooltipPositionKind}
        divRef={tooltipRef}
        showTimeout={tooltipShowTimeout}
        hideTimeout={tooltipHideTimeout}
      />
    </div>
  );
}

function etymologyTreeSVG(
  svgElement: SVGSVGElement,
  tree: Etymology,
  treeRootItem: Item,
  showTooltip: { value: boolean },
  tooltipTreeNode: { value: HierarchyPointNode<Etymology> | null },
  tooltipSVGElement: { value: SVGElement | null },
  tooltipPositionKind: { value: PositionKind },
  tooltipRef: { current: HTMLDivElement | null },
  tooltipShowTimeout: { current: number | null },
  tooltipHideTimeout: { current: number | null }
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
    .separation((a, b) => sep);

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
    .map(function (d) {
      return { node: d, bbox: new DOMRect(0, 0, 0, 0) };
    });

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
    showTooltip,
    tooltipTreeNode,
    tooltipSVGElement,
    tooltipPositionKind,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout
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
    .attr("transform", function (d) {
      const x = d.node.x - xMargin;
      const y = d.node.y - yMargin;
      return `translate(${x},${y})`;
    })
    .attr("x", (d) => d.bbox.x)
    .attr("y", (d) => d.bbox.y);
}
