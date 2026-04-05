import "./ItemSearch.css";
import { Item, term } from "./types";
import Autocomplete from "./Autocomplete";
import {
  selectedLang,
  selectedItem,
  selectedDescLangs,
  debounce,
} from "../signals";

import { useSignal } from "@preact/signals";
import { Ref } from "preact";
import { useCallback, useMemo } from "preact/hooks";

interface ItemSearchProps {
  inputRef: Ref<HTMLInputElement>;
  descLangsSearchInputRef: Ref<HTMLInputElement>;
  etyButtonRef: Ref<HTMLButtonElement>;
}

export default function ItemSearch({
  inputRef,
  descLangsSearchInputRef,
  etyButtonRef,
}: ItemSearchProps) {
  const itemOptions = useSignal<Item[]>([]);

  const clearSelectedItemAndOptions = useCallback(() => {
    itemOptions.value = [];
    selectedItem.value = null;
  }, [itemOptions]);

  const fetchItems = useMemo(
    () =>
      debounce(async (input: string) => {
        const lang = selectedLang.value;
        if (lang === null) {
          clearSelectedItemAndOptions();
          return;
        }
        try {
          const response = await fetch(
            `${import.meta.env.VITE_API_BASE_URL}/search/item/${lang.id}?term=${input}`
          );
          const newOptions = (await response.json()) as Item[];
          itemOptions.value = newOptions;
        } catch (error) {
          console.log(error);
          clearSelectedItemAndOptions();
        }
      }, 500),
    [itemOptions, clearSelectedItemAndOptions]
  );

  const handleSelect = useCallback(
    (item: Item | null) => {
      selectedItem.value = item;
      if (selectedLang.value && item) {
        if (selectedDescLangs.value.length > 0) {
          const btn = (etyButtonRef as { current: HTMLButtonElement | null })
            .current;
          if (btn) {
            btn.disabled = false;
            btn.focus();
          }
        } else {
          const descInput = (
            descLangsSearchInputRef as { current: HTMLInputElement | null }
          ).current;
          descInput?.focus();
        }
      }
    },
    [etyButtonRef, descLangsSearchInputRef]
  );

  const handleInputChange = useCallback(
    (value: string) => {
      const cleanValue = cleanSearchTerm(value);
      if (cleanValue === "" || selectedLang.value === null) {
        clearSelectedItemAndOptions();
        return;
      }
      fetchItems(cleanValue);
    },
    [clearSelectedItemAndOptions, fetchItems]
  );

  return (
    <Autocomplete
      width="30ch"
      label="Term"
      placeholder="Term..."
      options={itemOptions}
      onSelect={handleSelect}
      onInputChange={handleInputChange}
      getLabel={(item) => term(item)}
      isEqual={(a, b) => a.id === b.id}
      inputRef={inputRef}
      renderOption={(option) => {
        const pos = option.pos ?? [];
        const gloss = option.gloss ?? [];
        return (
          <>
            <div class="term-line">{term(option)}</div>
            {pos.map((p, i) => (
              <div key={i} class="pos-line">
                <span class="pos">{p}</span>:{" "}
                <span class="gloss">{gloss[i]}</span>
              </div>
            ))}
          </>
        );
      }}
    />
  );
}

function cleanSearchTerm(term: string) {
  return term.trim().replace(/^\*/, "").toLowerCase();
}
