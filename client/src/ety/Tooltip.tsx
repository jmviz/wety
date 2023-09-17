import "./Tooltip.css";
import { ExpandedItem } from "../search/responses";
import { ExpandedItemNode, langColor } from "./Tree";

import { Selection, HierarchyPointNode } from "d3";
import { MutableRefObject, RefObject, useEffect } from "react";

interface TooltipProps {
  itemNode: HierarchyPointNode<ExpandedItem> | null;
  positionType: string;
  divRef: RefObject<HTMLDivElement>;
  showTimeout: MutableRefObject<number | null>;
  hideTimeout: MutableRefObject<number | null>;
}

export default function Tooltip({
  itemNode,
  positionType,
  divRef,
  showTimeout,
  hideTimeout,
}: TooltipProps) {
  useEffect(() => {
    console.log("tooltip effect");
    const tooltip = divRef.current;

    if (tooltip === null) {
      return;
    }

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

  console.log("tooltip render");

  if (itemNode === null) {
    return <div ref={divRef} />;
  }

  const item = itemNode.data.item;
  const parent = itemNode.parent
    ? {
        lang: itemNode.parent.data.item.lang,
        term: itemNode.parent.data.item.term,
        langDistance: itemNode.parent.data.langDistance,
      }
    : null;

  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];

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
        <span className="term">{item.term}</span>
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
      {item.etyMode && parent && (
        <div className="ety-line">
          <span className="ety-mode">{item.etyMode}</span>
          <span className="ety-prep">{etyPrep(item.etyMode)}</span>
          <span
            className="parent-lang"
            style={{ color: langColor(parent.langDistance) }}
          >
            {parent.lang}
          </span>
          <span className="parent-term">{parent.term}</span>
        </div>
      )}
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

function etyPrep(etyMode: string): string {
  switch (etyMode) {
    case "derived":
    case "undefined derivation":
    case "inherited":
    case "borrowed":
    case "back-formation":
      return "from";
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
      return "with";
    case "vṛddhi":
    case "vṛddhi-ya":
      return "derivative of";
    case "root":
      return "reflex of";
    case "mention":
      return "in";
    default:
      return "of";
  }
}

function positionHoverTooltip(
  element: SVGElement,
  tooltipRef: RefObject<HTMLDivElement>
) {
  if (tooltipRef.current === null) {
    return;
  }
  tooltipRef.current.style.position = "absolute";

  const tooltipRect = tooltipRef.current.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();

  // Position the tooltip above the element. If there is not enough space,
  // position it below the element.
  if (elementRect.top >= tooltipRect.height) {
    tooltipRef.current.style.top =
      elementRect.top + window.scrollY - tooltipRect.height + "px";
  } else {
    tooltipRef.current.style.top = elementRect.bottom + window.scrollY + "px";
  }

  // Align the tooltip with the left side of the element. If there is not
  // enough space, align it with the right side.
  if (elementRect.left + tooltipRect.width <= window.innerWidth) {
    tooltipRef.current.style.left = elementRect.left + window.scrollX + "px";
  } else {
    tooltipRef.current.style.left =
      elementRect.right + window.scrollX - tooltipRect.width + "px";
  }
}

function positionFixedTooltip(tooltipRef: RefObject<HTMLDivElement>) {
  if (tooltipRef.current === null) {
    return;
  }
  tooltipRef.current.style.position = "fixed";
  tooltipRef.current.style.top = "50%";
  tooltipRef.current.style.left = "50%";
  tooltipRef.current.style.transform = "translate(-50%, -50%)";
}

function positionTooltip(
  element: SVGElement,
  tooltipRef: RefObject<HTMLDivElement>,
  type: string
) {
  if (type === "hover") {
    positionHoverTooltip(element, tooltipRef);
  } else {
    positionFixedTooltip(tooltipRef);
  }
}

function showTooltip(
  element: SVGElement,
  tooltipRef: RefObject<HTMLDivElement>,
  type: string
) {
  hideTooltip(tooltipRef);
  positionTooltip(element, tooltipRef, type);
  if (tooltipRef.current === null) {
    return;
  }
  tooltipRef.current.style.zIndex = "9000";
  tooltipRef.current.style.opacity = "1";
}

function hideTooltip(tooltipRef: RefObject<HTMLDivElement>) {
  if (tooltipRef.current === null) {
    return;
  }
  tooltipRef.current.style.opacity = "0";
  tooltipRef.current.style.zIndex = "-9000";
  tooltipRef.current.style.top = "0px";
  tooltipRef.current.style.left = "0px";
}

export function setNodeTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    ExpandedItemNode,
    SVGGElement,
    undefined
  >,
  tooltipRef: RefObject<HTMLDivElement>,
  setTooltipItem: (item: HierarchyPointNode<ExpandedItem> | null) => void,
  setPositionType: (type: string) => void,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // for non-mouse, show tooltip on pointerup
  node.on("pointerup", function (event: PointerEvent, d: ExpandedItemNode) {
    if (event.pointerType !== "mouse") {
      setTooltipItem(d.node);
      setPositionType("fixed");
      showTooltip(this, tooltipRef, "fixed");
    }
  });

  // for mouse, show tooltip on hover
  node.on("pointerenter", function (event: PointerEvent, d: ExpandedItemNode) {
    if (event.pointerType === "mouse") {
      window.clearTimeout(tooltipHideTimeout.current ?? undefined);
      tooltipShowTimeout.current = window.setTimeout(() => {
        setTooltipItem(d.node);
        setPositionType("hover");
        showTooltip(this, tooltipRef, "hover");
      }, 100);
    }
  });

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
