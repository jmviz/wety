import "./Tooltip.css";
import {
  Etymology,
  Item,
  term,
} from "../search/types";
import { BoundedHierarchyPointNode, langColor } from "./tree";
import {
  PositionKind,
  etyModeRep,
  etyPrep,
  hideTooltip,
  positionTooltip,
  TooltipRefs,
} from "./tooltip";
import {
  selectedDescLangs,
  setSelectedLang,
  setSelectedItem,
  debounce,
} from "../state";

import { useNavigate } from "@tanstack/solid-router";
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

interface EtymologyTooltipProps {
  showTooltip: Accessor<boolean>;
  setShowTooltip: Setter<boolean>;
  treeNode: Accessor<HierarchyPointNode<Etymology> | null>;
  svgElement: Accessor<SVGElement | null>;
  positionKind: Accessor<PositionKind>;
  tooltipRefs: TooltipRefs;
}

export default function EtymologyTooltip(props: EtymologyTooltipProps) {
  onMount(() => {
    // Set up after the element is available
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

  const navigate = useNavigate();

  const navigateToDescendants = debounce((item: Item) => {
    const descLangs = selectedDescLangs();
    setSelectedLang(item.lang);
    setSelectedItem(item);
    navigate({
      to: `/descendants/${item.id}`,
      search: { distLang: item.lang.id, descLang: descLangs.map((l) => l.id) },
    });
  }, 0);

  return (
    <div ref={(el) => (props.tooltipRefs.el = el)}>
      <Show when={props.treeNode() && props.svgElement()}>
        {(_) => {
          const node = () => props.treeNode()!;
          const item = () => node().data.item;
          const posList = () => item().pos ?? [];
          const glossList = () => item().gloss ?? [];
          const etyMode = () => node().data.etyMode;
          const parents = (): EtyParent[] | null =>
            node().children
              ? node()
                  .children!.sort(
                    (a, b) => a.data.etyOrder - b.data.etyOrder
                  )
                  .map((parentNode) => ({
                    lang: parentNode.data.item.lang.name,
                    term: term(parentNode.data.item),
                    langDistance: parentNode.data.langDistance,
                  }))
              : null;

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
                style={{ color: langColor(node().data.langDistance) }}
              >
                {item().lang.name}
              </p>
              <p>
                <span class="term">{term(item())}</span>
                <Show when={item().romanization}>
                  <span class="romanization"> ({item().romanization})</span>
                </Show>
              </p>
              <Show when={item().imputed}>
                <div class="pos-line">
                  <span class="imputed">(imputed)</span>
                </div>
              </Show>
              <Show
                when={
                  item().pos &&
                  item().gloss &&
                  item().pos!.length === item().gloss!.length
                }
              >
                <div>
                  <For each={posList()}>
                    {(pos, i) => (
                      <div class="pos-line">
                        <span class="pos">{pos}</span>:{" "}
                        <span class="gloss">{glossList()[i()]}</span>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              <Show when={etyMode() && parents()}>
                {etyLine(etyMode()!, parents()!)}
              </Show>
              <div class="tooltip-actions">
                <button
                  class="tooltip-btn"
                  onClick={() => navigateToDescendants(item())}
                >
                  Descendants
                </button>
              </div>
              <Show when={item().url}>
                <a
                  href={item().url!}
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

function etyLine(etyMode: string, parents: EtyParent[]): JSX.Element {
  const parts: JSX.Element[] = [];
  for (let i = 0; i < parents.length; i++) {
    const parent = parents[i];
    if (i === 0 || parent.lang !== parents[i - 1].lang) {
      parts.push(
        <span
          class="ety-lang"
          style={{ color: langColor(parent.langDistance) }}
        >
          {parent.lang}{" "}
        </span>
      );
    }
    parts.push(<span class="ety-term">{parent.term}</span>);
    if (i < parents.length - 1) {
      parts.push(<span>{" + "}</span>);
    }
  }

  return (
    <div class="ety-line">
      <span class="ety-mode">{etyModeRep(etyMode)}</span>
      <span class="ety-prep">{etyPrep(etyMode)}</span>
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
  setShowTooltip: Setter<boolean>,
  setTooltipTreeNode: Setter<HierarchyPointNode<Etymology> | null>,
  setTooltipSVGElement: Setter<SVGElement | null>,
  setTooltipPositionKind: Setter<PositionKind>,
  tooltipRefs: TooltipRefs
) {
  node.on(
    "pointerup",
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Etymology>) {
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
    function (event: PointerEvent, d: BoundedHierarchyPointNode<Etymology>) {
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
