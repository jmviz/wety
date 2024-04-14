import "./ItemSearch.css";
import { Item, Lang, term } from "./types";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { useCallback, useMemo, useState, RefObject } from "react";

interface ItemSearchProps {
  selectedLang: Lang | null;
  selectedItem: Item | null;
  setSelectedItem: (item: Item | null) => void;
  inputRef: RefObject<HTMLInputElement>;
  selectedDescLangs: Lang[];
  descLangsSearchInputRef: RefObject<HTMLInputElement>;
  etyButtonRef: RefObject<HTMLButtonElement>;
}

export default function ItemSearch({
  selectedLang,
  selectedItem,
  setSelectedItem,
  inputRef,
  selectedDescLangs,
  descLangsSearchInputRef,
  etyButtonRef,
}: ItemSearchProps) {
  const [itemOptions, setItemOptions] = useState<Item[]>([]);

  const clearSelectedItemAndOptions = useCallback(() => {
    setItemOptions([]);
    setSelectedItem(null);
  }, [setSelectedItem]);

  const setSelectedItemAndMaybeFocus = useCallback(
    (item: Item | null) => {
      setSelectedItem(item);
      if (selectedLang && item) {
        if (selectedDescLangs.length > 0) {
          if (etyButtonRef.current) {
            etyButtonRef.current.disabled = false;
            etyButtonRef.current.focus();
          }
        } else {
          descLangsSearchInputRef.current?.focus();
        }
      }
    },
    [
      setSelectedItem,
      selectedLang,
      selectedDescLangs.length,
      descLangsSearchInputRef,
      etyButtonRef,
    ]
  );

  const fetchItems = useMemo(
    () =>
      debounce(async (input: string) => {
        if (selectedLang === null) {
          clearSelectedItemAndOptions();
          return;
        }
        try {
          const response = await fetch(
            `${process.env.REACT_APP_API_BASE_URL}/search/item/${selectedLang.id}?term=${input}`
          );
          const newOptions = (await response.json()) as Item[];
          setItemOptions(newOptions);
        } catch (error) {
          console.log(error);
          clearSelectedItemAndOptions();
        }
      }, 500),
    [selectedLang, clearSelectedItemAndOptions]
  );

  return (
    <Autocomplete
      sx={{
        width: "30ch",
      }}
      ListboxProps={{
        sx: {
          ".MuiAutocomplete-option": {
            display: "block",
          },
        },
      }}
      freeSolo
      value={selectedItem}
      onChange={(event, newValue) => {
        if (typeof newValue === "string") {
          const match = itemOptions.find(
            (io) => io.term.toLowerCase() === cleanSearchTerm(newValue)
          );
          if (match) {
            setSelectedItemAndMaybeFocus(match);
            return;
          }
          clearSelectedItemAndOptions();
          return;
        }
        setSelectedItemAndMaybeFocus(newValue);
      }}
      blurOnSelect
      onInputChange={(event, newInputValue) => {
        const cleanInputValue = cleanSearchTerm(newInputValue);
        if (cleanInputValue === "" || selectedLang === null) {
          clearSelectedItemAndOptions();
          return;
        }
        fetchItems(cleanInputValue);
      }}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Term"
          placeholder="Term..."
          inputRef={inputRef}
        />
      )}
      options={itemOptions}
      filterOptions={(x) => x}
      getOptionLabel={(option) =>
        typeof option === "string" ? option : term(option)
      }
      isOptionEqualToValue={(option, value) => option.id === value.id}
      renderOption={(props, option) => {
        const pos = option.pos ?? [];
        const gloss = option.gloss ?? [];
        return (
          <li {...props} key={option.id}>
            <div className="term-line">{term(option)}</div>
            {pos.map((p, i) => (
              <div key={i} className="pos-line">
                <span className="pos">{p}</span>:{" "}
                <span className="gloss">{gloss[i]}</span>
              </div>
            ))}
          </li>
        );
      }}
    />
  );
}

function cleanSearchTerm(term: string) {
  return term.trim().replace(/^\*/, "").toLowerCase();
}
