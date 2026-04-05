import { Item, Lang } from "./types";
import Autocomplete from "./Autocomplete";
import {
  selectedLang,
  selectedItem,
  selectedDescLangs,
  debounce,
} from "../signals";

import { useSignal } from "@preact/signals";
import { Ref } from "preact";
import { useEffect, useCallback, useMemo } from "preact/hooks";

interface LangSearchProps {
  inputRef: Ref<HTMLInputElement>;
  itemSearchInputRef: Ref<HTMLInputElement>;
}

export default function LangSearch({
  inputRef,
  itemSearchInputRef,
}: LangSearchProps) {
  const langOptions = useSignal<Lang[]>([]);

  const getStoredLastLang = useCallback(async () => {
    const lastLangStr = window.localStorage.getItem("lastLang");
    const input = (inputRef as { current: HTMLInputElement | null }).current;
    if (lastLangStr === null) {
      input?.focus();
      return;
    }
    try {
      const lastLang = JSON.parse(lastLangStr) as Lang;
      console.log(`Attempting to use stored last language ${lastLang.name}...`);
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${lastLang.name}`
      );
      const options = (await response.json()) as Lang[];
      const lang = options[0];
      if (lang.name === lastLang.name) {
        console.log(
          `Using stored last language ${lang.name} with id ${lang.id}.`
        );
        if (lang.id !== lastLang.id) {
          console.log(`The previous id for ${lang.name} was ${lastLang.id}.`);
        }
        selectedLang.value = lang;
        selectedDescLangs.value = [lang];
        if (input) input.value = lang.name;
        const itemInput = (
          itemSearchInputRef as { current: HTMLInputElement | null }
        ).current;
        itemInput?.focus();
        return;
      }
      throw new Error("Unable to use stored last language.");
    } catch (error) {
      console.log(error);
      window.localStorage.removeItem("lastLang");
      input?.focus();
    }
  }, [inputRef, itemSearchInputRef]);

  useEffect(() => {
    getStoredLastLang();
  }, [getStoredLastLang]);

  const clearSelectedLangAndOptions = useCallback(() => {
    langOptions.value = [];
    selectedLang.value = null;
    window.localStorage.removeItem("lastLang");
    selectedItem.value = null;
  }, [langOptions]);

  const fetchLangs = useMemo(
    () =>
      debounce(async (input: string) => {
        try {
          const response = await fetch(
            `${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${input}`
          );
          const newOptions = (await response.json()) as Lang[];
          langOptions.value = newOptions;
        } catch (error) {
          console.log(error);
          clearSelectedLangAndOptions();
        }
      }, 500),
    [langOptions, clearSelectedLangAndOptions]
  );

  const handleSelect = useCallback(
    (lang: Lang | null) => {
      selectedLang.value = lang;
      selectedItem.value = null;
      if (lang !== null) {
        const itemInput = (
          itemSearchInputRef as { current: HTMLInputElement | null }
        ).current;
        itemInput?.focus();
        if (selectedDescLangs.value.length === 0) {
          selectedDescLangs.value = [lang];
        }
        window.localStorage.setItem("lastLang", JSON.stringify(lang));
      }
    },
    [itemSearchInputRef]
  );

  const handleInputChange = useCallback(
    (value: string) => {
      if (value === "") {
        clearSelectedLangAndOptions();
        return;
      }
      fetchLangs(value);
    },
    [clearSelectedLangAndOptions, fetchLangs]
  );

  return (
    <Autocomplete
      width="25ch"
      label="Language"
      placeholder="Language..."
      options={langOptions}
      onSelect={handleSelect}
      onInputChange={handleInputChange}
      getLabel={(lang) => lang.name}
      isEqual={(a, b) => a.id === b.id}
      inputRef={inputRef}
    />
  );
}
