import "./ItemSearch.css";
import { ItemOption, LangOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { useCallback, useMemo, useState, RefObject } from "react";

interface ItemSearchProps {
  selectedLang: LangOption | null;
  selectedItem: ItemOption | null;
  setSelectedItem: (item: ItemOption | null) => void;
  inputRef: RefObject<HTMLInputElement>;
  selectedDescLangs: LangOption[];
  descLangsSearchInputRef: RefObject<HTMLInputElement>;
}

export default function ItemSearch({
  selectedLang,
  selectedItem,
  setSelectedItem,
  inputRef,
  selectedDescLangs,
  descLangsSearchInputRef,
}: ItemSearchProps) {
  const [itemOptions, setItemOptions] = useState<ItemOption[]>([]);

  const clearSelectedItemAndOptions = useCallback(() => {
    setItemOptions([]);
    setSelectedItem(null);
  }, [setSelectedItem]);

  const setSelectedItemAndMaybeFocus = useCallback(
    (item: ItemOption | null) => {
      setSelectedItem(item);
      if (selectedLang && item && selectedDescLangs.length === 0) {
        descLangsSearchInputRef.current?.focus();
      }
    },
    [
      setSelectedItem,
      selectedLang,
      selectedDescLangs.length,
      descLangsSearchInputRef,
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
            `${process.env.REACT_APP_API_BASE_URL}/items/${selectedLang.id}/${input}`
          );
          const newOptions = (await response.json()) as ItemOption[];
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
            (io) => io.item.term.toLowerCase() === newValue.trim().toLowerCase()
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
        if (newInputValue === "" || selectedLang === null) {
          clearSelectedItemAndOptions();
          return;
        }
        fetchItems(newInputValue);
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
        typeof option === "string" ? option : option.item.term
      }
      isOptionEqualToValue={(option, value) => option.item.id === value.item.id}
      renderOption={(props, option) => {
        const pos = option.item.pos ?? [];
        const gloss = option.item.gloss ?? [];
        return (
          <li {...props} key={option.item.id}>
            <div className="term-line">{option.item.term}</div>
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
