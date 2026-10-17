import { allEtyModes, etyModeGroups } from "./etyModes";
import {
  disabledEtyModes, setDisabledEtyModes, hiddenLangs, setHiddenLangs,
  treeLangs, treeModes, resetFilters,
} from "../state";
import { For, createSignal } from "solid-js";

const loans = new Set([
  "borrowed", "learned borrowing", "semi-learned borrowing", "unadapted borrowing",
  "orthographic borrowing", "semantic loan", "calque", "partial calque", "phono-semantic matching",
]);

export default function FiltersPanel() {
  const [languageQuery, setLanguageQuery] = createSignal("");
  const modes = () => [...new Set([...allEtyModes, ...treeModes().keys()])];
  const matches = (keep: (mode: string) => boolean) => modes().every((m) => disabledEtyModes().has(m) !== keep(m));
  const preset = (keep: (mode: string) => boolean) =>
    setDisabledEtyModes(new Set(modes().filter((m) => !keep(m))));
  const toggleMode = (mode: string) => {
    const next = new Set(disabledEtyModes());
    if (next.has(mode)) next.delete(mode); else next.add(mode);
    setDisabledEtyModes(next);
  };
  const toggleGroup = (modes: string[]) => {
    const allOn = modes.every((m) => !disabledEtyModes().has(m));
    const next = new Set(disabledEtyModes());
    for (const m of modes) { if (allOn) next.add(m); else next.delete(m); }
    setDisabledEtyModes(next);
  };
  const groups = () => {
    const known = new Set(allEtyModes);
    return [...etyModeGroups, { label: "Additional connections", modes: [...treeModes().keys()].filter((m) => !known.has(m)) }]
      .map((g) => ({ ...g, modes: g.modes.filter((m) => treeModes().has(m)) }))
      .filter((g) => g.modes.length);
  };
  return (
    <div class="panel filters" role="group" aria-label="filters">
      <div class="filter-presets" role="group" aria-label="connection presets">
        <span>Connections:</span>
        <button type="button" class="scope-choice" aria-pressed={disabledEtyModes().size === 0} onClick={() => setDisabledEtyModes(new Set())}>All connections</button>
        <button type="button" class="scope-choice" aria-pressed={matches((m) => !loans.has(m))} onClick={() => preset((m) => !loans.has(m))}>Without borrowing</button>
        <button type="button" class="scope-choice" aria-pressed={matches((m) => m === "inherited" || m === "root")} onClick={() => preset((m) => m === "inherited" || m === "root")}>Inheritance and roots</button>
        <button type="button" class="link" onClick={resetFilters}>Reset all filters</button>
        <p class="context-note">Filters affect this display. Unchecking a connection hides the branch reached through it.</p>
      </div>
      <details class="filter-section">
        <summary>Languages in this tree <span class="count">{treeLangs().length}</span></summary>
        <input class="filter-search" type="search" aria-label="find a language in this tree" placeholder="find a language…"
          value={languageQuery()} onInput={(e) => setLanguageQuery(e.currentTarget.value)} />
        <ul class="check-list bounded-list">
          <For each={treeLangs().filter((l) => l.lang.name.toLowerCase().includes(languageQuery().toLowerCase()))}>
            {(lc) => (
              <li><label>
                <input type="checkbox" checked={!hiddenLangs().has(lc.lang.id)} onChange={() => {
                  const next = new Set(hiddenLangs());
                  if (next.has(lc.lang.id)) next.delete(lc.lang.id); else next.add(lc.lang.id);
                  setHiddenLangs(next);
                }} />
                <span class="lang">{lc.lang.name}</span><span class="count">{lc.count}</span>
              </label></li>
            )}
          </For>
        </ul>
      </details>
      <section class="filter-section connection-groups" aria-label="connection types in this tree">
        <For each={groups()}>{(group) => (
          <details class="mode-group">
            <summary>{group.label} <span class="count">{group.modes.length}</span></summary>
            <button type="button" class="link group-toggle" onClick={() => toggleGroup(group.modes)}>Toggle this group</button>
            <ul class="check-list bounded-list">
              <For each={group.modes}>{(mode) => (
                <li><label>
                  <input type="checkbox" checked={!disabledEtyModes().has(mode)} onChange={() => toggleMode(mode)} />
                  {mode}<span class="count">{treeModes().get(mode)}</span>
                </label></li>
              )}</For>
            </ul>
          </details>
        )}</For>
      </section>
    </div>
  );
}
