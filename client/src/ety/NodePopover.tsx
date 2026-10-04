import "./popover.css";
import { Item, TreeKind, term } from "../search/types";
import { EtySource, sourcesOf } from "./model";
import { langColor } from "./langColor";
import { etyModeRep, etyPrep } from "./etyMode";
import { BASIS_NAME, LangPeriod, PeriodEvidence, formatLocator, periodOf } from "./periods";
import {
  popover,
  popoverEnter,
  hoverLeave,
  close,
  isFinePointer,
} from "./popover";
import { forest, fetchItem, selectedDescLangs } from "../state";
import { useGoTo } from "../search/navigate";
import {
  For,
  JSX,
  Show,
  createEffect,
  createResource,
  onCleanup,
  onMount,
} from "solid-js";

const supportsAnchor =
  typeof CSS !== "undefined" && CSS.supports("position-anchor: --x");

export default function NodePopover() {
  let el!: HTMLDivElement;
  let lastAnchor: HTMLElement | null = null;
  const goTo = useGoTo();

  const show = () => {
    if (typeof el.showPopover === "function") {
      if (!el.matches(":popover-open")) el.showPopover();
    } else el.dataset.open = "";
  };
  const hide = () => {
    if (typeof el.hidePopover === "function") {
      if (el.matches(":popover-open")) el.hidePopover();
    } else delete el.dataset.open;
  };

  // Fallback placement for browsers without CSS anchor positioning.
  const placeManually = (anchor: HTMLElement) => {
    const a = anchor.getBoundingClientRect();
    const p = el.getBoundingClientRect();
    const margin = 6;
    let top = a.top - p.height - margin;
    if (top < margin) top = a.bottom + margin;
    let left = a.left;
    if (left + p.width > window.innerWidth - margin)
      left = Math.max(margin, window.innerWidth - margin - p.width);
    el.style.top = `${top}px`;
    el.style.left = `${left}px`;
  };

  createEffect(() => {
    const state = popover();
    if (lastAnchor) {
      lastAnchor.style.removeProperty("anchor-name");
      lastAnchor.classList.remove("popover-anchor");
      lastAnchor = null;
    }
    if (!state) {
      hide();
      return;
    }
    const sheet = !isFinePointer();
    el.dataset.mode = sheet ? "sheet" : "float";
    if (state.pinned) el.dataset.pinned = "";
    else delete el.dataset.pinned;
    if (!sheet) {
      lastAnchor = state.anchor;
      lastAnchor.classList.add("popover-anchor");
      if (supportsAnchor) lastAnchor.style.setProperty("anchor-name", "--wety-node");
    }
    show();
    if (!sheet && !supportsAnchor) {
      requestAnimationFrame(() => placeManually(state.anchor));
    }
  });

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && popover()) close();
    };
    const onPointerDown = (e: PointerEvent) => {
      const state = popover();
      if (!state?.pinned) return;
      const t = e.target as Node;
      if (el.contains(t) || state.anchor.contains(t)) return;
      close();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointerDown);
    onCleanup(() => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointerDown);
    });
  });

  const go = (kind: "descendants" | "etymology" | "cognates", item: Item) => {
    close();
    const treeKind = {
      etymology: TreeKind.Etymology,
      descendants: TreeKind.Descendants,
      cognates: TreeKind.Cognates,
    }[kind];
    const descLangs = selectedDescLangs();
    goTo(treeKind, item, descLangs.length ? descLangs : [item.lang]);
  };

  return (
    <div
      ref={el}
      class="node-popover"
      popover="manual"
      role="dialog"
      aria-label="word details"
      on:pointerenter={popoverEnter}
      on:pointerleave={hoverLeave}
    >
      <Show when={popover()}>
        {(state) => {
          const node = () => state().node;
          const item = () => node().item;
          const [full] = createResource(
            () => item(),
            (it) => (it.gloss === undefined ? fetchItem(it.id) : Promise.resolve(it))
          );
          const glosses = () => full()?.gloss ?? null;
          const posList = () => full()?.pos ?? item().pos ?? [];
          return (
            <div class="popover-body">
              <Show when={state().pinned}>
                <button class="popover-close" aria-label="close" onClick={close}>
                  ×
                </button>
              </Show>
              <div class="popover-lang" style={{ color: langColor(node().langDistance) }}>
                {item().lang.name}
              </div>
              <div class="popover-term">
                <span class="term">{term(item())}</span>
                <Show when={item().romanization}>
                  <span class="romanization"> {item().romanization}</span>
                </Show>
                <Show when={item().imputed}>
                  <span class="imputed" title="No wiktionary entry; inferred from other entries">
                    {" "}imputed
                  </span>
                </Show>
              </div>
              <Show when={full.loading}>
                <div class="popover-muted">…</div>
              </Show>
              <Show when={glosses()}>
                {(g) => (
                  <ol class="popover-glosses">
                    <For each={g()}>
                      {(gloss, i) => (
                        <li>
                          <Show when={posList()[i()]}>
                            <span class="pos">{posList()[i()]}</span>{" "}
                          </Show>
                          {gloss}
                        </li>
                      )}
                    </For>
                  </ol>
                )}
              </Show>
              <Show when={periodOf(item().lang.id)}>
                {(p) => <PeriodSection period={p()} />}
              </Show>
              <Show when={node().etyMode && sourcesOf(forest()!, node()).length}>
                <div class="popover-ety">{etyLine(node().etyMode!, sourcesOf(forest()!, node()))}</div>
              </Show>
              <Show when={forest()?.kind !== TreeKind.Etymology && node().descendantCount > 0}>
                <div class="popover-muted">
                  {node().descendantCount} descendant{node().descendantCount === 1 ? "" : "s"} shown
                  <Show when={node().langCounts.length > 1}>
                    {" "}in {node().langCounts.length} languages
                  </Show>
                </div>
              </Show>
              <div class="popover-actions">
                <button onClick={() => go("descendants", item())}>Descendants</button>
                <button onClick={() => go("cognates", item())}>Cognates</button>
                <button onClick={() => go("etymology", item())}>Etymology</button>
                <Show when={item().url}>
                  <a href={item().url!} target="_blank" rel="noopener noreferrer">
                    Wiktionary ↗
                  </a>
                </Show>
              </div>
            </div>
          );
        }}
      </Show>
    </div>
  );
}

