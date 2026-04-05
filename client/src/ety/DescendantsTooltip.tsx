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
  TooltipRefs,
} from "./tooltip";
import {
  setSelectedLang,
  setSelectedItem,
  selectedDescLangs,
  setSelectedTreeKind,
  setTree,
  lastRequest,
  setLastRequest,
  debounce,
} from "../state";

import { HierarchyPointNode, Selection } from "d3";
import {
  Accessor,
  Setter,
  Show,
  For,
  createEffect,
  onMount,
  onCleanup,
  JSX,
} from "solid-js";

interface DescendantsTooltipProps {
  showTooltip: Accessor<boolean>;
  setShowTooltip: Setter<boolean>;
  treeNode: Accessor<HierarchyPointNode<InterLangDescendants> | null>;
  svgElement: Accessor<SVGElement | null>;
  positionKind: Accessor<PositionKind>;
  tooltipRefs: TooltipRefs;
}

export default function DescendantsTooltip(props: DescendantsTooltipProps) {
  onMount(() => {
    setTimeout(() => {
      const tooltip = props.tooltipRefs.el;
      if (!tooltip) return;

      const handleMouseEnter = (event: PointerEvent) => {
        if (event.pointerType === "mouse") {
          props.setShowTooltip(true);
          window.clearTimeout(props.tooltipRefs.hideTimeout ?? undefined);
        }
      };

      const handleMouseLeave = (event: PointerEvent) => {
        if (event.pointerType === "mouse") {
          window.clearTimeout(props.tooltipRefs.showTimeout ?? undefined);
          props.tooltipRefs.hideTimeout = window.setTimeout(
            () => hideTooltip(props.tooltipRefs, props.setShowTooltip),
            100
          );
        }
      };

      tooltip.addEventListener("pointerenter", handleMouseEnter);
      tooltip.addEventListener("pointerleave", handleMouseLeave);

      onCleanup(() => {
        tooltip.removeEventListener("pointerenter", handleMouseEnter);
        tooltip.removeEventListener("pointerleave", handleMouseLeave);
      });
    }, 0);
  });

  createEffect(() => {
    const tooltip = props.tooltipRefs.el;
    const node = props.treeNode();
    const svg = props.svgElement();
    if (!tooltip || !node || !svg || !props.showTooltip()) return;
    positionTooltip(svg, tooltip, props.positionKind());
    tooltip.style.zIndex = "9000";
    tooltip.style.opacity = "1";
  });

  const getDescendants = debounce(async (item: Item) => {
    const request = new TreeRequest(
      item.lang,
      item,
      selectedDescLangs(),
      TreeKind.Descendants
    );

    const current = lastRequest();
    if (current && request.equals(current)) return;

    try {
      const response = await fetch(request.url());
      const data = (await response.json()) as Descendants;
      console.log(data);
      setLastRequest(request);
      setSelectedLang(item.lang);
      setSelectedItem(item);
      setTree([interLangDescendants(data)]);
      setSelectedTreeKind(TreeKind.Descendants);
    } catch (error) {
      console.log(error);
    }
  }, 0);

  const getEtymology = debounce(async (item: Item) => {
    const request = new TreeRequest(
      item.lang,
      item,
      selectedDescLangs(),
      TreeKind.Etymology
    );

    const current = lastRequest();
    if (current && request.equals(current)) return;

    try {
      const response = await fetch(request.url());
      const data = (await response.json()) as Etymology;
      console.log(data);
      setLastRequest(request);
      setSelectedLang(item.lang);
      setSelectedItem(item);
      setTree(data);
      setSelectedTreeKind(TreeKind.Etymology);
    } catch (error) {
      console.log(error);
    }
  }, 0);

  return (
    <div ref={(el) => (props.tooltipRefs.el = el)}>
      <Show when={props.treeNode() && props.svgElement()}>
        {(_) => {
          const node = props.treeNode()!;
          const item = node.data.item;
          const posList = item.pos ?? [];
          const glossList = item.gloss ?? [];

          return (
            <div class="tooltip">
              <Show when={props.positionKind() === PositionKind.Fixed}>
                <button
                  class="close-button"
                  onClick={() =>
                    hideTooltip(props.tooltipRefs, props.setShowTooltip)
                  }
                >
                  x
                </button>
              </Show>
              <p
                class="lang"
                style={{ color: langColor(node.data.langDistance) }}
              >
                {item.lang.name}
              </p>
              <p>
                <span class="term">{term(item)}</span>
                <Show when={item.romanization}>
                  <span class="romanization"> ({item.romanization})</span>
                </Show>
              </p>
              <Show when={item.imputed}>
                <div class="pos-line">
                  <span class="imputed">(imputed)</span>
                </div>
              </Show>
              <Show
                when={
                  item.pos &&
                  item.gloss &&
                  item.pos.length === item.gloss.length
                }
              >
                <div>
                  <For each={posList}>
                    {(pos, i) => (
                      <div class="pos-line">
                        <span class="pos">{pos}</span>:{" "}
                        <span class="gloss">{glossList[i()]}</span>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              {etyLine(node)}
              <div class="tooltip-actions">
                <button
                  class="tooltip-btn"
                  onClick={() => getDescendants(item)}
                >
                  Descendants
                </button>
                <button
                  class="tooltip-btn"
                  onClick={() => getEtymology(item)}
                >
                  Etymology
                </button>
              </div>
              <Show when={item.url}>
                <a
                  href={item.url!}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="wiktionary-link"
                >
                  Wiktionary
                </a>
              </Show>
            </div>
          );
        }}
      </Show>
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

  const parts: JSX.Element[] = [];
  let prev_lang = "";
  let ancestor = treeNode.data.parent;
  while (ancestor && ancestor.etyMode) {
    if (parts.length !== 0) {
      parts.push(<span>{", "}</span>);
    }
    parts.push(
      <span class="ety-mode">{etyModeRep(ancestor.etyMode)}</span>
    );
    parts.push(
      <span class="ety-prep">{etyPrep(ancestor.etyMode)}</span>
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
            class="ety-lang"
            style={{ color: langColor(parent.langDistance) }}
          >
            {parent.lang}{" "}
          </span>
        );
        prev_lang = parent.lang;
      }
      parts.push(<span class="ety-term">{parent.term}</span>);
      if (parent !== parents[parents.length - 1]) {
        parts.push(<span>{" + "}</span>);
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
  setShowTooltip: Setter<boolean>,
  setTooltipTreeNode: Setter<
    HierarchyPointNode<InterLangDescendants> | null
  >,
  setTooltipSVGElement: Setter<SVGElement | null>,
  setTooltipPositionKind: Setter<PositionKind>,
  tooltipRefs: TooltipRefs
) {
  node.on(
    "pointerup",
    function (
      event: PointerEvent,
      d: BoundedHierarchyPointNode<InterLangDescendants>
    ) {
      if (event.pointerType !== "mouse") {
        setShowTooltip(true);
        setTooltipTreeNode(() => d.node);
        setTooltipSVGElement(this as unknown as SVGElement);
        setTooltipPositionKind(PositionKind.Fixed);
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
        const el = this as unknown as SVGElement;
        window.clearTimeout(tooltipRefs.hideTimeout ?? undefined);
        tooltipRefs.showTimeout = window.setTimeout(() => {
          setShowTooltip(true);
          setTooltipTreeNode(() => d.node);
          setTooltipSVGElement(el);
          setTooltipPositionKind(PositionKind.Hover);
        }, 100);
      }
    }
  );

  node.on("pointerleave", (event: PointerEvent) => {
    if (event.pointerType === "mouse") {
      window.clearTimeout(tooltipRefs.showTimeout ?? undefined);
      tooltipRefs.hideTimeout = window.setTimeout(
        () => hideTooltip(tooltipRefs, setShowTooltip),
        100
      );
    }
  });
}
