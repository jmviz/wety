import "./Tooltip.css";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
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
import {
  selectedLang,
  selectedItem,
  selectedDescLangs,
  selectedTreeKind,
  tree,
  lastRequest,
  debounce,
} from "../signals";

import { Signal } from "@preact/signals";
import { HierarchyPointNode, Selection } from "d3";
import { useEffect, useLayoutEffect, useMemo } from "preact/hooks";
import { ComponentChildren } from "preact";

interface DescendantsTooltipProps {
  showTooltip: Signal<boolean>;
  treeNode: Signal<HierarchyPointNode<InterLangDescendants> | null>;
  svgElement: Signal<SVGElement | null>;
  positionKind: Signal<PositionKind>;
  divRef: { current: HTMLDivElement | null };
  showTimeout: { current: number | null };
  hideTimeout: { current: number | null };
}

export default function DescendantsTooltip({
  showTooltip,
  treeNode,
  svgElement,
  positionKind,
  divRef,
  showTimeout,
  hideTimeout,
}: DescendantsTooltipProps) {
  useEffect(() => {
    const tooltip = divRef.current;
    if (!tooltip) return;

    const handleMouseEnter = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        showTooltip.value = true;
        window.clearTimeout(hideTimeout.current ?? undefined);
      }
    };

    const handleMouseLeave = (event: PointerEvent) => {
      if (event.pointerType === "mouse") {
        window.clearTimeout(showTimeout.current ?? undefined);
        hideTimeout.current = window.setTimeout(
          () => hideTooltip(divRef, showTooltip),
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
  }, [divRef, showTooltip, showTimeout, hideTimeout]);

  useLayoutEffect(() => {
    const tooltip = divRef.current;
    const node = treeNode.value;
    const svg = svgElement.value;
    if (!tooltip || !node || !svg || !showTooltip.value) return;
    positionTooltip(svg, tooltip, positionKind.value);
    tooltip.style.zIndex = "9000";
    tooltip.style.opacity = "1";
  }, [
    divRef,
    treeNode.value,
    svgElement.value,
    showTooltip.value,
    positionKind.value,
  ]);

  const getDescendants = useMemo(
    () =>
      debounce(async (item: Item) => {
        const request = new TreeRequest(
          item.lang,
          item,
          selectedDescLangs.value,
          TreeKind.Descendants
        );

        if (lastRequest.value && request.equals(lastRequest.value)) {
          return;
        }

        try {
          const response = await fetch(request.url());
          const data = (await response.json()) as Descendants;
          console.log(data);
          lastRequest.value = request;
          selectedLang.value = item.lang;
          selectedItem.value = item;
          tree.value = [interLangDescendants(data)];
          selectedTreeKind.value = TreeKind.Descendants;
        } catch (error) {
          console.log(error);
        }
      }, 0),
    []
  );

  const getEtymology = useMemo(
    () =>
      debounce(async (item: Item) => {
        const request = new TreeRequest(
          item.lang,
          item,
          selectedDescLangs.value,
          TreeKind.Etymology
        );

        if (lastRequest.value && request.equals(lastRequest.value)) {
          return;
        }

        try {
          const response = await fetch(request.url());
          const data = (await response.json()) as Etymology;
          console.log(data);
          lastRequest.value = request;
          selectedLang.value = item.lang;
          selectedItem.value = item;
          tree.value = data;
          selectedTreeKind.value = TreeKind.Etymology;
        } catch (error) {
          console.log(error);
        }
      }, 0),
    []
  );

  const node = treeNode.value;
  const svg = svgElement.value;

  if (node === null || svg === null) {
    return <div ref={divRef} />;
  }

  const item = node.data.item;
  const posList = item.pos ?? [];
  const glossList = item.gloss ?? [];

  return (
    <div class="tooltip" ref={divRef}>
      {positionKind.value === PositionKind.Fixed && (
        <button
          class="close-button"
          onClick={() => hideTooltip(divRef, showTooltip)}
        >
          x
        </button>
      )}
      <p
        class="lang"
        style={{ color: langColor(node.data.langDistance) }}
      >
        {item.lang.name}
      </p>
      <p>
        <span class="term">{term(item)}</span>
        {item.romanization && (
          <span class="romanization"> ({item.romanization})</span>
        )}
      </p>
      {item.imputed && (
        <div class="pos-line">
          <span class="imputed">(imputed)</span>
        </div>
      )}
      {item.pos && item.gloss && item.pos.length === item.gloss.length && (
        <div>
          {posList.map((pos, i) => (
            <div key={i} class="pos-line">
              <span class="pos">{pos}</span>:{" "}
              <span class="gloss">{glossList[i]}</span>
            </div>
          ))}
        </div>
      )}
      {etyLine(node)}
      <div class="tooltip-actions">
        <button class="tooltip-btn" onClick={() => getDescendants(item)}>
          Descendants
        </button>
        <button class="tooltip-btn" onClick={() => getEtymology(item)}>
          Etymology
        </button>
      </div>
      {item.url && (
        <a
          href={item.url}
          target="_blank"
          rel="noopener noreferrer"
          class="wiktionary-link"
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
): ComponentChildren | null {
  if (!treeNode.parent || !treeNode.data.etyMode) {
    return null;
  }

  let parts: ComponentChildren[] = [];
  let prev_lang = "";
  let ancestor = treeNode.data.parent;
  while (ancestor && ancestor.etyMode) {
    if (parts.length !== 0) {
      parts.push(<span key={parts.length}>{", "}</span>);
    }
    parts.push(
      <span key={parts.length} class="ety-mode">
        {etyModeRep(ancestor.etyMode)}
      </span>
    );
    parts.push(
      <span key={parts.length} class="ety-prep">
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
            class="ety-lang"
            style={{ color: langColor(parent.langDistance) }}
          >
            {parent.lang}{" "}
          </span>
        );
        prev_lang = parent.lang;
      }
      parts.push(
        <span key={parts.length} class="ety-term">
          {parent.term}
        </span>
      );
      if (parent !== parents[parents.length - 1]) {
        parts.push(<span key={parts.length}>{" + "}</span>);
      }
    }
    ancestor = ancestor.ancestralLine;
  }
  return <div class="ety-line">{parts}</div>;
}

export function setDescendantsTooltipListeners(
  node: Selection<
    SVGGElement | SVGTextElement,
    BoundedHierarchyPointNode<InterLangDescendants>,
    SVGGElement,
    undefined
  >,
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
  node.on(
    "pointerup",
    function (
      event: PointerEvent,
      d: BoundedHierarchyPointNode<InterLangDescendants>
    ) {
      if (event.pointerType !== "mouse") {
        showTooltip.value = true;
        tooltipTreeNode.value = d.node;
        tooltipSVGElement.value = this;
        tooltipPositionKind.value = PositionKind.Fixed;
      }
    }
  );

  node.on(
    "pointerenter",
    function (
      event: PointerEvent,
      d: BoundedHierarchyPointNode<InterLangDescendants>
    ) {
      if (event.pointerType === "mouse") {
        window.clearTimeout(tooltipHideTimeout.current ?? undefined);
        tooltipShowTimeout.current = window.setTimeout(() => {
          showTooltip.value = true;
          tooltipTreeNode.value = d.node;
          tooltipSVGElement.value = this;
          tooltipPositionKind.value = PositionKind.Hover;
        }, 100);
      }
    }
  );

  node.on("pointerleave", (event: PointerEvent) => {
    if (event.pointerType === "mouse") {
      window.clearTimeout(tooltipShowTimeout.current ?? undefined);
      tooltipHideTimeout.current = window.setTimeout(
        () => hideTooltip(tooltipRef, showTooltip as Signal<boolean>),
        100
      );
    }
  });
}
