import "./Tooltip.css";
import { Descendants, term } from "../search/responses";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import {
  PositionKind,
  etyModeRep,
  etyPrep,
  hideTooltip,
  positionTooltip,
} from "./tooltip";

import { HierarchyPointNode, Selection } from "d3";
import { MutableRefObject, RefObject, useEffect, useLayoutEffect } from "react";

export interface DescendantsTooltipState {
  itemNode: HierarchyPointNode<Descendants> | null;
  svgElement: SVGElement | null;
  positionKind: PositionKind;
}

interface DescendantsTooltipProps {
  state: DescendantsTooltipState;
  divRef: RefObject<HTMLDivElement>;
  showTimeout: MutableRefObject<number | null>;
  hideTimeout: MutableRefObject<number | null>;
}

export default function DescendantsTooltip({
  state: { itemNode, svgElement, positionKind },
  divRef,
  showTimeout,
  hideTimeout,
}: DescendantsTooltipProps) {
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
    positionTooltip(svgElement, tooltip, positionKind);
    tooltip.style.zIndex = "9000";
    tooltip.style.opacity = "1";
  });

  if (itemNode === null || svgElement === null) {
    return <div ref={divRef} />;
  }

  const item = itemNode.data.item;
  const parents: EtyParent[] = [];
  if (itemNode.parent && itemNode.data.parentEtyOrder) {
    if (itemNode.data.otherParents) {
      for (let i = 0; i < itemNode.data.otherParents.length; i++) {
        const otherParent = itemNode.data.otherParents[i];
        if (i === itemNode.data.parentEtyOrder) {
          parents.push({
            lang: itemNode.parent.data.item.lang,
            term: term(itemNode.parent.data.item),
            langDistance: itemNode.parent.data.langDistance,
          });
        }
        parents.push({
          lang: otherParent.item.lang,
          term: term(otherParent.item),
          langDistance: otherParent.langDistance,
        });
      }
    } else {
      parents.push({
        lang: itemNode.parent.data.item.lang,
        term: term(itemNode.parent.data.item),
        langDistance: itemNode.parent.data.langDistance,
      });
    }
  }

  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];
  const etyMode = itemNode.data.etyMode;

  return (
    <div className="tooltip" ref={divRef}>
      {positionKind === PositionKind.Fixed && (
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
        <div className="actions-container">
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

export function setDescendantsTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<Descendants>,
    SVGGElement,
    undefined
  >,
  setTooltipState: (state: DescendantsTooltipState) => void,
  tooltipRef: RefObject<HTMLDivElement>,
  tooltipShowTimeout: MutableRefObject<number | null>,
  tooltipHideTimeout: MutableRefObject<number | null>
) {
  // for non-mouse, show tooltip on pointerup
  node.on(
    "pointerup",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Descendants>) {
      if (event.pointerType !== "mouse") {
        setTooltipState({
          itemNode: d.node,
          svgElement: this,
          positionKind: PositionKind.Fixed,
        });
      }
    }
  );

  // for mouse, show tooltip on hover
  node.on(
    "pointerenter",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Descendants>) {
      if (event.pointerType === "mouse") {
        window.clearTimeout(tooltipHideTimeout.current ?? undefined);
        tooltipShowTimeout.current = window.setTimeout(
          () =>
            setTooltipState({
              itemNode: d.node,
              svgElement: this,
              positionKind: PositionKind.Hover,
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
