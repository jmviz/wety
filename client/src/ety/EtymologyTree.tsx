import "./Tree.css";
import {
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeRequest,
} from "../search/types";
import { xMeanClusterLayout } from "./treeCluster";
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

import { curveStepAfter } from "d3-shape";
import { hierarchy, HierarchyPointNode } from "d3-hierarchy";
import { select } from "d3-selection";
import { useRef, useEffect, useState } from "react";

interface EtymologyTreeProps {
  setSelectedLang: (lang: Lang | null) => void;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedTreeKind: (treeKind: TreeKind) => void;
  tree: Etymology | InterLangDescendants[] | null;
  setTree: (tree: Etymology | InterLangDescendants[] | null) => void;
  lastRequest: TreeRequest | null;
  setLastRequest: (request: TreeRequest | null) => void;
}

export default function EtymologyTree({
  setSelectedLang,
  setSelectedItem,
  selectedDescLangs,
  setSelectedTreeKind,
  tree,
  setTree,
  lastRequest,
  setLastRequest,
}: EtymologyTreeProps) {
  const [showTooltip, setShowTooltip] = useState(false);
  const [tooltipTreeNode, setTooltipTreeNode] =
    useState<HierarchyPointNode<Etymology> | null>(null);
  const [tooltipSVGElement, setTooltipSVGElement] = useState<SVGElement | null>(
    null
  );
  const [tooltipPositionKind, setTooltipPositionKind] = useState<PositionKind>(
    PositionKind.Hover
  );
  const svgRef = useRef<SVGSVGElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  useEffect(() => {
    const svg = svgRef.current;

    if (svg === null || tree === null || lastRequest === null) {
      return;
    }

    etymologyTreeSVG(
      svg,
      tree as Etymology,
      lastRequest.item,
      setShowTooltip,
      setTooltipTreeNode,
      setTooltipSVGElement,
      setTooltipPositionKind,
      tooltipRef,
      tooltipShowTimeout,
      tooltipHideTimeout
    );

    return () => {
      // clear the previous svg
      select(svg).selectAll("*").remove();
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
  ]);

  return (
    <div className="tree-container">
      <svg className="tree" ref={svgRef} />
      <TreeTooltip
        treeKind={TreeKind.Etymology}
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

function etymologyTreeSVG(
  svgElement: SVGSVGElement,
  tree: Etymology,
  treeRootItem: Item,
  setShowTooltip: (show: boolean) => void,
  setTooltipTreeNode: (node: HierarchyPointNode<Etymology> | null) => void,
  setTooltipSVGElement: (svg: SVGElement | null) => void,
  setTooltipPositionKind: (positionKind: PositionKind) => void,
  tooltipRef: React.RefObject<HTMLDivElement>,
  tooltipShowTimeout: React.MutableRefObject<number | null>,
  tooltipHideTimeout: React.MutableRefObject<number | null>
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
    (n) => [n.x, n.y]
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

  addSVGTextBackgrounds(node, nodeBackground, (d) => [d.node.x, d.node.y]);
}
