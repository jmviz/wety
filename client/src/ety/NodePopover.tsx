import "./popover.css";
import { Assertion, Credit, Hypothesis, Item, TreeKind, term } from "../search/types";
import { EtySource, TreeNode } from "./model";
import { LangName, shortLangsInDetails } from "./langName";
import { etyModeRep, etyModeShort, etyPrep } from "./etyMode";
import { BASIS_NAME, LangPeriod, METHOD_NAME, PeriodEvidence, formatLocator, periodOf } from "./periods";
import {
  popover,
  popoverEnter,
  hoverLeave,
  close,
  isFinePointer,
} from "./popover";
import { forest, fetchItem, fetchOrigin } from "../state";
import { useGoTo } from "../search/navigate";
import {
  For,
  JSX,
  Show,
  createEffect,
  createResource,
  createSignal,
  on,
  onCleanup,
  onMount,
} from "solid-js";

const supportsAnchor =
  typeof CSS !== "undefined" && CSS.supports("position-anchor: --x");

export default function NodePopover() {
  let el!: HTMLDivElement;
  let lastAnchor: HTMLElement | null = null;
  const goTo = useGoTo();
  const dismiss = () => {
    const anchor = popover()?.anchor;
    close();
    if (anchor?.isConnected) anchor.focus({ preventScroll: true });
  };

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

  // Fallback placement for browsers without CSS anchor positioning: beside
  // the word (right, else left), so its ancestors and descendants stay in
  // view; below it when neither side has room.
  const placeManually = (anchor: HTMLElement) => {
    const a = anchor.getBoundingClientRect();
    const p = el.getBoundingClientRect();
    const margin = 8;
    let top = a.top - 4;
    let left = a.right + margin;
    if (left + p.width > window.innerWidth - margin) left = a.left - margin - p.width;
    if (left < margin) {
      left = Math.max(margin, Math.min(a.left, window.innerWidth - margin - p.width));
      top = a.bottom + margin;
    }
    top = Math.max(margin, Math.min(top, window.innerHeight - p.height - margin));
    el.style.top = `${top}px`;
    el.style.left = `${left}px`;
  };

  createEffect(() => {
    const state = popover();
    const f = forest();
    if (state && f?.byKey.get(state.node.key) !== state.node) {
      close();
      hide();
      return;
    }
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
    if (state.pinned) {
      requestAnimationFrame(() => {
        if (popover() === state)
          el.querySelector<HTMLButtonElement>(".popover-close")?.focus({ preventScroll: true });
      });
    }
    if (!sheet && !supportsAnchor) {
      requestAnimationFrame(() => placeManually(state.anchor));
    }
  });

  onMount(() => {
    const place = () => {
      const state = popover();
      if (state && isFinePointer() && !supportsAnchor) placeManually(state.anchor);
    };
    const observer = new ResizeObserver(place);
    observer.observe(el);
    window.addEventListener("resize", place);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && popover()) dismiss();
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
      observer.disconnect();
      window.removeEventListener("resize", place);
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
    goTo(treeKind, item);
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
          // The word, or its language's dates and their evidence.
          const [langView, setLangView] = createSignal(false);
          createEffect(on(node, () => setLangView(false)));
          return (
            <div class="popover-body">
              <Show when={state().pinned}>
                <button class="popover-close" aria-label="close" onClick={dismiss}>
                  ×
                </button>
              </Show>
              <Show when={langView() && periodOf(item().lang.id)}>
                {(p) => (
                  <>
                    <button class="popover-back" onClick={() => setLangView(false)}>
                      ← {term(item())}
                    </button>
                    <PeriodSection period={p()} />
                  </>
                )}
              </Show>
              <Show when={!langView()}>
              <div class="popover-lang">
                <Show when={periodOf(item().lang.id)} fallback={item().lang.name}>
                  {(p) => (
                    <button
                      class="popover-lang-link"
                      title={`When ${item().lang.name} was spoken, with sources`}
                      onClick={() => setLangView(true)}
                    >
                      {item().lang.name} <span class="period-label">{p().label}</span>
                    </button>
                  )}
                </Show>
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
              {/* In an outline, the word's way back up the tree (the chart is
                  itself the lineage); then its origin, with who says so. */}
              <Show when={forest()?.kind !== TreeKind.Etymology && node().parent}>
                <Lineage node={node()} />
              </Show>
              <OriginList item={item()} />
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
              </Show>
            </div>
          );
        }}
      </Show>
    </div>
  );
}

