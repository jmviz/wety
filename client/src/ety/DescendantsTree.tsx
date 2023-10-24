import "./DescendantsTree.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  term,
  TreeRequest,
} from "../search/types";
import { xMinClusterLayout } from "./treeCluster";
import DescendantsTooltip, {
  DescendantsTooltipState,
  setDescendantsTooltipListeners,
} from "./DescendantsTooltip";
import { PositionKind, hideTooltip } from "./tooltip";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import { TreeKind } from "../search/types";

import { select, Selection } from "d3-selection";
import { link, curveStepBefore } from "d3-shape";
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

interface DescendantsTreeProps {
  setSelectedLang: (lang: Lang | null) => void;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedTreeKind: (treeKind: TreeKind) => void;
  tree: Etymology | InterLangDescendants | null;
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  lastRequest: TreeRequest | null;
  setLastRequest: (request: TreeRequest | null) => void;
}

export default function DescendantsTree({
  setSelectedLang,
  setSelectedItem,
  selectedDescLangs,
  setSelectedTreeKind,
  tree,
  setTree,
  lastRequest,
  setLastRequest,
}: DescendantsTreeProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltipState, setTooltipState] = useState<DescendantsTooltipState>({
    itemNode: null,
    svgElement: null,
    positionKind: PositionKind.Hover,
  });
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  useEffect(() => {
    const svg = svgRef.current;

    if (svg === null || tree === null || lastRequest === null) {
      return;
    }

    descendantsTreeSVG(
      svg,
      tree as InterLangDescendants,
      lastRequest.item,
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
    tree,
    lastRequest,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout,
  ]);

  return (
    <div className="tree-container">
      <svg className="tree" ref={svgRef} />
      <DescendantsTooltip
        state={tooltipState}
        setSelectedLang={setSelectedLang}
        setSelectedItem={setSelectedItem}
        selectedDescLangs={selectedDescLangs}
        setTree={setTree}
        setSelectedTreeKind={setSelectedTreeKind}
        divRef={tooltipRef}
        showTimeout={tooltipShowTimeout}
        hideTimeout={tooltipHideTimeout}
        lastRequest={lastRequest}
        setLastRequest={setLastRequest}
      />
    </div>
  );
}

function descendantsTreeSVG(
  svgElement: SVGSVGElement,
  tree: InterLangDescendants,
  treeRootItem: Item,
  setTooltipState: (state: DescendantsTooltipState) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // https://github.com/d3/d3-hierarchy#hierarchy
  const root = hierarchy<InterLangDescendants>(
    tree,
    (d: InterLangDescendants) => d.children
  );

  const selectedItemNode = root.find((d) => d.data.item.id === treeRootItem.id);
  const selectedItemNodeAncestors = selectedItemNode?.ancestors() ?? [];

  root
    .count() // counts node leaves and assigns count to .value
    .sort(
      (a, b) =>
        +selectedItemNodeAncestors.includes(a) -
          +selectedItemNodeAncestors.includes(b) ||
        a.height - b.height ||
        (a.value ?? 0) - (b.value ?? 0) ||
        +(a.data.item.term < b.data.item.term) * 2 - 1
    );

  // There is a confusion between "x" and "y" concepts in the below. The d3
  // api assumes that the tree is oriented vertically, with the root at the
  // top and the leaves at the bottom. But we are using a horizontal tree,
  // with the root on the left and the leaves on the right. So variables
  // defined by d3 like e.g. `root.height` and `d.x` correspond in our case to
  // width and y.
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

  // Center the tree vertically.
  let y0 = Infinity;
  let y1 = -y0;
  pointRoot.each((d) => {
    if (d.x > y1) y1 = d.x;
    if (d.x < y0) y0 = d.x;
  });

  // root.height is the number of links between the root and the furthest leaf.
  const width = (root.height + 1) * dx;
  const height = y1 - y0 + dy * 4;

  const viewBox = [-dx / 2, y0 - dy * 2, width, height];

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
      link<
        HierarchyPointLink<InterLangDescendants>,
        HierarchyPointNode<InterLangDescendants>
      >(curveStepBefore)
        .x((d) => d.y)
        .y((d) => d.x)
    );

  const descendants: BoundedHierarchyPointNode<InterLangDescendants>[] =
    pointRoot.descendants().map(function (d) {
      return { node: d, bbox: new DOMRect(0, 0, 0, 0) };
    });

  // placeholder rects for text backgrounds to be set in addSVGTextBackgrounds()
  const nodeBackground = svg
    .append("g")
    .selectAll<SVGRectElement, unknown>("rect")
    .data(descendants)
    .join("rect")
    .attr("fill", "white");

  // the text nodes
  const node = svg
    .append("g")
    .selectAll<SVGTextElement, unknown>("g")
    .data(descendants)
    .join("g")
    .attr("font-weight", (d) =>
      d.node.data.item.id === treeRootItem.id ? "bold" : null
    )
    .attr("transform", (d) => `translate(${d.node.y},${d.node.x})`);

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

  setDescendantsTooltipListeners(
    node,
    setTooltipState,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout
  );

  addSVGTextBackgrounds(node, nodeBackground);
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

function addSVGTextBackgrounds(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<InterLangDescendants>,
    SVGGElement,
    undefined
  >,
  nodeBackground: Selection<
    SVGRectElement,
    BoundedHierarchyPointNode<InterLangDescendants>,
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
      const x = d.node.y - xMargin;
      const y = d.node.x - yMargin;
      return `translate(${x},${y})`;
    })
    .attr("x", (d) => d.bbox.x)
    .attr("y", (d) => d.bbox.y);
}
