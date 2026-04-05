import styles from "./SettingsSidebar.module.scss";
import { etyModeGroups } from "./etyModes";
import { disabledEtyModes, setDisabledEtyModes } from "../state";

import { For } from "solid-js";
import { Drawer } from "@ark-ui/solid";
import { Switch as ArkSwitch } from "@ark-ui/solid";

export default function SettingsSidebar() {
  const toggleMode = (mode: string) => {
    const next = new Set(disabledEtyModes());
    if (next.has(mode)) {
      next.delete(mode);
    } else {
      next.add(mode);
    }
    setDisabledEtyModes(next);
  };

  const toggleGroup = (modes: string[]) => {
    const allDisabled = modes.every((m) => disabledEtyModes().has(m));
    const next = new Set(disabledEtyModes());
    for (const m of modes) {
      if (allDisabled) {
        next.delete(m);
      } else {
        next.add(m);
      }
    }
    setDisabledEtyModes(next);
  };

  return (
    <Drawer.Root>
      <Drawer.Trigger class={styles.toggle} aria-label="settings">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor">
          <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z" />
        </svg>
      </Drawer.Trigger>
      <Drawer.Backdrop class={styles.backdrop} />
      <Drawer.Positioner>
        <Drawer.Content class={styles.drawer}>
          <div class={styles.content}>
            <h3 class={styles.title}>Settings</h3>
            <p class={styles.subtitle}>Connection types</p>

            <For each={etyModeGroups}>
              {(group) => {
                const allDisabled = () =>
                  group.modes.every((m) => disabledEtyModes().has(m));
                const someDisabled = () =>
                  group.modes.some((m) => disabledEtyModes().has(m));

                return (
                  <div class={styles.modeGroup}>
                    <div class={styles.modeGroupHeader}>
                      <span class={styles.modeGroupLabel}>{group.label}</span>
                      <button
                        class={styles.modeGroupToggle}
                        onClick={() => toggleGroup(group.modes)}
                      >
                        {allDisabled()
                          ? "enable all"
                          : someDisabled()
                          ? "enable all"
                          : "disable all"}
                      </button>
                    </div>
                    <For each={group.modes}>
                      {(mode) => (
                        <div class={styles.modeItem}>
                          <span class={styles.modeLabel}>{mode}</span>
                          <ArkSwitch.Root
                            checked={!disabledEtyModes().has(mode)}
                            onCheckedChange={() => toggleMode(mode)}
                          >
                            <ArkSwitch.Control class={styles.switchControl}>
                              <ArkSwitch.Thumb class={styles.switchThumb} />
                            </ArkSwitch.Control>
                            <ArkSwitch.HiddenInput />
                          </ArkSwitch.Root>
                        </div>
                      )}
                    </For>
                  </div>
                );
              }}
            </For>
          </div>
        </Drawer.Content>
      </Drawer.Positioner>
    </Drawer.Root>
  );
}
