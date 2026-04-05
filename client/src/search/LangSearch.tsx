import { Lang } from "./types";
import {
  selectedLang,
  setSelectedLang,
  setSelectedItem,
  selectedDescLangs,
  setSelectedDescLangs,
  debounce,
} from "../state";

import { createSignal, createMemo, onMount, For } from "solid-js";
import { Combobox, createListCollection } from "@ark-ui/solid";

interface LangSearchProps {
  setInputRef: (el: HTMLInputElement) => void;
  focusItemInput: () => void;
}

export default function LangSearch(props: LangSearchProps) {
  const [langOptions, setLangOptions] = createSignal<Lang[]>([]);

  const collection = createMemo(() =>
    createListCollection({
      items: langOptions(),
      itemToString: (item) => item.name,
      itemToValue: (item) => String(item.id),
    })
  );

  const fetchLangs = debounce(async (input: string) => {
    try {
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${input}`
      );
      const options = (await response.json()) as Lang[];
      setLangOptions(options);
    } catch (error) {
      console.log(error);
      setLangOptions([]);
      setSelectedLang(null);
      setSelectedItem(null);
    }
  }, 500);

  onMount(async () => {
    const lastLangStr = window.localStorage.getItem("lastLang");
    if (lastLangStr === null) return;
    try {
      const lastLang = JSON.parse(lastLangStr) as Lang;
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${lastLang.name}`
      );
      const options = (await response.json()) as Lang[];
      const lang = options[0];
      if (lang.name === lastLang.name) {
        setSelectedLang(lang);
        setSelectedDescLangs([lang]);
        props.focusItemInput();
      } else {
        throw new Error("Unable to use stored last language.");
      }
    } catch (error) {
      console.log(error);
      window.localStorage.removeItem("lastLang");
    }
  });

  return (
    <Combobox.Root
      collection={collection()}
      openOnClick
      allowCustomValue
      inputBehavior="autohighlight"
      onValueChange={(details) => {
        const val = details.value[0];
        if (!val) {
          setSelectedLang(null);
          setSelectedItem(null);
          window.localStorage.removeItem("lastLang");
          return;
        }
        const lang = langOptions().find((l) => String(l.id) === val);
        if (lang) {
          setSelectedLang(lang);
          setSelectedItem(null);
          if (selectedDescLangs().length === 0) {
            setSelectedDescLangs([lang]);
          }
          window.localStorage.setItem("lastLang", JSON.stringify(lang));
          props.focusItemInput();
        }
      }}
      onInputValueChange={(details) => {
        if (details.inputValue === "") {
          setLangOptions([]);
          setSelectedLang(null);
          setSelectedItem(null);
          window.localStorage.removeItem("lastLang");
          return;
        }
        fetchLangs(details.inputValue);
      }}
    >
      <Combobox.Label>Language</Combobox.Label>
      <Combobox.Control>
        <Combobox.Input
          ref={props.setInputRef}
          placeholder="Language..."
          value={selectedLang()?.name ?? ""}
        />
      </Combobox.Control>
      <Combobox.Positioner>
        <Combobox.Content>
          <For each={collection().items}>
            {(item) => (
              <Combobox.Item item={item}>
                <Combobox.ItemText>{item.name}</Combobox.ItemText>
              </Combobox.Item>
            )}
          </For>
        </Combobox.Content>
      </Combobox.Positioner>
    </Combobox.Root>
  );
}
