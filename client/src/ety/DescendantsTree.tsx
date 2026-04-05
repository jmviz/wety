import "./DescendantsTree.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  term,
} from "../search/types";
import { xMinClusterLayout } from "./treeCluster";
import DescendantsTooltip, {
  setDescendantsTooltipListeners,
} from "./DescendantsTooltip";
import { PositionKind, hideTooltip } from "./tooltip";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import { lastRequest } from "../signals";

import { select, Selection } from "d3-selection";
import { link, curveStepBefore } from "d3-shape";
import {
  hierarchy,
  HierarchyPointLink,
  HierarchyPointNode,
} from "d3-hierarchy";
import { useSignal } from "@preact/signals";
import { useRef, useEffect, useMemo } from "preact/hooks";

interface DescendantsTreeProps {
  tree: Etymology | InterLangDescendants[] | null;
}

export default function DescendantsTree({ tree }: DescendantsTreeProps) {
  const showTooltip = useSignal(false);
  const tooltipTreeNode =
    useSignal<HierarchyPointNode<InterLangDescendants> | null>(null);
  const tooltipSVGElement = useSignal<SVGElement | null>(null);
  const tooltipPositionKind = useSignal<PositionKind>(PositionKind.Hover);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  const request = lastRequest.value;

  const svgRefs = useMemo(() => {
    if (!Array.isArray(tree)) {
      return [];
    }
    return tree.map(() => ({ current: null as SVGSVGElement | null }));
  }, [tree]);

  useEffect(() => {
    if (!Array.isArray(tree)) {
      return;
    }

    const cleanupSVGs: (() => void)[] = [];

    for (let index = 0; index < tree.length; index++) {
      const svgRef = svgRefs[index];
      const svg = svgRef.current;

      if (!svg || !tree[index] || !request) {
        return;
      }

      descendantsTreeSVG(
        svg,
        tree[index] as InterLangDescendants,
        request.item,
        showTooltip,
        tooltipTreeNode,
        tooltipSVGElement,
        tooltipPositionKind,
        tooltipRef,
        tooltipShowTimeout,
        tooltipHideTimeout
      );

      cleanupSVGs.push(() => select(svg).selectAll("*").remove());
    }

    return () => {
      cleanupSVGs.forEach((cleanup) => cleanup());
      hideTooltip(tooltipRef, showTooltip);
      showTooltip.value = false;
      tooltipTreeNode.value = null;
      tooltipSVGElement.value = null;
      tooltipPositionKind.value = PositionKind.Hover;
    };
  }, [tree, request, svgRefs]);

  return (
    <div class="tree-container">
      {svgRefs.map((ref, index) => (
        <svg
          key={index}
          class="tree"
          ref={(el) => {
            ref.current = el;
          }}
        />
      ))}
      <DescendantsTooltip
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

function descendantsTreeSVG(
  svgElement: SVGSVGElement,
  tree: InterLangDescendants,
  treeRootItem: Item,
  showTooltip: { value: boolean },
  tooltipTreeNode: {
    value: HierarchyPointNode<InterLangDescendants> | null;
  },
  tooltipSVGElement: { value: SVGElement | null },
  tooltipPositionKind: { value: PositionKind },
  tooltipRef: { current: HTMLDivElement | null },
  tooltipShowTimeout: { current: number | null },
  tooltipHideTimeout: { current: number | null }
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

  const nodeBackground = svg
    .append("g")
    .selectAll<SVGRectElement, unknown>("rect")
    .data(descendants)
    .join("rect")
    .attr("fill", "white");

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
      d.node.data.item.romanization
        ? `(${d.node.data.item.romanization})`
        : ""
    );

  setDescendantsTooltipListeners(
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
