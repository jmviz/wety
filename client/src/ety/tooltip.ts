import { RefObject } from "react";

export enum PositionKind {
  Hover,
  Fixed,
}

export function etyModeRep(etyMode: string): string {
  switch (etyMode) {
    case "undefined derivation":
    case "mention":
      return "derived";
    case "transfix":
      return "transfixation";
    case "suffix":
      return "suffixation";
    case "prefix":
      return "prefixation";
    case "infix":
      return "infixation";
    case "confix":
      return "confixation";
    case "circumfix":
      return "circumfixation";
    case "affix":
      return "affixation";
    default:
      return etyMode;
  }
}

export function etyPrep(etyMode: string): string {
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
    case "surface analysis":
    case "blend":
    case "transfix":
    case "suffix":
    case "prefix":
    case "infix":
    case "confix":
    case "circumfix":
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

export function positionHoverTooltip(
  element: SVGElement,
  tooltip: HTMLDivElement
) {
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

export function positionFixedTooltip(tooltip: HTMLDivElement) {
  tooltip.style.position = "fixed";
  tooltip.style.top = "50%";
  tooltip.style.left = "50%";
  tooltip.style.transform = "translate(-50%, -50%)";
}

export function positionTooltip(
  element: SVGElement,
  tooltip: HTMLDivElement,
  kind: PositionKind
) {
  if (kind === PositionKind.Hover) {
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
