import "./Tooltip.css";
import { Etymology, term } from "../search/responses";
import { ExpandedItemNode, langColor } from "./Tree";

import { HierarchyPointNode, Selection } from "d3";
import { MutableRefObject, RefObject, useEffect, useLayoutEffect } from "react";

export interface TooltipState {
  itemNode: HierarchyPointNode<Etymology> | null;
  svgElement: SVGElement | null;
  positionType: string;
}

interface TooltipProps {
  state: TooltipState;
  divRef: RefObject<HTMLDivElement>;
  showTimeout: MutableRefObject<number | null>;
  hideTimeout: MutableRefObject<number | null>;
}

export default function Tooltip({
  state: { itemNode, svgElement, positionType },
  divRef,
  showTimeout,
  hideTimeout,
}: TooltipProps) {
  useEffect(() => {
    const tooltip = divRef.current;
    if (!tooltip) return;

    const handleMouseEnter = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        window.clearTimeout(hideTimeout.current ?? undefined);
      }
    };

    const handleMouseLeave = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        window.clearTimeout(showTimeout.current ?? undefined);
        hideTimeout.current = window.setTimeout(() => hideTooltip(divRef), 100);
      }
    };

    tooltip.addEventListener("pointerenter", handleMouseEnter);
    tooltip.addEventListener("pointerleave", handleMouseLeave);

    return () => {
      tooltip.removeEventListener("pointerenter", handleMouseEnter);
      tooltip.removeEventListener("pointerleave", handleMouseLeave);
    };
  }, [divRef, showTimeout, hideTimeout]);

  useLayoutEffect(() => {
    const tooltip = divRef.current;
    if (!tooltip || !itemNode || !svgElement) return;
    positionTooltip(svgElement, tooltip, positionType);
    tooltip.style.zIndex = "9000";
    tooltip.style.opacity = "1";
  });

  if (itemNode === null || svgElement === null) {
    return <div ref={divRef} />;
  }

  const item = itemNode.data.item;
  // Confusingly, the "children" with respect to the tree structure and d3 api
  // are the parents with respect to the etymology.
  const parents: EtyParent[] | null = itemNode.children
    ? itemNode.children
        .sort((a, b) => a.data.etyOrder - b.data.etyOrder)
        .map((parentNode) => ({
          lang: parentNode.data.item.lang,
          term: term(parentNode.data.item),
          langDistance: parentNode.data.langDistance,
        }))
    : null;

  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];
  const etyMode = itemNode.data.etyMode
    ? etyModeRep(itemNode.data.etyMode)
    : null;

  return (
    <div className="tooltip" ref={divRef}>
      {positionType === "fixed" && (
        <button className="close-button" onClick={() => hideTooltip(divRef)}>
          ✕
        </button>
      )}
      <p
        className="lang"
        style={{ color: langColor(itemNode.data.langDistance) }}
      >
        {item.lang}
      </p>
      <p>
        <span className="term">{term(item)}</span>
        {item.romanization && (
          <span className="romanization">({item.romanization})</span>
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
      {item.url && (
        <div className="wiktionary-link-container">
          <a
            href={item.url}
            target="_blank"
            rel="noopener noreferrer"
            className="wiktionary-link"
          >
            Wiktionary
          </a>
        </div>
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
          className="parent-lang"
          style={{ color: langColor(parent.langDistance) }}
        >
          {parent.lang}{" "}
        </span>
      );
    }
    parts.push(
      <span key={i + parents.length} className="parent-term">
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

function etyModeRep(etyMode: string): string {
  switch (etyMode) {
    case "undefined derivation":
    case "mention":
      return "derived";
    default:
      return etyMode;
  }
}

function etyPrep(etyMode: string): string {
  switch (etyMode) {
    case "derived":
    case "inherited":
    case "borrowed":
    case "back-formation":
    case "undefined derivation":
    case "mention":
      return " from ";
    case "compound":
    case "univerbation":
    case "transfix":
    case "surface analysis":
    case "suffix":
    case "prefix":
    case "infix":
    case "confix":
    case "circumfix":
    case "blend":
    case "affix":
      return ": ";
    case "vṛddhi":
    case "vṛddhi-ya":
      return " derivative of ";
    case "root":
      return " reflex of ";
    default:
      return " of ";
  }
}

function positionHoverTooltip(element: SVGElement, tooltip: HTMLDivElement) {
  tooltip.style.position = "absolute";

  const tooltipRect = tooltip.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();

  // Position the tooltip above the element. If there is not enough space,
  // position it below the element.
  if (elementRect.top >= tooltipRect.height) {
    tooltip.style.top =
      elementRect.top + window.scrollY - tooltipRect.height + "px";
  } else {
    tooltip.style.top = elementRect.bottom + window.scrollY + "px";
  }

  // Align the tooltip with the left side of the element. If there is not
  // enough space, align it with the right side.
  if (elementRect.left + tooltipRect.width <= window.innerWidth) {
    tooltip.style.left = elementRect.left + window.scrollX + "px";
  } else {
    tooltip.style.left =
      elementRect.right + window.scrollX - tooltipRect.width + "px";
  }
}

function positionFixedTooltip(tooltip: HTMLDivElement) {
  tooltip.style.position = "fixed";
  tooltip.style.top = "50%";
  tooltip.style.left = "50%";
  tooltip.style.transform = "translate(-50%, -50%)";
}

function positionTooltip(
  element: SVGElement,
  tooltip: HTMLDivElement,
  type: string
) {
  if (type === "hover") {
    positionHoverTooltip(element, tooltip);
  } else {
    positionFixedTooltip(tooltip);
  }
}

export function hideTooltip(tooltip: RefObject<HTMLDivElement>) {
  if (tooltip.current === null) {
    return;
  }
  tooltip.current.style.opacity = "0";
  tooltip.current.style.zIndex = "-9000";
  tooltip.current.style.top = "0px";
  tooltip.current.style.left = "0px";
}

export function setNodeTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    ExpandedItemNode<Etymology>,
    SVGGElement,
    undefined
  >,
  setTooltipState: (state: TooltipState) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // for non-mouse, show tooltip on pointerup
  node.on(
    "pointerup",
    function (event: PointerEvent, d: ExpandedItemNode<Etymology>) {
      if (event.pointerType !== "mouse") {
        setTooltipState({
          itemNode: d.node,
          svgElement: this,
          positionType: "fixed",
        });
      }
    }
  );

  // for mouse, show tooltip on hover
  node.on(
    "pointerenter",
    function (event: PointerEvent, d: ExpandedItemNode<Etymology>) {
      if (event.pointerType === "mouse") {
        window.clearTimeout(tooltipHideTimeout.current ?? undefined);
        tooltipShowTimeout.current = window.setTimeout(
          () =>
            setTooltipState({
              itemNode: d.node,
              svgElement: this,
              positionType: "hover",
            }),
          100
        );
      }
    }
  );

  node.on("pointerleave", (event: PointerEvent) => {
    if (event.pointerType === "mouse") {
      window.clearTimeout(tooltipShowTimeout.current ?? undefined);
      tooltipHideTimeout.current = window.setTimeout(
        () => hideTooltip(tooltipRef),
        100
      );
    }
  });
}
