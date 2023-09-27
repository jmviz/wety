import { HierarchyPointNode } from "d3-hierarchy";
import { Selection } from "d3-selection";

export interface BoundedHierarchyPointNode<T> {
  node: HierarchyPointNode<T>;
  bbox: SVGRect;
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
