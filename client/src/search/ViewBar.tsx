import { Lang, TreeKind, term } from "./types";
import { searchLangs } from "./api";
import { useGoTo, withOwnLang } from "./navigate";
import FiltersPanel from "../settings/FiltersPanel";
import { showDates, toggleDates } from "../ety/periods";
import { browserLangName } from "../prefs";
import {
  lastRequest,
  selectedDescLangs,
  setSelectedDescLangs,
  disabledEtyModes,
  hiddenLangs,
  debounce,
  forest,
  exploration,
  explorationError,
  resetFilters,
  treeModes,
  treeLangs,
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
  const hiddenConnections = () => [...treeModes().keys()].filter((m) => disabledEtyModes().has(m)).length;
  const hiddenLanguages = () => treeLangs().filter((l) => hiddenLangs().has(l.lang.id)).length;
  const filterCount = () => hiddenConnections() + hiddenLanguages();

  const summary = () => {
    const ls = selectedDescLangs().slice().sort((a, b) => a.name.localeCompare(b.name));
    const head = ls.slice(0, 3).map((l) => l.name).join(", ");
    return ls.length > 3 ? `${head} +${ls.length - 3}` : head || "no languages";
  };

  const applyDescLangs = (next: Lang[]) => {
    const r = request();
    const langs = r ? withOwnLang(next, r.item.lang) : next;
    setSelectedDescLangs(langs);
    if (r && r.kind !== TreeKind.Etymology) goTo(r.kind, r.item, langs);
  };
  const suggestions = () => {
    const browserName = browserLangName();
    return (exploration()?.languages ?? [])
      .filter((l) => !selectedDescLangs().some((s) => s.code === l.lang.code))
      .slice().sort((a, b) => Number(b.lang.name === browserName) - Number(a.lang.name === browserName) || b.count - a.count)
      .slice(0, 4);
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
                Target languages: <span class="langs-summary">{summary()}</span>
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
          <Show when={kind() === TreeKind.Cognates}>
            <div class="relationship-scope" role="group" aria-label="cognate relationships">
              <For each={[["head", "Base lineage"], ["all", "All components"]] as const}>
                {([scope, label]) => (
                  <button type="button" class="scope-choice" aria-pressed={r().scope === scope}
                    onClick={() => r().scope !== scope && goTo(r().kind, r().item, selectedDescLangs(), scope)}>
                    {label}
                  </button>
                )}
              </For>
              <span class="scope-explanation">
                <Show when={r().scope === "head"} fallback={<>Includes shared affixes and compound components.</>}>
                  <Show when={exploration()?.head} fallback={<>Follows the single recorded base at each step.</>}>
                    {(head) => <>Through <span class="lang">{head().lang.name}</span> <span class="term">{term(head())}</span>.</>}
                  </Show>
                </Show>{" "}Loans and word formations are included.
              </span>
            </div>
          </Show>
          <Show when={panel() === "langs" && hasDescLangs()}>
            <LangSetEditor
              value={selectedDescLangs()}
              own={r().item.lang}
              onChange={applyDescLangs}
            />
          </Show>
          <Show when={hasDescLangs() && (panel() === "langs" || forest()?.nodeCount === 1)}>
            <div class="language-discovery">
              <Show when={suggestions().length} fallback={
                <p class="context-note">
                  {explorationError() ? "Language suggestions could not be loaded. Use Target languages to search for one."
                    : exploration() ? "No additional languages are reachable in this relationship view."
                    : "Checking languages in this tree…"}
                </p>
              }>
                <p class="context-note">Explore more branches of <span class="term">{term(r().item)}</span>. Suggestions rank languages by reachable words, with your browser language first when present. Counts precede target-language and display filtering.</p>
                <div class="suggestions" aria-label="suggested target languages">
                  <For each={suggestions()}>{(s) => (
                    <button type="button" class="language-suggestion" onClick={() => applyDescLangs([...selectedDescLangs(), s.lang])}>
                      {s.lang.name} <span>{s.count.toLocaleString()} reachable {s.count === 1 ? "word" : "words"}
                      {s.lang.name === browserLangName() ? " · browser language" : ""}</span>
                    </button>
                  )}</For>
                </div>
              </Show>
            </div>
          </Show>
          <Show when={disabledEtyModes().size + hiddenLangs().size > 0}>
            <p class="active-filters" role="status">
              {filterCount() ? `${hiddenConnections()} connection types and ${hiddenLanguages()} languages hidden in this tree.` : "Filters are set; none affect this tree."}{" "}
              <button type="button" class="link" onClick={resetFilters}>Reset filters</button>
            </p>
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
function LangSetEditor(props: { value: Lang[]; own: Lang; onChange: (v: Lang[]) => void }) {
  let inputEl!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [found, setFound] = createSignal<Lang[]>([]);
  const [active, setActive] = createSignal(0);
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
    <div class="panel lang-set" role="group" aria-label="target languages">
      <p class="panel-title">Target languages</p>
      <div class="chips">
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
            aria-controls="target-language-menu"
            aria-autocomplete="list"
            aria-activedescendant={candidates().length ? `target-language-${active()}` : undefined}
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
            <ul id="target-language-menu" class="lang-add-menu" role="listbox" aria-label="matching languages">
              <For each={candidates()}>
                {(lang, i) => (
                  <li
                    id={`target-language-${i()}`}
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
        <span class="panel-note">The word’s language is always included. Connecting languages remain visible along the paths.</span>
      </p>
    </div>
  );
}
