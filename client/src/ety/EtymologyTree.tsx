import "./EtymologyTree.css";
import { TreeData } from "../App";
import { Etymology, Item, term } from "../search/responses";
import { xMeanClusterLayout } from "./treeCluster";
import EtymologyTooltip, {
  EtymologyTooltipState,
  setEtymologyTooltipListeners,
} from "./EtymologyTooltip";
import { PositionKind, hideTooltip } from "./tooltip";
import {
  addSVGTextBackgrounds,
  BoundedHierarchyPointNode,
  langColor,
} from "./tree";

import { select } from "d3-selection";
import { link, curveStepAfter } from "d3-shape";
import {
  hierarchy,
  HierarchyPointLink,
  HierarchyPointNode,
} from "d3-hierarchy";
import {
  RefObject,
  useRef,
  useEffect,
  MutableRefObject,
  useState,
} from "react";

interface EtymologyTreeProps {
  treeData: TreeData;
  setTreeData: (treeData: TreeData) => void;
}

export default function EtymologyTree({
  treeData,
  setTreeData,
}: EtymologyTreeProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltipState, setTooltipState] = useState<EtymologyTooltipState>({
    itemNode: null,
    svgElement: null,
    positionKind: PositionKind.Hover,
  });
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  useEffect(() => {
    const svg = svgRef.current;
    const tree = treeData.tree;
    const selectedItem = treeData.selectedItem;

    if (svg === null || tree === null || selectedItem === null) {
      return;
    }

    etymologyTreeSVG(
      svg,
      tree as Etymology,
      selectedItem,
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
        positionKind: PositionKind.Hover,
      });
    };
  }, [
    treeData,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout,
  ]);

  return (
    <div className="tree-container">
      <svg className="tree" ref={svgRef} />
      <EtymologyTooltip
        state={tooltipState}
        treeData={treeData}
        setTreeData={setTreeData}
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
  selectedItem: Item,
  setTooltipState: (state: EtymologyTooltipState) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // https://github.com/d3/d3-hierarchy#hierarchy
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
  const ancestors: BoundedHierarchyPointNode<Etymology>[] = pointRoot
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

  setEtymologyTooltipListeners(
    node,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout
  );

  addSVGTextBackgrounds<Etymology>(node, nodeBackground);
}
