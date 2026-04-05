import "./SearchPane.css";
import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import TreeKindSelect from "./TreeKindSelect";

import { useRef } from "preact/hooks";

export default function SearchPane() {
  const langSearchInputRef = useRef<HTMLInputElement>(null);
  const itemSearchInputRef = useRef<HTMLInputElement>(null);
  const descLangsSearchInputRef = useRef<HTMLInputElement>(null);
  const etyButtonRef = useRef<HTMLButtonElement>(null);

  return (
    <div class="search-container">
      <div class="search-pane">
        <LangSearch
          inputRef={langSearchInputRef}
          itemSearchInputRef={itemSearchInputRef}
        />
        <ItemSearch
          inputRef={itemSearchInputRef}
          descLangsSearchInputRef={descLangsSearchInputRef}
          etyButtonRef={etyButtonRef}
        />
        <MultiLangSearch
          label="Descendant language(s)"
          inputRef={descLangsSearchInputRef}
        />
        <TreeKindSelect />
        <EtyButton buttonRef={etyButtonRef} />
      </div>
    </div>
  );
}
