import { Lang } from "../search/types";

import { createSignal } from "solid-js";

// Language names shortened to their initials (Proto-Indo-European: PIE, Middle
// English: ME), with the full name on hover. In the tree by choice; in word
// details by default, where the header gives the word's language in full.
export const [shortLangsInTree, setShortLangsInTree] = createSignal(false);
export const [shortLangsInDetails, setShortLangsInDetails] = createSignal(true);

export function initials(name: string): string {
  return name
    .split(/[\s\-–]+/)
    .map((w) => w.match(/\p{L}/u)?.[0] ?? "")
    .join("")
    .toUpperCase();
}

// A language name, full or as initials.
export function LangName(props: { lang: Lang; short: boolean; class?: string }) {
  return (
    <span class={props.class ?? "lang"} title={props.short ? props.lang.name : undefined}>
      {props.short ? initials(props.lang.name) : props.lang.name}
    </span>
  );
}