// "English motherhood ← Middle English moderhode ← moder ← … ← Proto-Indo-European
// *méh₂tēr": the word back up the tree to its root, the language named where it
// changes. A long one shows its two ends; the gap opens on click.
function Lineage(props: { node: TreeNode }) {
  const steps = () => {
    const out: TreeNode[] = [];
    for (let n: TreeNode | null = props.node; n; n = n.parent) out.push(n);
    return out;
  };
  const [open, setOpen] = createSignal(false);
  createEffect(on(() => props.node, () => setOpen(false)));
  // How many steps after the second hide behind the gap.
  const hidden = () => (open() || steps().length <= 5 ? 0 : steps().length - 4);
  const shown = (i: number) => i < 2 || i >= 2 + hidden();
  return (
    <p class="popover-lineage">
      <For each={steps()}>
        {(n, i) => (
          <Show when={shown(i())}>
            <Show when={hidden() > 0 && i() === 2 + hidden()}>
              <span class="lineage-step">
                {" ← "}
                <button
                  class="lineage-gap"
                  aria-label={`show ${hidden()} more steps`}
                  onClick={() => setOpen(true)}
                >
                  …
                </button>
              </span>
            </Show>
            <span class="lineage-step">
              <Show when={i() > 0}>
                <span title={steps()[i() - 1].etyMode ? etyModeShort(steps()[i() - 1].etyMode!) : undefined}>
                  {" ← "}
                </span>
              </Show>
              {/* The header above names the word's own language. */}
              <Show when={i() > 0 && (!shown(i() - 1) || steps()[i() - 1].item.lang.id !== n.item.lang.id)}>
                <LangName lang={n.item.lang} short={shortLangsInDetails()} class="ety-lang" />{" "}
              </Show>
              <span class="ety-term">{term(n.item)}</span>
            </span>
          </Show>
        )}
      </For>
    </p>
  );
}

// The word's etymologies, each with who states it, where, and the works they
// follow: "from Latin lupus · Wiktionary's editors, following De Vaan (2008),
// p. 353". Several are alternatives, "or".
function OriginList(props: { item: Item }) {
  const [origin] = createResource(() => props.item.id, fetchOrigin);
  const hypotheses = () => origin()?.hypotheses ?? [];
  return (
    <Show when={hypotheses().length > 0}>
      <ol class="popover-origin" aria-label="origin">
        <For each={hypotheses()}>
          {(h, i) => (
            <li>
              <div class="popover-ety">
                <Show when={i() > 0}>
                  <span class="ety-or">or </span>
                </Show>
                <Show when={h.kind !== "unknown"} fallback={<span class="ety-mode">origin unknown</span>}>
                  {etyLine(
                    h.mode!,
                    h.parents.map((p) => ({ item: p.item, langDistance: 0, certainty: p.certainty }))
                  )}
                </Show>
                <Show when={hypotheses().length > 1 && origin()!.primary === i()}>
                  <span class="popover-muted"> · preferred</span>
                </Show>
              </div>
              <AssertedBy asserted={h.asserted} item={props.item} hypothesis={h} />
            </li>
          )}
        </For>
      </ol>
    </Show>
  );
}

