import "./ItemSearch.css";
import { Item, term } from "./types";
import {
  selectedLang,
  selectedItem,
  setSelectedItem,
  selectedDescLangs,
  debounce,
} from "../state";

import { createSignal, createMemo, For } from "solid-js";
import { Combobox, createListCollection } from "@ark-ui/solid";

interface ItemSearchProps {
  setInputRef: (el: HTMLInputElement) => void;
  focusDescLangsInput: () => void;
  focusEtyButton: () => void;
}

export default function ItemSearch(props: ItemSearchProps) {
  const [itemOptions, setItemOptions] = createSignal<Item[]>([]);

  const collection = createMemo(() =>
    createListCollection({
      items: itemOptions(),
      itemToString: (item) => term(item),
      itemToValue: (item) => String(item.id),
    })
  );

  const fetchItems = debounce(async (input: string) => {
    const lang = selectedLang();
    if (lang === null) {
      setItemOptions([]);
      setSelectedItem(null);
      return;
    }
    try {
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}/search/item/${lang.id}?term=${input}`
      );
      const options = (await response.json()) as Item[];
      setItemOptions(options);
    } catch (error) {
      console.log(error);
      setItemOptions([]);
      setSelectedItem(null);
    }
  }, 500);

  return (
    <Combobox.Root
      collection={collection()}
      openOnClick
      allowCustomValue
      inputBehavior="autohighlight"
      onValueChange={(details) => {
        const val = details.value[0];
        if (!val) {
          setSelectedItem(null);
          return;
        }
        const item = itemOptions().find((i) => String(i.id) === val);
        if (item) {
          setSelectedItem(item);
          if (selectedLang() && selectedDescLangs().length > 0) {
            props.focusEtyButton();
          } else {
            props.focusDescLangsInput();
          }
        }
      }}
      onInputValueChange={(details) => {
        const clean = cleanSearchTerm(details.inputValue);
        if (clean === "" || selectedLang() === null) {
          setItemOptions([]);
          setSelectedItem(null);
          return;
        }
        fetchItems(clean);
      }}
    >
      <Combobox.Label>Term</Combobox.Label>
      <Combobox.Control>
        <Combobox.Input
          ref={props.setInputRef}
          placeholder="Term..."
          value={selectedItem() ? term(selectedItem()!) : ""}
        />
      </Combobox.Control>
      <Combobox.Positioner>
        <Combobox.Content>
          <For each={collection().items}>
            {(item) => (
              <Combobox.Item item={item}>
                <Combobox.ItemText>{term(item)}</Combobox.ItemText>
                <For each={item.pos ?? []}>
                  {(pos, i) => (
                    <div class="pos-line">
                      <span class="pos">{pos}</span>:{" "}
                      <span class="gloss">{(item.gloss ?? [])[i()]}</span>
                    </div>
                  )}
                </For>
              </Combobox.Item>
            )}
          </For>
        </Combobox.Content>
      </Combobox.Positioner>
    </Combobox.Root>
  );
}

function cleanSearchTerm(term: string) {
  return term.trim().replace(/^\*/, "").toLowerCase();
}
