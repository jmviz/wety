import "./SettingsSidebar.css";
import { etyModeGroups } from "./etyModes";
import { disabledEtyModes } from "../signals";

import { useSignal } from "@preact/signals";

export default function SettingsSidebar() {
  const open = useSignal(false);

  const toggleMode = (mode: string) => {
    const next = new Set(disabledEtyModes.value);
    if (next.has(mode)) {
      next.delete(mode);
    } else {
      next.add(mode);
    }
    disabledEtyModes.value = next;
  };

  const toggleGroup = (modes: string[]) => {
    const allDisabled = modes.every((m) => disabledEtyModes.value.has(m));
    const next = new Set(disabledEtyModes.value);
    for (const m of modes) {
      if (allDisabled) {
        next.delete(m);
      } else {
        next.add(m);
      }
    }
    disabledEtyModes.value = next;
  };

  return (
    <>
      <button
        class="sidebar-toggle"
        onClick={() => {
          open.value = true;
        }}
        aria-label="settings"
      >
        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor">
          <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z" />
        </svg>
      </button>

      {open.value && (
        <div class="sidebar-overlay" onClick={() => { open.value = false; }} />
      )}

      <div class={`sidebar-drawer ${open.value ? "open" : ""}`}>
        <div class="sidebar-content">
          <h3 class="sidebar-title">Settings</h3>
          <p class="sidebar-subtitle">Connection types</p>

          {etyModeGroups.map((group) => {
            const allDisabled = group.modes.every((m) =>
              disabledEtyModes.value.has(m)
            );
            const someDisabled = group.modes.some((m) =>
              disabledEtyModes.value.has(m)
            );
            return (
              <div key={group.label} class="mode-group">
                <div class="mode-group-header">
                  <span class="mode-group-label">{group.label}</span>
                  <button
                    class="mode-group-toggle"
                    onClick={() => toggleGroup(group.modes)}
                  >
                    {allDisabled
                      ? "enable all"
                      : someDisabled
                      ? "enable all"
                      : "disable all"}
                  </button>
                </div>
                {group.modes.map((mode) => (
                  <div key={mode} class="mode-item">
                    <span class="mode-label">{mode}</span>
                    <label class="toggle-switch">
                      <input
                        type="checkbox"
                        checked={!disabledEtyModes.value.has(mode)}
                        onChange={() => toggleMode(mode)}
                      />
                      <span class="toggle-slider" />
                    </label>
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
