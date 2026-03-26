import { Item, term } from "../search/types";
import { PositionKind, hideTooltip } from "./tooltip";

import { select, Selection } from "d3-selection";
import { link, CurveFactory } from "d3-shape";
import { HierarchyPointLink, HierarchyPointNode } from "d3-hierarchy";
import { MutableRefObject, RefObject } from "react";

export interface BoundedHierarchyPointNode<T> {
  node: HierarchyPointNode<T>;
  bbox: SVGRect;
}

// https://accessiblepalette.com/?lightness=98.2,93.95,85.1,76.5,67.65,52,47.6,40.4,32.4,23.55&770039=1,12&720614=1,0&672000=1,0&493500=1,0&224000=1,0&004300=1,0&004a32=1,0&004f64=1,0&004e94=1,0&003c88=1,0&2e2d79=1,0&750039=1,0
const langDistanceColors = [
  "#2F2E7A",
  "#0B3577",
  "#143867",
  "#0D3D4D",
  "#06412C",
  "#004300",
  "#224000",
  "#493500",
  "#672001",
  "#740A16",
  "#740549",
  "#730138",
];

const langUnrelatedColor = "#696969";

export function langColor(distance: number | null) {
  if (distance === null) return langUnrelatedColor;
  if (distance < 0) return langDistanceColors[0];
  if (distance > langDistanceColors.length)
    return langDistanceColors[langDistanceColors.length - 1];
  return langDistanceColors[distance];
}

export function configureSVG(
  svgElement: SVGSVGElement,
  viewBox: number[],
  width: number,
  height: number
): Selection<SVGSVGElement, unknown, null, undefined> {
  // crispEdges implementation quality varies from browser to browser. It
  // generally seems to work well but for example Windows Firefox renders
  // random lines with 2px instead of 1px. Consider this as a solution:
  // https://github.com/engray/subpixelFix.
  return select(svgElement)
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
    // this noop event listener is to cajole mobile browsers (or, at least,
    // ios webkit) into responding to touch events on svg elements, cf.
    // https://stackoverflow.com/a/65777666/10658294
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    .on("touchstart", () => {});
}

export function renderTreeLinks<T>(
  svg: Selection<SVGSVGElement, unknown, null, undefined>,
  pointRoot: HierarchyPointNode<T>,
  curve: CurveFactory,
  coords: {
    x: (d: HierarchyPointNode<T>) => number;
    y: (d: HierarchyPointNode<T>) => number;
  }
): void {
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
      link<HierarchyPointLink<T>, HierarchyPointNode<T>>(curve)
        .x((d) => coords.x(d))
        .y((d) => coords.y(d))
    );
}

export function renderTreeNodes<
  T extends { item: Item; langDistance: number }
>(
  svg: Selection<SVGSVGElement, unknown, null, undefined>,
  pointRoot: HierarchyPointNode<T>,
  treeRootItem: Item,
  coords: (node: HierarchyPointNode<T>) => [number, number]
): {
  node: Selection<
    SVGGElement,
    BoundedHierarchyPointNode<T>,
    SVGGElement,
    undefined
  >;
  nodeBackground: Selection<
    SVGRectElement,
    BoundedHierarchyPointNode<T>,
    SVGGElement,
    undefined
  >;
} {
  const nodes: BoundedHierarchyPointNode<T>[] = pointRoot
    .descendants()
    .map(function (d) {
      return { node: d, bbox: new DOMRect(0, 0, 0, 0) };
    });

  // placeholder rects for text backgrounds to be set in addSVGTextBackgrounds()
  const nodeBackground = svg
    .append("g")
    .selectAll<SVGRectElement, unknown>("rect")
    .data(nodes)
    .join("rect")
    .attr("fill", "white");

  // the text nodes
  const node = svg
    .append("g")
    .selectAll<SVGTextElement, unknown>("g")
    .data(nodes)
    .join("g")
    .attr("font-weight", (d) =>
      d.node.data.item.id === treeRootItem.id ? "bold" : null
    )
    .attr("transform", (d) => {
      const [x, y] = coords(d.node);
      return `translate(${x},${y})`;
    });

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

  return { node, nodeBackground };
}

export function addSVGTextBackgrounds<T>(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<T>,
    SVGGElement,
    undefined
  >,
  nodeBackground: Selection<
    SVGRectElement,
    BoundedHierarchyPointNode<T>,
    SVGGElement,
    undefined
  >,
  coords: (d: BoundedHierarchyPointNode<T>) => [number, number]
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
      const [x, y] = coords(d);
      return `translate(${x - xMargin},${y - yMargin})`;
    })
    .attr("x", (d) => d.bbox.x)
    .attr("y", (d) => d.bbox.y);
}

export function setTooltipListeners<T>(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<T>,
    SVGGElement,
    undefined
  >,
  setShowTooltip: (show: boolean) => void,
  setTooltipTreeNode: (node: HierarchyPointNode<T> | null) => void,
  setTooltipSVGElement: (element: SVGElement | null) => void,
  setTooltipPositionKind: (kind: PositionKind) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
): void {
  // for non-mouse, show tooltip on pointerup
  node.on(
    "pointerup",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<T>) {
      if (event.pointerType !== "mouse") {
        setShowTooltip(true);
        setTooltipTreeNode(d.node);
        setTooltipSVGElement(this);
        setTooltipPositionKind(PositionKind.Fixed);
      }
    }
  );

  // for mouse, show tooltip on hover
  node.on(
    "pointerenter",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<T>) {
      if (event.pointerType === "mouse") {
        window.clearTimeout(tooltipHideTimeout.current ?? undefined);
        tooltipShowTimeout.current = window.setTimeout(() => {
          setShowTooltip(true);
          setTooltipTreeNode(d.node);
          setTooltipSVGElement(this);
          setTooltipPositionKind(PositionKind.Hover);
        }, 100);
      }
    }
  );

  node.on("pointerleave", (event: PointerEvent) => {
    if (event.pointerType === "mouse") {
      window.clearTimeout(tooltipShowTimeout.current ?? undefined);
      tooltipHideTimeout.current = window.setTimeout(
        () => hideTooltip(tooltipRef, setShowTooltip),
        100
      );
    }
  });
}
