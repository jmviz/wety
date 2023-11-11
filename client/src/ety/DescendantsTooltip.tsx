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
import { interLangDescendants } from "./DescendantsTree";
import {
  PositionKind,
  etyModeRep,
  etyPrep,
  hideTooltip,
  positionTooltip,
} from "./tooltip";

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

interface DescendantsTooltipProps {
  setSelectedLang: (lang: Lang | null) => void;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedTreeKind: (treeKind: TreeKind) => void;
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  showTooltip: boolean;
  setShowTooltip: (show: boolean) => void;
  treeNode: HierarchyPointNode<InterLangDescendants> | null;
  svgElement: SVGElement | null;
  positionKind: PositionKind;
  divRef: RefObject<HTMLDivElement>;
  showTimeout: MutableRefObject<number | null>;
  hideTimeout: MutableRefObject<number | null>;
  lastRequest: TreeRequest | null;
  setLastRequest: (request: TreeRequest | null) => void;
}

export default function DescendantsTooltip({
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
}: DescendantsTooltipProps) {
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

  const getEtymology = useMemo(
    () =>
      debounce(async (item: Item) => {
        const request = new TreeRequest(
          item.lang,
          item,
          selectedDescLangs,
          TreeKind.Etymology
        );

        if (lastRequest && request.equals(lastRequest)) {
          return;
        }

        try {
          const response = await fetch(request.url());
          const tree = (await response.json()) as Etymology;
          console.log(tree);
          setLastRequest(request);
          setSelectedLang(item.lang);
          setSelectedItem(item);
          setTree(tree);
          setSelectedTreeKind(TreeKind.Etymology);
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [
      selectedDescLangs,
      lastRequest,
      setLastRequest,
      setSelectedLang,
      setSelectedItem,
      setTree,
      setSelectedTreeKind,
    ]
  );

  if (treeNode === null || svgElement === null) {
    return <div ref={divRef} />;
  }

  const item = treeNode.data.item;
  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];

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
      {etyLine(treeNode)}
      <Stack
        direction={{ xs: "column", sm: "row" }}
        justifyContent="flex-start"
        alignItems="flex-start"
      >
        <Button size="small" onClick={() => getDescendants(item)}>
          Descendants
        </Button>
        <Button size="small" onClick={() => getEtymology(item)}>
          Etymology
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

function etyLine(
  treeNode: HierarchyPointNode<InterLangDescendants>
): JSX.Element | null {
  if (!treeNode.parent || !treeNode.data.etyMode) {
    return null;
  }

  let parts = [];
  let prev_lang = "";
  let ancestor = treeNode.data.parent;
  while (ancestor && ancestor.etyMode) {
    if (parts.length !== 0) {
      parts.push(<span key={parts.length}>{", "}</span>);
    }
    parts.push(
      <span key={parts.length} className="ety-mode">
        {etyModeRep(ancestor.etyMode)}
      </span>
    );
    parts.push(
      <span key={parts.length} className="ety-prep">
        {etyPrep(ancestor.etyMode)}
      </span>
    );
    const parents: EtyParent[] = ancestor.otherParents
      .sort((a, b) => a.etyOrder - b.etyOrder)
      .map((parent) => ({
        lang: parent.item.lang.name,
        term: term(parent.item),
        langDistance: parent.langDistance,
      }));
    if (ancestor.etyOrder !== null) {
      parents.splice(ancestor.etyOrder, 0, {
        lang: ancestor.item.lang.name,
        term: term(ancestor.item),
        langDistance: ancestor.langDistance,
      });
    }
    for (const parent of parents) {
      if (parent.lang !== prev_lang) {
        parts.push(
          <span
            key={parts.length}
            className="ety-lang"
            style={{ color: langColor(parent.langDistance) }}
          >
            {parent.lang}{" "}
          </span>
        );
        prev_lang = parent.lang;
      }
      parts.push(
        <span key={parts.length} className="ety-term">
          {parent.term}
        </span>
      );
      if (parent !== parents[parents.length - 1]) {
        parts.push(<span key={parts.length}>{" + "}</span>);
      }
    }
    ancestor = ancestor.ancestralLine;
  }
  return <div className="ety-line">{parts}</div>;
}

export function setDescendantsTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<InterLangDescendants>,
    SVGGElement,
    undefined
  >,
  setShowTooltip: (show: boolean) => void,
  setTooltipTreeNode: (
    node: HierarchyPointNode<InterLangDescendants> | null
  ) => void,
  setTooltipSVGElement: (element: SVGElement | null) => void,
  setTooltipPositionKind: (kind: PositionKind) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // for non-mouse, show tooltip on pointerup
  node.on(
    "pointerup",
    function (
      event: PointerEvent,
      d: BoundedHierarchyPointNode<InterLangDescendants>
    ) {
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
    function (
      event: PointerEvent,
      d: BoundedHierarchyPointNode<InterLangDescendants>
    ) {
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
