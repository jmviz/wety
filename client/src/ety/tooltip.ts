import { Setter } from "solid-js";

export enum PositionKind {
  Hover,
  Fixed,
}

export function etyModeRep(etyMode: string): string {
  switch (etyMode) {
    case "undefined derivation":
    case "mention":
      return "derived";
    case "morphological derivation":
      return "derivation";
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
    case "surface analysis":
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

  if (elementRect.top >= tooltipRect.height) {
    tooltip.style.top =
      elementRect.top + window.scrollY - tooltipRect.height + "px";
  } else {
    tooltip.style.top = elementRect.bottom + window.scrollY + "px";
  }

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

export interface TooltipRefs {
  el: HTMLDivElement | undefined;
  showTimeout: number | null;
  hideTimeout: number | null;
  justDismissed: boolean;
}

export function hideTooltip(
  refs: TooltipRefs,
  setShow: Setter<boolean>
) {
  setShow(false);
  if (!refs.el) return;
  refs.el.style.opacity = "0";
  refs.el.style.zIndex = "-9000";
  refs.el.style.top = "0px";
  refs.el.style.left = "0px";
}
