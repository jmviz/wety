import { Lang } from "./types";
import { selectedDescLangs, setSelectedDescLangs, debounce } from "../state";

import { createSignal, createMemo, For } from "solid-js";
import { Combobox, createListCollection } from "@ark-ui/solid";

interface MultiLangSearchProps {
  label: string;
  setInputRef: (el: HTMLInputElement) => void;
}

export default function MultiLangSearch(props: MultiLangSearchProps) {
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
    }
  }, 300);

  return (
    <Combobox.Root
      collection={collection()}
      multiple
      openOnClick
      allowCustomValue
      inputBehavior="autohighlight"
      value={selectedDescLangs().map((l) => String(l.id))}
      onValueChange={(details) => {
        const newLangs: Lang[] = [];
        for (const val of details.value) {
          // Look up from current desc langs first, then from options
          const existing = selectedDescLangs().find(
            (l) => String(l.id) === val
          );
          if (existing) {
            newLangs.push(existing);
          } else {
            const fromOpts = langOptions().find(
              (l) => String(l.id) === val
            );
            if (fromOpts) newLangs.push(fromOpts);
          }
        }
        setSelectedDescLangs(newLangs);
      }}
      onInputValueChange={(details) => {
        if (details.inputValue === "") {
          setLangOptions([]);
          return;
        }
        fetchLangs(details.inputValue);
      }}
    >
      <Combobox.Label>{props.label}</Combobox.Label>
      <Combobox.Control>
        <For each={selectedDescLangs()}>
          {(lang) => (
            <span data-scope="combobox" data-part="tag">
              {lang.name}
              <button
                data-scope="combobox"
                data-part="tag-remove"
                onClick={() =>
                  setSelectedDescLangs(
                    selectedDescLangs().filter((l) => l.id !== lang.id)
                  )
                }
              >
                x
              </button>
            </span>
          )}
        </For>
        <Combobox.Input
          ref={props.setInputRef}
          placeholder="Language(s)..."
          onMouseDown={(e) => {
            if (document.activeElement !== e.currentTarget) {
              e.preventDefault();
              e.currentTarget.focus();
            }
          }}
          onFocus={(e) => e.currentTarget.select()}
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
