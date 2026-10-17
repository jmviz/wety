import { allEtyModes, etyModeCategories } from "./etyModes";
import {
  disabledEtyModes,
  setDisabledEtyModes,
  hiddenLangs,
  setHiddenLangs,
  treeLangs,
  treeModes,
} from "../state";

import { For, Show } from "solid-js";

// Inline panel of client-side filters for the current tree: which languages
// are drawn and which kinds of connection are followed. Nothing here
// re-fetches.
export default function FiltersPanel() {
  const toggleMode = (mode: string) => {
    const next = new Set(disabledEtyModes());
    if (next.has(mode)) next.delete(mode);
    else next.add(mode);
    setDisabledEtyModes(next);
  };

  const toggleGroup = (modes: string[]) => {
    const allOn = modes.every((m) => !disabledEtyModes().has(m));
    const next = new Set(disabledEtyModes());
    for (const m of modes) {
      if (allOn) next.add(m);
      else next.delete(m);
    }
    setDisabledEtyModes(next);
  };

  // The categories present in this tree, each with only its present modes;
  // a mode the client does not know counts as "other".
  const categories = () => {
    const present = treeModes();
    const known = new Set(allEtyModes);
    return etyModeCategories
      .map((c) => {
        const modes = [
          ...c.modes.filter((m) => present.has(m)),
          ...(c.label === "other" ? [...present.keys()].filter((m) => !known.has(m)) : []),
        ];
        return { label: c.label, modes, count: modes.reduce((n, m) => n + present.get(m)!, 0) };
      })
      .filter((c) => c.modes.length > 0);
  };
  const state = (modes: string[]) => {
    const off = modes.filter((m) => disabledEtyModes().has(m)).length;
    return off === 0 ? "on" : off === modes.length ? "off" : "mixed";
  };

  const toggleLang = (id: number) => {
    const next = new Set(hiddenLangs());
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setHiddenLangs(next);
  };

  return (
    <div class="panel filters" role="group" aria-label="filters">
      <Show when={treeLangs().length > 0}>
        <section class="filter-section">
          <p class="panel-title">
            Languages in this tree
            <Show when={hiddenLangs().size > 0}>
              <button type="button" class="link" onClick={() => setHiddenLangs(new Set())}>
                show all
              </button>
            </Show>
          </p>
          <ul class="check-list columns">
            <For each={treeLangs()}>
              {(lc) => (
                <li>
                  <label>
                    <input
                      type="checkbox"
                      checked={!hiddenLangs().has(lc.lang.id)}
                      onChange={() => toggleLang(lc.lang.id)}
                    />
                    <span class="lang">{lc.lang.name}</span>
                    <span class="count">{lc.count}</span>
                  </label>
                </li>
              )}
            </For>
          </ul>
        </section>
      </Show>
      <section class="filter-section">
        <p class="panel-title">
          Connections
          <Show when={disabledEtyModes().size > 0}>
            <button type="button" class="link" onClick={() => setDisabledEtyModes(new Set())}>
              follow all
            </button>
          </Show>
        </p>
        <div class="chips">
          <For each={categories()}>
            {(c) => (
              <button
                type="button"
                class="chip toggle"
                aria-pressed={state(c.modes) === "on" ? "true" : state(c.modes) === "off" ? "false" : "mixed"}
                onClick={() => toggleGroup(c.modes)}
              >
                {c.label} <span class="count">{c.count.toLocaleString()}</span>
              </button>
            )}
          </For>
        </div>
        <details class="mode-detail">
          <summary>more</summary>
          <ul class="check-list columns">
            <For each={categories().flatMap((c) => c.modes)}>
              {(mode) => (
                <li>
                  <label>
                    <input
                      type="checkbox"
                      checked={!disabledEtyModes().has(mode)}
                      onChange={() => toggleMode(mode)}
                    />
                    {mode}
                    <span class="count">{treeModes().get(mode)}</span>
                  </label>
                </li>
              )}
            </For>
          </ul>
        </details>
      </section>
    </div>
  );
}
