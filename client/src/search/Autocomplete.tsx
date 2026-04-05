import "./Autocomplete.css";
import { Signal, useSignal } from "@preact/signals";
import { useRef, useCallback } from "preact/hooks";
import { Ref, ComponentChildren } from "preact";

interface AutocompleteProps<T> {
  options: Signal<T[]>;
  onSelect: (value: T | null) => void;
  onInputChange: (value: string) => void;
  getLabel: (option: T) => string;
  isEqual?: (a: T, b: T) => boolean;
  renderOption?: (option: T) => ComponentChildren;
  placeholder?: string;
  label: string;
  inputRef?: Ref<HTMLInputElement>;
  width?: string;
}

export default function Autocomplete<T>({
  options,
  onSelect,
  onInputChange,
  getLabel,
  isEqual,
  renderOption,
  placeholder,
  label,
  inputRef,
  width,
}: AutocompleteProps<T>) {
  const open = useSignal(false);
  const highlightedIndex = useSignal(-1);
  const internalRef = useRef<HTMLInputElement>(null);
  const ref = (inputRef as { current: HTMLInputElement | null }) ?? internalRef;
  const containerRef = useRef<HTMLDivElement>(null);

  const handleInput = useCallback(
    (e: Event) => {
      const value = (e.target as HTMLInputElement).value;
      onInputChange(value);
      open.value = true;
      highlightedIndex.value = -1;
    },
    [onInputChange, open, highlightedIndex]
  );

  const selectOption = useCallback(
    (option: T) => {
      const input = ref.current;
      if (input) input.value = getLabel(option);
      onSelect(option);
      open.value = false;
      highlightedIndex.value = -1;
    },
    [onSelect, getLabel, open, highlightedIndex, ref]
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const opts = options.value;
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
          selectOption(opts[highlightedIndex.value]);
        } else if (opts.length > 0) {
          // try exact match
          const input = ref.current;
          if (input) {
            const val = input.value.trim().toLowerCase();
            const match = opts.find(
              (o) => getLabel(o).toLowerCase() === val
            );
            if (match) selectOption(match);
          }
        }
      } else if (e.key === "Escape") {
        open.value = false;
      }
    },
    [options, open, highlightedIndex, selectOption, getLabel, ref]
  );

  const handleBlur = useCallback(
    (e: FocusEvent) => {
      // delay to allow click on option
      setTimeout(() => {
        if (!containerRef.current?.contains(document.activeElement)) {
          // on blur, try to match the typed text
          const input = ref.current;
          if (input) {
            const val = input.value.trim().toLowerCase();
            if (val === "") {
              onSelect(null);
            } else {
              const match = options.value.find(
                (o) => getLabel(o).toLowerCase() === val
              );
              if (match) {
                selectOption(match);
              }
            }
          }
          open.value = false;
        }
      }, 150);
    },
    [onSelect, options, getLabel, selectOption, open, ref]
  );

  return (
    <div
      class="autocomplete"
      style={width ? { width } : undefined}
      ref={containerRef}
    >
      <label class="autocomplete-label">{label}</label>
      <input
        ref={inputRef}
        type="text"
        class="autocomplete-input"
        placeholder={placeholder}
        onInput={handleInput}
        onKeyDown={handleKeyDown}
        onFocus={() => {
          if (options.value.length > 0) open.value = true;
        }}
        onBlur={handleBlur}
      />
      {open.value && options.value.length > 0 && (
        <ul class="autocomplete-listbox">
          {options.value.map((option, i) => (
            <li
              key={i}
              class={`autocomplete-option ${
                i === highlightedIndex.value ? "highlighted" : ""
              }`}
              onMouseDown={(e) => {
                e.preventDefault();
                selectOption(option);
              }}
              onMouseEnter={() => {
                highlightedIndex.value = i;
              }}
            >
              {renderOption ? renderOption(option) : getLabel(option)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
