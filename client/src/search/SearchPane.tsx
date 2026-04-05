import styles from "./SearchPane.module.scss";
import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import TreeKindSelect from "./TreeKindSelect";

export default function SearchPane() {
  let langInputEl: HTMLInputElement | undefined;
  let itemInputEl: HTMLInputElement | undefined;
  let descLangsInputEl: HTMLInputElement | undefined;
  let etyButtonEl: HTMLButtonElement | undefined;

  return (
    <div class={styles.container}>
      <div class={styles.pane}>
        <LangSearch
          setInputRef={(el) => (langInputEl = el)}
          focusItemInput={() => itemInputEl?.focus()}
        />
        <ItemSearch
          setInputRef={(el) => (itemInputEl = el)}
          focusDescLangsInput={() => descLangsInputEl?.focus()}
          focusEtyButton={() => {
            if (etyButtonEl) {
              etyButtonEl.disabled = false;
              etyButtonEl.focus();
            }
          }}
        />
        <MultiLangSearch
          label="Descendant language(s)"
          setInputRef={(el) => (descLangsInputEl = el)}
        />
        <TreeKindSelect />
        <EtyButton setButtonRef={(el) => (etyButtonEl = el)} />
      </div>
    </div>
  );
}
