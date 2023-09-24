import "./Tree.css";
import { EtyData } from "./Ety";
import { Descendants, Etymology, term } from "../search/responses";
import { xMinClusterLayout, xMeanClusterLayout } from "./treeCluster";
import { TooltipState, hideTooltip, setNodeTooltipListeners } from "./Tooltip";

import { select, Selection } from "d3-selection";
import { link, curveStepBefore, curveStepAfter } from "d3-shape";
import {
  hierarchy,
  HierarchyPointLink,
  HierarchyPointNode,
} from "d3-hierarchy";
import { RefObject, useRef, useEffect, MutableRefObject } from "react";

interface TreeProps {
  etyData: EtyData<Etymology>;
  setTooltipState: (state: TooltipState) => void;
  tooltipRef: RefObject<HTMLDivElement>;
  tooltipShowTimeout: MutableRefObject<number | null>;
  tooltipHideTimeout: MutableRefObject<number | null>;
}

export default function Tree({
  etyData,
  setTooltipState,
  tooltipRef,
  tooltipShowTimeout,
  tooltipHideTimeout,
}: TreeProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    const svg = svgRef.current;
    if (svg === null) {
      return;
    }

    // treeSVG(
    etymologyTreeSVG(
      svg,
      etyData,
      setTooltipState,
      tooltipRef,
      tooltipShowTimeout,
      tooltipHideTimeout
    );

    return () => {
      // clear the previous svg
      select(svg).selectAll("*").remove();
      hideTooltip(tooltipRef);
      setTooltipState({
        itemNode: null,
        svgElement: null,
        positionType: "hover",
      });
    };
  }, [
    etyData,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout,
  ]);

  return <svg className="tree" ref={svgRef} />;
}

