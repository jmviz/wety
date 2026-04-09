import "./Tooltip.css";
import {
  Etymology,
  InterLangDescendants,
  Item,
  TreeKind,
  term,
} from "../search/types";
import { langColor } from "./tree";
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
import { HierarchyPointNode } from "d3";
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

interface TreeTooltipProps {
  treeKind: TreeKind;
  showTooltip: Accessor<boolean>;
  setShowTooltip: Setter<boolean>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  treeNode: Accessor<HierarchyPointNode<any> | null>;
  svgElement: Accessor<SVGElement | null>;
  positionKind: Accessor<PositionKind>;
  tooltipRefs: TooltipRefs;
}

export default function TreeTooltip(props: TreeTooltipProps) {
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

  const navigateToEtymology = debounce((item: Item) => {
    setSelectedLang(item.lang);
    setSelectedItem(item);
    navigate({ to: `/etymology/${item.id}`, search: {} });
  }, 0);

  return (
    <div ref={(el) => (props.tooltipRefs.el = el)}>
      <Show when={props.treeNode() && props.svgElement()}>
        {(_) => {
          const node = () => props.treeNode()!;
          const item = () => node().data.item;
          const posList = () => item().pos ?? [];
          const glossList = () => item().gloss ?? [];

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
              <Show when={props.treeKind === TreeKind.Etymology}>
                {etymologyEtyLine(node() as HierarchyPointNode<Etymology>)}
              </Show>
              <Show when={props.treeKind === TreeKind.Descendants}>
                {descendantsEtyLine(
                  node() as HierarchyPointNode<InterLangDescendants>
                )}
              </Show>
              <div class="tooltip-actions">
                <button
                  class="tooltip-btn"
                  onClick={() => navigateToDescendants(item())}
                >
                  Descendants
                </button>
                <Show when={props.treeKind === TreeKind.Descendants}>
                  <button
                    class="tooltip-btn"
                    onClick={() => navigateToEtymology(item())}
                  >
                    Etymology
                  </button>
                </Show>
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

function etymologyEtyLine(
  node: HierarchyPointNode<Etymology>
): JSX.Element | null {
  const etyMode = node.data.etyMode;
  if (!etyMode || !node.children) return null;

  const parents: EtyParent[] = node.children
    .sort((a, b) => a.data.etyOrder - b.data.etyOrder)
    .map((parentNode) => ({
      lang: parentNode.data.item.lang.name,
      term: term(parentNode.data.item),
      langDistance: parentNode.data.langDistance,
    }));

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

function descendantsEtyLine(
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
