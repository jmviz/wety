import { Lang, TreeKind } from "./types";
import { searchLangs } from "./api";
import { useGoTo } from "./navigate";
import FiltersPanel from "../settings/FiltersPanel";
import { showDates, toggleDates } from "../ety/periods";
import {
  lastRequest,
  selectedDescLangs,
  setSelectedDescLangs,
  disabledEtyModes,
  hiddenLangs,
  debounce,
} from "../state";

import { For, Show, createSignal } from "solid-js";

const VIEWS: [TreeKind, string][] = [
  [TreeKind.Etymology, "Etymology"],
  [TreeKind.Descendants, "Descendants"],
  [TreeKind.Cognates, "Cognates"],
];

// One quiet line under the search box: which view of the word is shown,
// which languages its descendants are drawn in, and the filters toggle.
export default function ViewBar() {
  const goTo = useGoTo();
  const [panel, setPanel] = createSignal<"langs" | "filters" | null>(null);
  const toggle = (p: "langs" | "filters") => setPanel(panel() === p ? null : p);

  const request = () => lastRequest();
  const kind = () => request()?.kind;
  const hasDescLangs = () => kind() !== TreeKind.Etymology;
  const filterCount = () => disabledEtyModes().size + hiddenLangs().size;

  const summary = () => {
    const ls = selectedDescLangs().slice().sort((a, b) => a.name.localeCompare(b.name));
    const head = ls.slice(0, 3).map((l) => l.name).join(", ");
    return ls.length > 3 ? `${head} +${ls.length - 3}` : head || "no languages";
  };

  const applyDescLangs = (next: Lang[]) => {
    setSelectedDescLangs(next);
    const r = request();
    if (r && r.kind !== TreeKind.Etymology) goTo(r.kind, r.item, next);
  };

  return (
    <Show when={request()}>
      {(r) => (
        <div class="view">
          <nav class="view-bar" aria-label="view">
            <For each={VIEWS}>
              {([k, label]) => (
                <button
                  type="button"
                  class="view-link"
                  classList={{ current: kind() === k }}
                  aria-current={kind() === k ? "page" : undefined}
                  onClick={() => kind() !== k && goTo(k, r().item)}
                >
                  {label}
                </button>
              )}
            </For>
            <Show when={hasDescLangs()}>
              <button
                type="button"
                class="view-link"
                classList={{ open: panel() === "langs" }}
                aria-expanded={panel() === "langs"}
                onClick={() => toggle("langs")}
              >
                in <span class="langs-summary">{summary()}</span>
                <span class="disclosure" aria-hidden="true" />
              </button>
            </Show>
            <button
              type="button"
              class="view-link"
              classList={{ open: panel() === "filters" }}
              aria-expanded={panel() === "filters"}
              onClick={() => toggle("filters")}
            >
              filters
              <Show when={filterCount() > 0}>
                {" "}
                <span class="count">{filterCount()}</span>
              </Show>
              <span class="disclosure" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="view-link"
              classList={{ on: showDates() }}
              aria-pressed={showDates()}
              title="Show when each language was spoken, where a cited period is available"
              onClick={toggleDates}
            >
              dates
            </button>
          </nav>
          <Show when={panel() === "langs" && hasDescLangs()}>
            <LangSetEditor value={selectedDescLangs()} onChange={applyDescLangs} />
          </Show>
          <Show when={panel() === "filters"}>
            <FiltersPanel />
          </Show>
        </div>
      )}
    </Show>
  );
}

// Chips for the descendant-language set plus a small language search.
function LangSetEditor(props: { value: Lang[]; onChange: (v: Lang[]) => void }) {
  let inputEl!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [found, setFound] = createSignal<Lang[]>([]);
  const [active, setActive] = createSignal(0);

  const candidates = () =>
    found().filter((l) => !props.value.some((v) => v.id === l.id)).slice(0, 6);

  const search = debounce(async (q: string) => {
    if (!q.trim()) return setFound([]);
    setFound(await searchLangs(q.trim()).catch(() => []));
    setActive(0);
  }, 120);

  const add = (lang: Lang) => {
    props.onChange([...props.value, lang]);
    setText("");
    setFound([]);
    inputEl.focus();
  };
  const remove = (lang: Lang) =>
    props.onChange(props.value.filter((l) => l.id !== lang.id));

  return (
    <div class="panel lang-set" role="group" aria-label="descendant languages">
      <p class="panel-title">Descendant languages</p>
      <div class="chips">
        <For each={props.value.slice().sort((a, b) => a.name.localeCompare(b.name))}>
          {(lang) => (
            <span class="chip removable">
              {lang.name}
              <button
                type="button"
                class="chip-remove"
                aria-label={`remove ${lang.name}`}
                onClick={() => remove(lang)}
              >
                ×
              </button>
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
            value={text()}
            onInput={(e) => {
              setText(e.currentTarget.value);
              search(e.currentTarget.value);
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
                setText("");
                setFound([]);
              }
            }}
          />
          <Show when={candidates().length > 0}>
            <ul class="lang-add-menu" role="listbox">
              <For each={candidates()}>
                {(lang, i) => (
                  <li
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
      <p class="panel-foot">
        <button type="button" class="link" onClick={() => props.onChange([])}>
          clear
        </button>
        <span class="panel-note">The word's own language is always included.</span>
      </p>
    </div>
  );
}
