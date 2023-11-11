import "./Tooltip.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeRequest,
  term,
} from "../search/types";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import { TreeKind } from "../search/types";
import {
  PositionKind,
  etyModeRep,
  etyPrep,
  hideTooltip,
  positionTooltip,
} from "./tooltip";
import { interLangDescendants } from "./DescendantsTree";

import { HierarchyPointNode, Selection } from "d3";
import {
  MutableRefObject,
  RefObject,
  useEffect,
  useLayoutEffect,
  useMemo,
} from "react";
import Button from "@mui/material/Button/Button";
import { debounce } from "@mui/material/utils";
import Stack from "@mui/material/Stack/Stack";

interface EtymologyTooltipProps {
  setSelectedLang: (lang: Lang | null) => void;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedTreeKind: (treeKind: TreeKind) => void;
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  showTooltip: boolean;
  setShowTooltip: (show: boolean) => void;
  treeNode: HierarchyPointNode<Etymology> | null;
  svgElement: SVGElement | null;
  positionKind: PositionKind;
  divRef: RefObject<HTMLDivElement>;
  showTimeout: MutableRefObject<number | null>;
  hideTimeout: MutableRefObject<number | null>;
  lastRequest: TreeRequest | null;
  setLastRequest: (request: TreeRequest | null) => void;
}

export default function EtymologyTooltip({
  setSelectedLang,
  setSelectedItem,
  selectedDescLangs,
  setSelectedTreeKind,
  setTree,
  showTooltip,
  setShowTooltip,
  treeNode,
  svgElement,
  positionKind,
  divRef,
  showTimeout,
  hideTimeout,
  lastRequest,
  setLastRequest,
}: EtymologyTooltipProps) {
  useEffect(() => {
    const tooltip = divRef.current;
    if (!tooltip) return;

    const handleMouseEnter = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        setShowTooltip(true);
        window.clearTimeout(hideTimeout.current ?? undefined);
      }
    };

    const handleMouseLeave = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        window.clearTimeout(showTimeout.current ?? undefined);
        hideTimeout.current = window.setTimeout(
          () => hideTooltip(divRef, setShowTooltip),
          100
        );
      }
    };

    tooltip.addEventListener("pointerenter", handleMouseEnter);
    tooltip.addEventListener("pointerleave", handleMouseLeave);

    return () => {
      tooltip.removeEventListener("pointerenter", handleMouseEnter);
      tooltip.removeEventListener("pointerleave", handleMouseLeave);
    };
  }, [divRef, setShowTooltip, showTimeout, hideTimeout]);

  useLayoutEffect(() => {
    const tooltip = divRef.current;
    if (!tooltip || !treeNode || !svgElement || !showTooltip) return;
    positionTooltip(svgElement, tooltip, positionKind);
    tooltip.style.zIndex = "9000";
    tooltip.style.opacity = "1";
  }, [divRef, treeNode, svgElement, showTooltip, positionKind]);

  const getDescendants = useMemo(
    () =>
      debounce(async (item: Item) => {
        const request = new TreeRequest(
          item.lang,
          item,
          selectedDescLangs,
          TreeKind.Descendants
        );

        if (lastRequest && request.equals(lastRequest)) {
          return;
        }

        try {
          const response = await fetch(request.url());
          const tree = (await response.json()) as Descendants;
          console.log(tree);
          setLastRequest(request);
          setSelectedLang(item.lang);
          setSelectedItem(item);
          setTree(interLangDescendants(tree));
          setSelectedTreeKind(TreeKind.Descendants);
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [
      setSelectedLang,
      setSelectedItem,
      selectedDescLangs,
      setTree,
      setSelectedTreeKind,
      lastRequest,
      setLastRequest,
    ]
  );

  if (treeNode === null || svgElement === null) {
    return <div ref={divRef} />;
  }

  const item = treeNode.data.item;
  // Confusingly, the "children" with respect to the tree structure and d3 api
  // are the parents with respect to the etymology.
  const parents: EtyParent[] | null = treeNode.children
    ? treeNode.children
        .sort((a, b) => a.data.etyOrder - b.data.etyOrder)
        .map((parentNode) => ({
          lang: parentNode.data.item.lang.name,
          term: term(parentNode.data.item),
          langDistance: parentNode.data.langDistance,
        }))
    : null;

  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];
  const etyMode = treeNode.data.etyMode;

  return (
    <div className="tooltip" ref={divRef}>
      {positionKind === PositionKind.Fixed && (
        <button
          className="close-button"
          onClick={() => hideTooltip(divRef, setShowTooltip)}
        >
          ✕
        </button>
      )}
      <p
        className="lang"
        style={{ color: langColor(treeNode.data.langDistance) }}
      >
        {item.lang.name}
      </p>
      <p>
        <span className="term">{term(item)}</span>
        {item.romanization && (
          <span className="romanization"> ({item.romanization})</span>
        )}
      </p>
      {item.imputed && (
        <div className="pos-line">
          <span className="imputed">(imputed)</span>
        </div>
      )}
      {item.pos && item.gloss && item.pos.length === item.gloss.length && (
        <div>
          {posList.map((pos, i) => (
            <div key={i} className="pos-line">
              <span className="pos">{pos}</span>:{" "}
              <span className="gloss">{glossList[i]}</span>
            </div>
          ))}
        </div>
      )}
      {etyMode && parents && etyLine(etyMode, parents)}
      <Stack
        direction={{ xs: "column", sm: "row" }}
        justifyContent="flex-start"
        alignItems="flex-start"
      >
        <Button size="small" onClick={() => getDescendants(item)}>
          Descendants
        </Button>
      </Stack>
      {item.url && (
        <a
          href={item.url}
          target="_blank"
          rel="noopener noreferrer"
          className="wiktionary-link"
        >
          Wiktionary
        </a>
      )}
    </div>
  );
}

interface EtyParent {
  lang: string;
  term: string;
  langDistance: number;
}

function etyLine(etyMode: string, parents: EtyParent[]): JSX.Element {
  let parts = [];
  for (let i = 0; i < parents.length; i++) {
    const parent = parents[i];
    if (i === 0 || parent.lang !== parents[i - 1].lang) {
      parts.push(
        <span
          key={i}
          className="ety-lang"
          style={{ color: langColor(parent.langDistance) }}
        >
          {parent.lang}{" "}
        </span>
      );
    }
    parts.push(
      <span key={i + parents.length} className="ety-term">
        {parent.term}
      </span>
    );
    if (i < parents.length - 1) {
      parts.push(<span key={i + 2 * parents.length}>{" + "}</span>);
    }
  }

  return (
    <div className="ety-line">
      <span className="ety-mode">{etyModeRep(etyMode)}</span>
      <span className="ety-prep">{etyPrep(etyMode)}</span>
      {parts}
    </div>
  );
}

export function setEtymologyTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<Etymology>,
    SVGGElement,
    undefined
  >,
  setShowTooltip: (show: boolean) => void,
  setTooltipTreeNode: (node: HierarchyPointNode<Etymology> | null) => void,
  setTooltipSVGElement: (element: SVGElement | null) => void,
  setTooltipPositionKind: (kind: PositionKind) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // for non-mouse, show tooltip on pointerup
  node.on(
    "pointerup",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Etymology>) {
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
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Etymology>) {
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
