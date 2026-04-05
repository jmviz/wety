import "./Autocomplete.css";
import { Lang } from "./types";
import { selectedDescLangs, debounce } from "../signals";

import { useSignal } from "@preact/signals";
import { Ref } from "preact";
import { useCallback, useMemo } from "preact/hooks";

interface MultiLangSearchProps {
  label: string;
  inputRef: Ref<HTMLInputElement>;
}

export default function MultiLangSearch({
  label,
  inputRef,
}: MultiLangSearchProps) {
  const langOptions = useSignal<Lang[]>([]);
  const open = useSignal(false);
  const highlightedIndex = useSignal(-1);

  const clearOptions = useCallback(() => {
    langOptions.value = [];
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
          clearOptions();
        }
      }, 500),
    [langOptions, clearOptions]
  );

  const addLang = useCallback(
    (lang: Lang) => {
      const current = selectedDescLangs.value;
      if (!current.some((l) => l.id === lang.id)) {
        selectedDescLangs.value = [...current, lang];
      }
      const input = (inputRef as { current: HTMLInputElement | null }).current;
      if (input) input.value = "";
      open.value = false;
      highlightedIndex.value = -1;
    },
    [inputRef, open, highlightedIndex]
  );

  const removeLang = useCallback((lang: Lang) => {
    selectedDescLangs.value = selectedDescLangs.value.filter(
      (l) => l.id !== lang.id
    );
  }, []);

  const handleInput = useCallback(
    (e: Event) => {
      const value = (e.target as HTMLInputElement).value;
      if (value === "") {
        clearOptions();
        open.value = false;
        return;
      }
      fetchLangs(value);
      open.value = true;
      highlightedIndex.value = -1;
    },
    [clearOptions, fetchLangs, open, highlightedIndex]
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const opts = langOptions.value;
      if (!open.value || opts.length === 0) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        highlightedIndex.value = Math.min(
          highlightedIndex.value + 1,
          opts.length - 1
        );
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        highlightedIndex.value = Math.max(highlightedIndex.value - 1, 0);
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (highlightedIndex.value >= 0) {
          addLang(opts[highlightedIndex.value]);
        } else if (opts.length > 0) {
          const input = (inputRef as { current: HTMLInputElement | null })
            .current;
          if (input) {
            const val = input.value.trim().toLowerCase();
            const match = opts.find(
              (o) => o.name.toLowerCase() === val
            );
            if (match) addLang(match);
          }
        }
      } else if (e.key === "Escape") {
        open.value = false;
      }
    },
    [langOptions, open, highlightedIndex, addLang, inputRef]
  );

  const handleBlur = useCallback(() => {
    setTimeout(() => {
      open.value = false;
    }, 150);
  }, [open]);

  const selected = selectedDescLangs.value;

  return (
    <div class="autocomplete" style={{ width: "35ch" }}>
      <label class="autocomplete-label">{label}</label>
      <div class="multi-input-wrapper">
        {selected.map((lang) => (
          <span key={lang.id} class="multi-tag">
            {lang.name}
            <button
              class="multi-tag-remove"
              onClick={() => removeLang(lang)}
              type="button"
            >
              x
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          type="text"
          class="autocomplete-input multi-input"
          placeholder="Language(s)..."
          onInput={handleInput}
          onKeyDown={handleKeyDown}
          onFocus={() => {
            if (langOptions.value.length > 0) open.value = true;
          }}
          onBlur={handleBlur}
        />
      </div>
      {open.value && langOptions.value.length > 0 && (
        <ul class="autocomplete-listbox">
          {langOptions.value.map((option, i) => (
            <li
              key={option.id}
              class={`autocomplete-option ${
                i === highlightedIndex.value ? "highlighted" : ""
              }`}
              onMouseDown={(e) => {
                e.preventDefault();
                addLang(option);
              }}
              onMouseEnter={() => {
                highlightedIndex.value = i;
              }}
            >
              {option.name}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
