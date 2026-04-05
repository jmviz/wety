import { TreeKind } from "./types";
import { selectedTreeKind } from "../signals";

export default function TreeKindSelect() {
  return (
    <div class="autocomplete" style={{ minWidth: "120px" }}>
      <label class="autocomplete-label">Mode</label>
      <select
        class="autocomplete-input"
        value={selectedTreeKind.value}
        onChange={(e) => {
          selectedTreeKind.value = (e.target as HTMLSelectElement)
            .value as TreeKind;
        }}
      >
        {Object.values(TreeKind).map((kind) => (
          <option key={kind} value={kind}>
            {kind}
          </option>
        ))}
      </select>
    </div>
  );
}
