import { Item, term } from "../search/types";
import { TreeNode, isUncertain, outlineSources } from "./model";
import { etyModeShort, etyPrep } from "./etyMode";
import { showGlosses } from "../state";
import { BASIS_NAME, periodOf } from "./periods";
import { LangName, shortLangsInTree } from "./langName";

import { For, Show } from "solid-js";

interface NodeLabelProps {
  node: TreeNode;
  // Show the derivation mode inline (outline rows); the chart shows it on the
  // connector instead.
  inlineMode?: boolean;
  // Outline rows take one Tab stop between them; arrows move within.
  roving?: boolean;
}

// The clickable part of a row: language, term, romanization. Everything else
// on the row (mode mark, gloss, badge) is outside the button so it doesn't
// bloat the hit target.
export default function NodeLabel(props: NodeLabelProps) {
  const item = () => props.node.item;
  // Parents to name inline: the ones not already visible as the row's tree
  // parent (all of them for a root, whose parent isn't in the tree).
  const inlineSources = () =>
    props.node.parent
      ? props.node.otherParents
          .slice()
          .sort((a, b) => a.etyOrder - b.etyOrder)
          .map((p) => ({ item: p.item, langDistance: p.langDistance, certainty: p.certainty }))
      : outlineSources(props.node);
  const gloss = () => item().gloss?.[0];
  // Dates belong to languages: show them where the language changes.
  const datedHere = () => props.node.parent?.item.lang.id !== item().lang.id;
  // Inheritance is the unmarked case: say nothing unless there is a source
  // to name.
  const showMode = () =>
    props.inlineMode &&
    props.node.etyMode &&
    !(props.node.etyMode === "inherited" && props.node.parent && inlineSources().length === 0);

  return (
    <>
      <button
        type="button"
        class="label"
        data-key={props.node.key}
        tabIndex={props.roving ? -1 : undefined}
        aria-haspopup="dialog"
      >
        <LangName lang={item().lang} short={shortLangsInTree()} />{" "}
        <span class="term">
          {term(item())}
          <Show when={isUncertain(props.node)}>
            <span class="uncertain-mark" title={`${props.node.certainty} source`}>
              ?
            </span>
          </Show>
        </span>
        <Show when={item().romanization}>
          {" "}
          <span class="romanization">{item().romanization}</span>
        </Show>
      </button>
      <Show when={datedHere() && periodOf(item().lang.id)}>
        {(p) => (
          <span
            class="period"
            title={`${item().lang.name}: ${BASIS_NAME[p().basis]} (${p()
              .evidence.map((e) => e.source)
              .join("; ")})`}
          >
            {p().label}
            <Show when={p().revival?.length}>, being revived</Show>
          </span>
        )}
      </Show>
      <Show when={showMode()}>
        <span class="mode">
          <span class="mode-name">{etyModeShort(props.node.etyMode!)}</span>
          <For each={inlineSources()}>
            {(s, i) => (
              <>
                {i() === 0 && !props.node.parent
                  ? etyPrep(props.node.etyMode!)
                  : " + "}
                <Source item={s.item} certainty={s.certainty} lang={item().lang.id} />
              </>
            )}
          </For>
        </span>
      </Show>
      <Show when={props.inlineMode && props.node.otherEtys.length}>
        <span class="mode alts" title={standingTitle[props.node.standing ?? "preferred"]}>
          <For each={props.node.otherEtys}>
            {(e, i) => (
              <>
                {i() > 0 && " "}
                <span class="alt-or">{e.preferred ? "preferred" : "or"}</span>{" "}
                <Show when={e.kind !== "unknown"} fallback="origin unknown">
                  {e.mode && etyModeShort(e.mode)}
                  <Show when={e.kind === "language_only" && e.sourceLang}>
                    {(l) => (
                      <>
                        {etyPrep(e.mode ?? "derived")}
                        <span class="lang">{l().name}</span>
                        {l().certainty && "?"}, exact source uncertain
                      </>
                    )}
                  </Show>
                  <For each={e.parents}>
                    {(p, j) => (
                      <>
                        {j() === 0 ? etyPrep(e.mode ?? "derived") : " + "}
                        <Source item={p.item} certainty={p.certainty} lang={item().lang.id} />
                      </>
                    )}
                  </For>
                </Show>
              </>
            )}
          </For>
        </span>
      </Show>
      <Show when={showGlosses() && gloss()}>
        <span class="gloss">{gloss()}</span>
      </Show>
    </>
  );
}

const standingTitle = {
  preferred: "other etymologies given for this word, less favoured",
  unsettled: "sources give several etymologies and prefer none: the word is drawn under each",
  other:
    "drawn here for an etymology another is preferred to; its descendants are drawn under the preferred one",
};

// A source named on a row: its language when not the row's, its term, and
// "?" when the link is possible or doubtful.
function Source(props: { item: Item; certainty?: string; lang: number }) {
  return (
    <span class="mode-src">
      <Show when={props.item.lang.id !== props.lang}>
        <LangName lang={props.item.lang} short={shortLangsInTree()} />{" "}
      </Show>
      {term(props.item)}
      <Show when={props.certainty === "possible" || props.certainty === "doubtful"}>
        <span class="uncertain-mark" title={`${props.certainty} source`}>
          ?
        </span>
      </Show>
    </span>
  );
}