// "borrowed from Latin status + -ālis"
export function etyLine(mode: string, sources: EtySource[]): JSX.Element {
  const parts: JSX.Element[] = [];
  let prevLang = -1;
  sources.forEach((s, i) => {
    if (i > 0) parts.push(<span class="ety-plus"> + </span>);
    if (s.item.lang.id !== prevLang) {
      parts.push(
        <span class="ety-lang" style={{ color: langColor(s.langDistance) }}>
          {s.item.lang.name}{" "}
        </span>
      );
      prevLang = s.item.lang.id;
    }
    parts.push(<span class="ety-term">{term(s.item)}</span>);
  });
  return (
    <>
      <span class="ety-mode">{etyModeRep(mode)}</span>
      <span class="ety-prep">{etyPrep(mode)}</span>
      {parts}
    </>
  );
}

// "Spoken c. 450–1150 CE · conventional period", then each citation and any
// other sources' periods, kept separate.
function PeriodSection(props: { period: LangPeriod }) {
  const cite = (e: PeriodEvidence) => (
    <>
      <Show when={e.url} fallback={<span class="period-source">{e.source}</span>}>
        <a class="period-source" href={e.url!} target="_blank" rel="noopener noreferrer">
          {e.source}
        </a>
      </Show>
      <Show when={formatLocator(e.locator)}>{(l) => <>, {l()}</>}</Show>
      <Show when={e.quote}>
        <q class="period-quote">{e.quote}</q>
      </Show>
    </>
  );
  return (
    <div class="popover-period">
      <div>
        <span class="period-head">{props.period.lang.name} spoken</span>{" "}
        <span class="period-label">{props.period.label}</span>{" "}
        <span class="period-basis">· {BASIS_NAME[props.period.basis]}</span>
      </div>
      <ul class="period-cites">
        <For each={props.period.evidence}>{(e) => <li title={e.citation ?? undefined}>{cite(e)}</li>}</For>
      </ul>
      <Show when={props.period.span}>
        {(span) => (
          <>
            <div class="popover-muted">
              <Show when={!span().partial} fallback={<>Dated varieties (not all are dated):</>}>
                With its varieties: <span class="period-label">{span().label}</span>
              </Show>
            </div>
            <ul class="period-cites">
              <For each={span().components.filter((c) => !c.own)}>
                {(c) => (
                  <li>
                    {c.name} <span class="period-label">{c.label}</span> ·{" "}
                    {BASIS_NAME[c.basis]}
                  </li>
                )}
              </For>
            </ul>
          </>
        )}
      </Show>
      <Show when={props.period.alternatives.length}>
        <div class="popover-muted">Other sources:</div>
        <ul class="period-cites">
          <For each={props.period.alternatives}>
            {(a) => (
              <li title={a.citation ?? undefined}>
                <span class="period-label">{a.label}</span> · {cite(a)}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
