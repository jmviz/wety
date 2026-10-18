import { Lang, TreeKind } from "./types";
import { searchLangs } from "./api";
import { useGoTo, withOwnLang } from "./navigate";
import ConnectionsPanel from "../settings/ConnectionsPanel";
import DisplayPanel from "../settings/DisplayPanel";
import Trail from "../ety/Trail";
import { openLang } from "../ety/model";
import {
  debounce,
  disabledEtyModes,
  forest,
  hiddenLangs,
  lastRequest,
  selectedDescLangs,
  setHiddenLangs,
  setSelectedDescLangs,
  treeLangs,
  treeModes,
  treeSize,
} from "../state";

import { For, Show, createSignal } from "solid-js";

const VIEWS: [TreeKind, string][] = [
  [TreeKind.Etymology, "Etymology"],
  [TreeKind.Descendants, "Descendants"],
  [TreeKind.Cognates, "Cognates"],
];

type Panel = "langs" | "connections" | "display";

// The one row of controls under the word: its three views on the left; on the
// right, which languages are drawn, which connections are followed, and how
// rows are shown. Each opens a panel beneath the row.
export default function ViewBar() {
  const goTo = useGoTo();
  const [panel, setPanel] = createSignal<Panel | null>(null);
  const toggle = (p: Panel) => setPanel(panel() === p ? null : p);

  const request = () => lastRequest();
  const kind = () => request()?.kind;
  const isTree = () => kind() !== TreeKind.Etymology;

  // The languages a tree is drawn in: the link's, else the reader's.
  const langs = () => {
    const r = request()!;
    return r.descLangs.length ? r.descLangs : withOwnLang(selectedDescLangs(), r.item.lang);
  };
  const langSummary = () => {
    if (treeSize()?.allLangs) return "all languages";
    const ls = langs().slice().sort((a, b) => a.name.localeCompare(b.name));
    const head = ls.slice(0, 2).map((l) => l.name).join(", ");
    return ls.length > 2 ? `${head} +${ls.length - 2}` : head;
  };
  const hiddenHere = () => treeLangs().filter((l) => hiddenLangs().has(l.lang.id)).length;
  const offModes = () => [...treeModes().keys()].filter((m) => disabledEtyModes().has(m)).length;

  return (
    <Show when={request()}>
      {(r) => (
        <div class="view-controls">
          <div class="view-bar">
            <nav class="view-group" aria-label="view">
              <For each={VIEWS}>
                {([k, label]) => (
                  <button
                    type="button"
                    class="view-link"
                    classList={{ current: kind() === k }}
                    aria-current={kind() === k ? "page" : undefined}
                    onClick={() => kind() !== k && goTo(k, r().item, r().choice)}
                  >
                    {label}
                  </button>
                )}
              </For>
            </nav>
            <div class="view-group">
              <Show when={isTree()}>
                <button
                  type="button"
                  class="view-link"
                  classList={{ open: panel() === "langs" }}
                  aria-expanded={panel() === "langs"}
                  onClick={() => toggle("langs")}
                >
                  in <span class="langs-summary">{langSummary()}</span>
                  <Show when={hiddenHere() > 0}>
                    {" "}
                    <span class="count">{hiddenHere()} hidden</span>
                  </Show>
                  <span class="disclosure" aria-hidden="true" />
                </button>
              </Show>
              <button
                type="button"
                class="view-link"
                classList={{ open: panel() === "connections" }}
                aria-expanded={panel() === "connections"}
                onClick={() => toggle("connections")}
              >
                connections
                <Show when={offModes() > 0}>
                  {" "}
                  <span class="count">{offModes()} off</span>
                </Show>
                <span class="disclosure" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="view-link"
                classList={{ open: panel() === "display" }}
                aria-expanded={panel() === "display"}
                onClick={() => toggle("display")}
              >
                display
                <span class="disclosure" aria-hidden="true" />
              </button>
            </div>
          </div>
          <Show when={kind() === TreeKind.Cognates}>
            <Trail />
          </Show>
          <Show when={panel() === "langs" && isTree()}>
            <LanguagesPanel
              langs={langs()}
              onLangs={(next) => {
                const chosen = withOwnLang(next, r().item.lang);
                setSelectedDescLangs(chosen);
                goTo(r().kind, r().item, { langs: chosen, within: r().within }, r().root);
              }}
              onAll={() =>
                goTo(r().kind, r().item, { all: !r().allLangs, within: r().within }, r().root)
              }
              onWithin={(within) => goTo(r().kind, r().item, { ...r().choice, within }, r().root)}
            />
          </Show>
          <Show when={panel() === "connections"}>
            <ConnectionsPanel />
          </Show>
          <Show when={panel() === "display"}>
            <DisplayPanel />
          </Show>
        </div>
      )}
    </Show>
  );
}