// "stated by Wiktionary's editors, in the Descendants of Latin lupus", or a
// rule's reading: "inferred by wety from a form-of sense".
function AssertedBy(props: { asserted: Assertion; item: Item; hypothesis: Hypothesis }) {
  const a = () => props.asserted;
  const elsewhere = () => a().page.id !== props.item.id;
  const page = () => (
    <>
      <LangName lang={a().page.lang} short={shortLangsInDetails()} class="ety-lang" />{" "}
      <span class="ety-term">{term(a().page)}</span>
    </>
  );
  const where = () => {
    if (a().by !== "wiktionary-editors") {
      if (a().section === "senses") return <> from a form-of sense</>;
      if (a().section === "categories" && !a().words?.includes(" from ")) return <> from its root category</>;
      if (elsewhere()) return <> from the etymology of {page()}</>;
      return null;
    }
    if (a().section === "Descendants") return <>, in the Descendants of {page()}</>;
    if (elsewhere()) return <>, in the etymology of {page()}</>;
    return null;
  };
  const roles = (role: string) => a().credits.filter((c) => c.role === role);
  return (
    <div class="origin-by">
      <span title={a().byName}>
        {a().by === "wiktionary-editors" ? "stated by Wiktionary's editors" : a().words}
      </span>
      {where()}
      <Show when={a().page.url}>
        {" "}
        <a class="origin-source" href={a().page.url!} target="_blank" rel="noopener noreferrer">
          ↗
        </a>
      </Show>
      <Show when={roles("follows").length}>
        <CreditList head="following" credits={roles("follows")} />
      </Show>
      <Show when={roles("data").length}>
        <CreditList head="dated after" credits={roles("data")} />
      </Show>
    </div>
  );
}

const SCOPE_NAME: Record<string, string> = {
  note: "footnote",
  entry: "References",
  inline: "in the text",
};

// A page given as a bare number reads "p. 353"; anything else as written.
function creditLocator(locator: string): string {
  return /^\d[\d–-]*$/.test(locator) ? `p. ${locator}` : locator;
}

function CreditList(props: { head: string; credits: Credit[] }) {
  return (
    <ul class="origin-credits">
      <span class="popover-muted">{props.head}:</span>
      <For each={props.credits}>
        {(c) => (
          <li>
            <span class="credit-cite">{c.citation}</span>
            <Show when={c.locator}>{(l) => <>, {creditLocator(l())}</>}</Show>
            <span class="popover-muted"> · {SCOPE_NAME[c.scope] ?? c.scope}</span>
          </li>
        )}
      </For>
    </ul>
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
        <>
          <LangName lang={s.item.lang} short={shortLangsInDetails()} class="ety-lang" />{" "}
        </>
      );
      prevLang = s.item.lang.id;
    }
    parts.push(<span class="ety-term">{term(s.item)}</span>);
    if (s.certainty === "possible" || s.certainty === "doubtful")
      parts.push(<span class="uncertain-mark" title={`${s.certainty} source`}>?</span>);
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
      <Show when={METHOD_NAME[e.method]}>{(m) => <span class="popover-muted"> · {m()}</span>}</Show>
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
        <Show when={props.period.estimate}>
          {(e) => (
            <span class="popover-muted">
              ({e().statistic}; {e().interval}){" "}
            </span>
          )}
        </Show>
        <span class="period-basis">· {BASIS_NAME[props.period.basis]}</span>
      </div>
      <ul class="period-cites">
        <For each={props.period.evidence}>{(e) => <li title={e.citation ?? undefined}>{cite(e)}</li>}</For>
      </ul>
      <Show when={props.period.revival?.length}>
        <div class="popover-muted">Being revived:</div>
        <ul class="period-cites">
          <For each={props.period.revival}>
            {(r) => (
              <li title={r.citation ?? undefined}>
                <span class="period-label">{r.label}</span> · {cite(r)}
              </li>
            )}
          </For>
        </ul>
      </Show>
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
                <span class="period-label">{a.label}</span>
                <Show when={a.estimate}>
                  {(e) => (
                    <span class="popover-muted">
                      {" "}
                      ({e().statistic}; {e().interval})
                    </span>
                  )}
                </Show>{" "}
                · {cite(a)}
                <Show when={a.note}>
                  <div class="popover-muted">{a.note}</div>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
