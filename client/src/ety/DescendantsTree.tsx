import "./Tree.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeRequest,
} from "../search/types";
import { xMinClusterLayout } from "./treeCluster";
import TreeTooltip from "./TreeTooltip";
import { PositionKind, hideTooltip } from "./tooltip";
import {
  addSVGTextBackgrounds,
  configureSVG,
  renderTreeLinks,
  renderTreeNodes,
  setTooltipListeners,
} from "./tree";
import { TreeKind } from "../search/types";

import { curveStepBefore } from "d3-shape";
import { hierarchy, HierarchyPointNode } from "d3-hierarchy";
import { select } from "d3-selection";
import {
  RefObject,
  useRef,
  useEffect,
  useMemo,
  createRef,
  useState,
} from "react";

interface DescendantsTreeProps {
  setSelectedLang: (lang: Lang | null) => void;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedTreeKind: (treeKind: TreeKind) => void;
  tree: Etymology | InterLangDescendants[] | null;
  setTree: (tree: Etymology | InterLangDescendants[] | null) => void;
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
  const [showTooltip, setShowTooltip] = useState(false);
  const [tooltipTreeNode, setTooltipTreeNode] =
    useState<HierarchyPointNode<InterLangDescendants> | null>(null);
  const [tooltipSVGElement, setTooltipSVGElement] = useState<SVGElement | null>(
    null
  );
  const [tooltipPositionKind, setTooltipPositionKind] = useState<PositionKind>(
    PositionKind.Hover
  );
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  const svgRefs = useMemo(() => {
    if (!Array.isArray(tree)) {
      return [];
    }

    const refs: RefObject<SVGSVGElement>[] = [];
    for (let index = 0; index < tree.length; index++) {
      refs.push(createRef<SVGSVGElement>());
    }
    return refs;
  }, [tree]);

  useEffect(() => {
    if (!Array.isArray(tree)) {
      return;
    }

    const cleanupSVGs: (() => void)[] = [];

    for (let index = 0; index < tree.length; index++) {
      const svgRef = svgRefs[index];
      const svg = svgRef.current;

      if (!svg || !tree[index] || !lastRequest) {
        return;
      }

      descendantsTreeSVG(
        svg,
        tree[index] as InterLangDescendants,
        lastRequest.item,
        setShowTooltip,
        setTooltipTreeNode,
        setTooltipSVGElement,
        setTooltipPositionKind,
        tooltipRef,
        tooltipShowTimeout,
        tooltipHideTimeout
      );

      cleanupSVGs.push(() => select(svg).selectAll("*").remove());
    }

    return () => {
      cleanupSVGs.forEach((cleanup) => cleanup());
      hideTooltip(tooltipRef, setShowTooltip);
      setShowTooltip(false);
      setTooltipTreeNode(null);
      setTooltipSVGElement(null);
      setTooltipPositionKind(PositionKind.Hover);
    };
  }, [
    tree,
    lastRequest,
    setShowTooltip,
    setTooltipTreeNode,
    setTooltipSVGElement,
    setTooltipPositionKind,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout,
    svgRefs,
  ]);

  return (
    <div className="tree-container">
      {svgRefs.map((ref, index) => (
        <svg key={index} className="tree" ref={ref} />
      ))}
      <TreeTooltip
        treeKind={TreeKind.Descendants}
        setSelectedLang={setSelectedLang}
        setSelectedItem={setSelectedItem}
        selectedDescLangs={selectedDescLangs}
        setTree={setTree}
        setSelectedTreeKind={setSelectedTreeKind}
        showTooltip={showTooltip}
        setShowTooltip={setShowTooltip}
        treeNode={tooltipTreeNode as HierarchyPointNode<Etymology | InterLangDescendants> | null}
        svgElement={tooltipSVGElement}
        positionKind={tooltipPositionKind}
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
  setShowTooltip: (show: boolean) => void,
  setTooltipTreeNode: (
    node: HierarchyPointNode<InterLangDescendants> | null
  ) => void,
  setTooltipSVGElement: (svg: SVGElement | null) => void,
  setTooltipPositionKind: (positionKind: PositionKind) => void,
  tooltipRef: React.RefObject<HTMLDivElement>,
  tooltipShowTimeout: React.RefObject<number | null>,
  tooltipHideTimeout: React.RefObject<number | null>
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

  const svg = configureSVG(svgElement, viewBox, width, height);

  renderTreeLinks(svg, pointRoot, curveStepBefore, {
    x: (d) => d.y,
    y: (d) => d.x,
  });

  const { node, nodeBackground } = renderTreeNodes(
    svg,
    pointRoot,
    treeRootItem,
    (n) => [n.y, n.x]
  );

  setTooltipListeners(
    node,
    setShowTooltip,
    setTooltipTreeNode,
    setTooltipSVGElement,
    setTooltipPositionKind,
    tooltipRef,
    tooltipShowTimeout,
    tooltipHideTimeout
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