// Which languages are drawn (the reader's, or all of them), and the languages
// in this tree: hide one, or open every branch that reaches it.
function LanguagesPanel(props: {
  langs: Lang[];
  onLangs: (v: Lang[]) => void;
  onAll: () => void;
  onWithin: (within: boolean) => void;
}) {
  const r = () => lastRequest()!;
  const toggleLang = (id: number) => {
    const next = new Set(hiddenLangs());
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setHiddenLangs(next);
  };
  return (
    <div class="panel languages" role="group" aria-label="languages">
      <div class="panel-row">
        <span class="panel-title">Branches reaching</span>
        <LangSetEditor value={props.langs} own={r().item.lang} onChange={props.onLangs} />
        <span class="panel-or">or</span>
        <button type="button" class="link" aria-pressed={r().allLangs} classList={{ on: r().allLangs }} onClick={props.onAll}>
          all languages
        </button>
      </div>
      <ul class="check-list within-lang">
        <li>
          <label>
            <input
              type="checkbox"
              checked={r().within}
              onChange={(e) => props.onWithin(e.currentTarget.checked)}
            />
            {!r().allLangs && props.langs.length === 1
              ? `${props.langs[0].name} words formed from ${props.langs[0].name} words`
              : "words formed from words of the same language"}
          </label>
        </li>
      </ul>
      <Show when={treeLangs().length > 1}>
        <p class="panel-title">In this tree</p>
        <ul class="check-list columns">
          <For each={treeLangs()}>
            {(lc) => (
              <li>
                <label>
                  <input
                    type="checkbox"
                    checked={!hiddenLangs().has(lc.lang.id)}
                    aria-label={`show ${lc.lang.name}`}
                    onChange={() => toggleLang(lc.lang.id)}
                  />
                </label>
                <button
                  type="button"
                  class="lang-reveal"
                  title={`open every branch that reaches ${lc.lang.name}`}
                  onClick={() => forest() && openLang(forest()!, lc.lang.id)}
                >
                  <span class="lang">{lc.lang.name}</span> <span class="count">{lc.count}</span>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

// Chips for the reader's languages plus a small language search. The word's
// own language is always among them.
function LangSetEditor(props: { value: Lang[]; own: Lang; onChange: (v: Lang[]) => void }) {
  let inputEl!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [found, setFound] = createSignal<Lang[]>([]);
  const [active, setActive] = createSignal(0);
  // Bumped on every keystroke and pick, so a slow response for an earlier
  // query cannot reopen the menu.
  let seq = 0;

  const candidates = () =>
    found().filter((l) => !props.value.some((v) => v.id === l.id)).slice(0, 6);

  const search = debounce(async (q: string, mine: number) => {
    if (!q.trim()) return setFound([]);
    const result = await searchLangs(q.trim()).catch(() => []);
    if (mine !== seq) return;
    setFound(result);
    setActive(0);
  }, 120);

  const add = (lang: Lang) => {
    seq++;
    props.onChange([...props.value, lang]);
    setText("");
    setFound([]);
    inputEl.focus();
  };
  const remove = (lang: Lang) =>
    props.onChange(props.value.filter((l) => l.id !== lang.id));

  return (
    <div class="chips" role="group" aria-label="your languages">
        <For each={props.value.slice().sort((a, b) => a.name.localeCompare(b.name))}>
          {(lang) => (
            <span class="chip removable">
              {lang.name}
              {/* The word's own language is always included. */}
              <Show when={lang.id !== props.own.id}>
                <button
                  type="button"
                  class="chip-remove"
                  aria-label={`remove ${lang.name}`}
                  onClick={() => remove(lang)}
                >
                  ×
                </button>
              </Show>
            </span>
          )}
        </For>
        <span class="lang-add">
          <input
            ref={inputEl}
            type="text"
            class="lang-add-input"
            placeholder="add a language…"
            aria-label="add a language"
            autocomplete="off"
            role="combobox"
            aria-expanded={candidates().length > 0}
            aria-controls="desc-language-menu"
            aria-autocomplete="list"
            aria-activedescendant={candidates().length ? `desc-language-${active()}` : undefined}
            value={text()}
            onInput={(e) => {
              setText(e.currentTarget.value);
              setFound([]);
              search(e.currentTarget.value, ++seq);
            }}
            onKeyDown={(e) => {
              const c = candidates();
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((active() + 1) % Math.max(c.length, 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((active() - 1 + c.length) % Math.max(c.length, 1));
              } else if (e.key === "Enter") {
                e.preventDefault();
                if (c[active()]) add(c[active()]);
              } else if (e.key === "Escape") {
                seq++;
                setText("");
                setFound([]);
              }
            }}
          />
          <Show when={candidates().length > 0}>
            <ul id="desc-language-menu" class="lang-add-menu" role="listbox" aria-label="matching languages">
              <For each={candidates()}>
                {(lang, i) => (
                  <li
                    id={`desc-language-${i()}`}
                    role="option"
                    aria-selected={active() === i()}
                    classList={{ active: active() === i() }}
                    onPointerEnter={() => setActive(i())}
                    onPointerDown={(e) => e.preventDefault()}
                    onClick={() => add(lang)}
                  >
                    {lang.name}
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </span>
    </div>
  );
}