export interface ExpandedItemNode<T> {
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

// // The basic skeleton of this function is adapted from:
// //
// // https://observablehq.com/@d3/tree
// //
// // which has the following copyright and license notice:
// //
// // Copyright 2021 Observable, Inc.
// // Released under the ISC license.
// function descendantsTreeSVG(
//   svgElement: SVGSVGElement,
//   etyData: EtyData,
//   setTooltipState: (state: TooltipState) => void,
//   tooltipRef: RefObject<HTMLDivElement>,
//   tooltipShowTimeout: MutableRefObject<number | null>,
//   tooltipHideTimeout: MutableRefObject<number | null>
// ) {
//   const tree = etyData.tree;
//   const selectedItem = etyData.selectedItem;

//   if (tree === null || selectedItem === null) {
//     return;
//   }

//   // https://github.com/d3/d3-hierarchy#hierarchy
//   const root = hierarchy<Descendants>(tree, (d: Descendants) => d.children);

//   const selectedItemNode = root.find((d) => d.data.item.id === selectedItem.id);
//   const selectedItemNodeAncestors = selectedItemNode?.ancestors() ?? [];

//   root
//     .count() // counts node leaves and assigns count to .value
//     .sort(
//       (a, b) =>
//         +selectedItemNodeAncestors.includes(a) -
//           +selectedItemNodeAncestors.includes(b) ||
//         a.height - b.height ||
//         (a.value ?? 0) - (b.value ?? 0) ||
//         +(a.data.item.term < b.data.item.term) * 2 - 1
//     );

//   // There is a confusion between "x" and "y" concepts in the below. The d3
//   // api assumes that the tree is oriented vertically, with the root at the
//   // top and the leaves at the bottom. But we are using a horizontal tree,
//   // with the root on the left and the leaves on the right. So variables
//   // defined by d3 like e.g. `root.height` and `d.x` correspond in our case to
//   // width and y.
//   const fontSize = svgElement
//     ? parseFloat(window.getComputedStyle(svgElement).fontSize)
//     : 13;
//   const dx = 10 * fontSize;
//   const dy = fontSize;
//   const sep = Math.floor(0.25 * fontSize);
//   const layout = xMinClusterLayout<Descendants>()
//     .nodeSize([dy, dx])
//     .separation((a, b) => {
//       const aAncestors = a.ancestors();
//       const bAncestors = b.ancestors();
//       for (
//         let i = 0, j = 0;
//         i < aAncestors.length &&
//         j < bAncestors.length &&
//         aAncestors[i].data.item.id !== bAncestors[j].data.item.id &&
//         aAncestors[i].height === bAncestors[j].height;
//         i++, j++
//       ) {
//         if (aAncestors[i].data.item.romanization) {
//           return sep + 1;
//         }
//       }
//       return sep;
//     });

//   const pointRoot = layout(root);

//   // Center the tree vertically.
//   let y0 = Infinity;
//   let y1 = -y0;
//   pointRoot.each((d) => {
//     if (d.x > y1) y1 = d.x;
//     if (d.x < y0) y0 = d.x;
//   });

//   // root.height is the number of links between the root and the furthest leaf.
//   const width = (root.height + 1) * dx;
//   const height = y1 - y0 + dy * 4;

//   const viewBox = [-dx / 2, y0 - dy * 2, width, height];

//   // crispEdges implementation quality varies from browser to browser. It
//   // generally seems to work well but for example Windows Firefox renders
//   // random lines with 2px instead of 1px. Consider this as a solution:
//   // https://github.com/engray/subpixelFix.
//   const svg = select(svgElement)
//     .attr("version", "1.1")
//     .attr("xmlns", "http://www.w3.org/2000/svg")
//     .attr("xmlns:xlink", "http://www.w3.org/1999/xlink")
//     .attr("xmlns:xhtml", "http://www.w3.org/1999/xhtml")
//     .attr("viewBox", viewBox)
//     .attr("width", width)
//     .attr("height", height)
//     .attr(
//       "style",
//       `min-width: ${width}px; max-width: ${width}px; height: auto; height: intrinsic;`
//     )
//     .attr("shape-rendering", "crispEdges")
//     .attr("vector-effect", "non-scaling-stroke")
//     .attr("text-anchor", "middle")
//     .attr("text-rendering", "optimizeLegibility")
//     // this noop event listener is to cajole mobile browsers (or, at least,
//     // ios webkit) into responding to touch events on svg elements, cf.
//     // https://stackoverflow.com/a/65777666/10658294
//     // eslint-disable-next-line @typescript-eslint/no-empty-function
//     .on("touchstart", () => {});

//   // the lines forming the tree
//   svg
//     .append("g")
//     .attr("fill", "none")
//     .attr("stroke", "#555")
//     .attr("stroke-opacity", 1.0)
//     .attr("stroke-linecap", "butt")
//     .attr("stroke-linejoin", "miter")
//     .attr("stroke-width", 1.0)
//     .selectAll("path")
//     .data(pointRoot.links())
//     .join("path")
//     .attr(
//       "d",
//       link<HierarchyPointLink<Descendants>, HierarchyPointNode<Descendants>>(
//         curveStepBefore
//       )
//         .x((d) => d.y)
//         .y((d) => d.x)
//     );

//   const descendants: ExpandedItemNode<Descendants>[] = pointRoot
//     .descendants()
//     .map(function (d) {
//       return { node: d, bbox: new DOMRect(0, 0, 0, 0) };
//     });

//   // placeholder rects for text backgrounds to be set in addSVGTextBackgrounds()
//   const nodeBackground = svg
//     .append("g")
//     .selectAll<SVGRectElement, unknown>("rect")
//     .data(descendants)
//     .join("rect")
//     .attr("fill", "white");

//   // the text nodes
//   const node = svg
//     .append("g")
//     .selectAll<SVGTextElement, unknown>("g")
//     .data(descendants)
//     .join("g")
//     .attr("font-weight", (d) =>
//       d.node.data.item.id === selectedItem.id ? "bold" : null
//     )
//     .attr("transform", (d) => `translate(${d.node.y},${d.node.x})`);

//   node
//     .append("text")
//     .attr("class", "lang")
//     .attr("y", "-1em")
//     .attr("fill", (d) => langColor(d.node.data.langDistance))
//     .text((d) => d.node.data.item.lang);

//   node
//     .append("text")
//     .attr("class", "term")
//     .attr("y", "0.25em")
//     .text((d) => term(d.node.data.item));

//   node
//     .append("text")
//     .attr("class", "romanization")
//     .attr("y", "1.5em")
//     .text((d) =>
//       d.node.data.item.romanization ? `(${d.node.data.item.romanization})` : ""
//     );

//   setNodeTooltipListeners(
//     node,
//     setTooltipState,
//     tooltipRef,
//     tooltipShowTimeout,
//     tooltipHideTimeout
//   );

//   addSVGTextBackgrounds(node, nodeBackground);
// }

function etymologyTreeSVG(
  svgElement: SVGSVGElement,
  etyData: EtyData<Etymology>,
  setTooltipState: (state: TooltipState) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  const tree = etyData.tree;
  const selectedItem = etyData.selectedItem;

  if (tree === null || selectedItem === null) {
    return;
  }

  // https://github.com/d3/d3-hierarchy#hierarchy
  const root = hierarchy<Etymology>(tree, (d: Etymology) => d.parents);

  root.sort((a, b) => 1);

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

  // Center the tree horizontally.
  let x0 = Infinity;
  let x1 = -x0;
  pointRoot.each((d) => {
    if (d.x > x1) x1 = d.x;
    if (d.x < x0) x0 = d.x;
  });

  // const nLeaves = root.leaves().length;
  // const width = nLeaves * (dx + sep) - sep;
  const width = x1 - x0 + dx;
  const height = (root.height + 1) * dy;

  const viewBox = [x0 - dx / 2, -dy / 2, width, height];

  // crispEdges implementation quality varies from browser to browser. It
  // generally seems to work well but for example Windows Firefox renders
  // random lines with 2px instead of 1px. Consider this as a solution:
  // https://github.com/engray/subpixelFix.
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
    // this noop event listener is to cajole mobile browsers (or, at least,
    // ios webkit) into responding to touch events on svg elements, cf.
    // https://stackoverflow.com/a/65777666/10658294
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    .on("touchstart", () => {});

  // the lines forming the tree
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

  // Confusingly, with respect to the tree structure and d3 api, these are
  // descendants. But with respect to the etymology, they are ancestors.
  const ancestors: ExpandedItemNode<Etymology>[] = pointRoot
    .descendants()
    .map(function (d) {
      return { node: d, bbox: new DOMRect(0, 0, 0, 0) };
    });

  // placeholder rects for text backgrounds to be set in addSVGTextBackgrounds()
  const nodeBackground = svg
    .append("g")
    .selectAll<SVGRectElement, unknown>("rect")
    .data(ancestors)
    .join("rect")
    .attr("fill", "white");

  // the text nodes
  const node = svg
    .append("g")
    .selectAll<SVGTextElement, unknown>("g")
    .data(ancestors)
    .join("g")
    .attr("font-weight", (d) =>
      d.node.data.item.id === selectedItem.id ? "bold" : null
    )
    .attr("transform", (d) => `translate(${d.node.x},${d.node.y})`);

  node
    .append("text")
    .attr("class", "lang")
    .attr("y", "-1em")
    .attr("fill", (d) => langColor(d.node.data.langDistance))
    .text((d) => d.node.data.item.lang);

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

  setNodeTooltipListeners(
    node,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout
  );

  addSVGTextBackgrounds(node, nodeBackground);
}

function addSVGTextBackgrounds(
  node: Selection<
    SVGGElement | SVGTextElement,
    ExpandedItemNode<Etymology>,
    SVGGElement,
    undefined
  >,
  nodeBackground: Selection<
    SVGRectElement,
    ExpandedItemNode<Etymology>,
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
