import { For, Show } from "solid-js";
import { TreeKind } from "../search/types";
import { BASIS_MARK, BASIS_NAME, showDates } from "./periods";

export default function TreeGuide(props: { kind: TreeKind }) {
  return (
    <div class="tree-reading">
      <p class="context-note">{props.kind === TreeKind.Etymology
        ? "Read downward from the searched word to the words and components it comes from."
        : "Branches run from earlier sources to later forms. Target languages include the connecting languages along each path."}</p>
      <details class="reading-guide">
        <summary>How to read this tree</summary>
        <p>Click a word for definitions, relationships and sources. Hover gives a short preview. An asterisk (*) marks a reconstructed form.</p>
        <p>Connection labels distinguish inheritance, borrowing and word formation. Related words can be linked by loans or shared components; they are not necessarily inherited cognates.</p>
        <p>Colours show language distance from the searched word’s language, not dates or certainty. Dotted connectors join forms in the same language; dashed connectors cross a language hidden by filters.</p>
        <Show when={showDates()}>
          <p>Dates belong to languages, not individual words. A proto-language estimate may give only an estimated end, not a complete lifespan. Open a word’s details for uncertainty and citations.</p>
          <dl class="date-legend">
            <For each={Object.keys(BASIS_MARK) as (keyof typeof BASIS_MARK)[]}>{(basis) => <>
              <dt>{BASIS_MARK[basis]}</dt><dd>{BASIS_NAME[basis]}</dd>
            </>}</For>
            <dt>rev</dt><dd>being revived after native transmission ended</dd>
          </dl>
        </Show>
      </details>
    </div>
  );
}
