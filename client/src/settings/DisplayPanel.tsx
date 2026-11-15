import { TreeKind, namesLangs } from "../search/types";
import { setAllOpen } from "../ety/model";
import { showDates, toggleDates } from "../ety/periods";
import { forest, lastRequest, setShowGlosses, setShowRare, showGlosses, showRare } from "../state";
import {
  setShortLangsInDetails,
  setShortLangsInTree,
  shortLangsInDetails,
  shortLangsInTree,
} from "../ety/langName";

import { Show } from "solid-js";

// How the rows are shown: meanings, language dates, the constellation's rare
// words, and the outline's open/closed state.
export default function DisplayPanel() {
  return (
    <div class="panel display" role="group" aria-label="display">
      <ul class="check-list">
        <li>
          <label>
            <input
              type="checkbox"
              checked={showGlosses()}
              onChange={(e) => setShowGlosses(e.currentTarget.checked)}
            />
            meanings
          </label>
        </li>
        <li>
          <label title="When each language was spoken, where a cited period is available">
            <input type="checkbox" checked={showDates()} onChange={toggleDates} />
            dates
          </label>
        </li>
        <li>
          <label title="Proto-Indo-European as PIE, with the full name on hover">
            <input
              type="checkbox"
              checked={shortLangsInTree()}
              onChange={(e) => setShortLangsInTree(e.currentTarget.checked)}
            />
            language initials in the tree
          </label>
        </li>
        <li>
          <label title="Proto-Indo-European as PIE, with the full name on hover">
            <input
              type="checkbox"
              checked={shortLangsInDetails()}
              onChange={(e) => setShortLangsInDetails(e.currentTarget.checked)}
            />
            language initials in word details
          </label>
        </li>
        <Show when={lastRequest()?.kind === TreeKind.Constellation}>
          <li>
            <label title="English words that are in no desk dictionary, and plain derivatives of a word already listed (forgiveness beside forgive)">
              <input
                type="checkbox"
                checked={showRare()}
                onChange={(e) => setShowRare(e.currentTarget.checked)}
              />
              rare and derived words
            </label>
          </li>
        </Show>
      </ul>
      <Show when={lastRequest() && namesLangs(lastRequest()!.kind) && forest()}>
        {(f) => (
          <p class="panel-foot">
            <button type="button" class="link" onClick={() => setAllOpen(f(), true)}>
              expand all
            </button>
            <button type="button" class="link" onClick={() => setAllOpen(f(), false)}>
              collapse all
            </button>
          </p>
        )}
      </Show>
    </div>
  );
}
