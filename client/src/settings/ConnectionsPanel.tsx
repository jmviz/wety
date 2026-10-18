import { allEtyModes, etyModeCategories } from "./etyModes";
import { disabledEtyModes, setDisabledEtyModes, treeModes } from "../state";

import { For, Show, createEffect } from "solid-js";

// Which kinds of connection the tree follows, by the few categories present
// in it; the fine-grained modes are under "more". Client-side: nothing here
// re-fetches.
export default function ConnectionsPanel() {
  const setModes = (modes: string[], on: boolean) => {
    const next = new Set(disabledEtyModes());
    for (const m of modes) {
      if (on) next.delete(m);
      else next.add(m);
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
  const offCount = (modes: string[]) => modes.filter((m) => disabledEtyModes().has(m)).length;

  return (
    <div class="panel connections" role="group" aria-label="connections">
      <p class="panel-title">
        Follow
        <Show when={disabledEtyModes().size > 0}>
          <button type="button" class="link" onClick={() => setDisabledEtyModes(new Set())}>
            all
          </button>
        </Show>
      </p>
      <ul class="check-list">
        <For each={categories()}>
          {(c) => (
            <li>
              <label>
                <input
                  type="checkbox"
                  checked={offCount(c.modes) === 0}
                  ref={(el) =>
                    createEffect(() => {
                      el.indeterminate = offCount(c.modes) > 0 && offCount(c.modes) < c.modes.length;
                    })
                  }
                  onChange={(e) => setModes(c.modes, e.currentTarget.checked)}
                />
                {c.label} <span class="count">{c.count.toLocaleString()}</span>
              </label>
            </li>
          )}
        </For>
      </ul>
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
                    onChange={(e) => setModes([mode], e.currentTarget.checked)}
                  />
                  {mode} <span class="count">{treeModes().get(mode)}</span>
                </label>
              </li>
            )}
          </For>
        </ul>
      </details>
    </div>
  );
}
