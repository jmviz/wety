import { etyModeGroups } from "./etyModes";
import {
  disabledEtyModes,
  setDisabledEtyModes,
  hiddenLangs,
  setHiddenLangs,
  treeLangs,
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
        <div class="mode-groups">
          <For each={etyModeGroups}>
            {(group) => (
              <div class="mode-group">
                <p class="mode-group-title">
                  <button type="button" class="link" onClick={() => toggleGroup(group.modes)}>
                    {group.label}
                  </button>
                </p>
                <ul class="check-list">
                  <For each={group.modes}>
                    {(mode) => (
                      <li>
                        <label>
                          <input
                            type="checkbox"
                            checked={!disabledEtyModes().has(mode)}
                            onChange={() => toggleMode(mode)}
                          />
                          {mode}
                        </label>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            )}
          </For>
        </div>
      </section>
    </div>
  );
}
