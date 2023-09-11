import { ItemOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { useMemo, useState } from "react";

interface ItemSearchProps {
  selectedLangId: number | null;
  setSelectedItemId: (itemId: number | null) => void;
}

function ItemSearch({ selectedLangId, setSelectedItemId }: ItemSearchProps) {
  const [selectedItem, setSelectedItem] = useState<ItemOption | null>(null);
  const [itemOptions, setItemOptions] = useState<ItemOption[]>([]);

  const fetchItems = useMemo(
    () =>
      debounce(async (input: string) => {
        const response = await fetch(
          `${process.env.REACT_APP_API_BASE_URL}/items/${selectedLangId}/${input}`
        );
        const newOptions = (await response.json()) as ItemOption[];
        setItemOptions(newOptions);
      }, 500),
    [selectedLangId]
  );

  return (
    <Autocomplete
      sx={{
        width: 300,
      }}
      ListboxProps={{
        sx: {
          overflow: "scroll",
          whiteSpace: "nowrap",
        },
      }}
      freeSolo
      value={selectedItem}
      onChange={(event, newValue) => {
        if (typeof newValue === "string" || !newValue) {
          return;
        }
        setSelectedItem(newValue);
        setSelectedItemId(newValue.item.id);
      }}
      blurOnSelect
      onInputChange={(event, newInputValue) => {
        if (newInputValue === "" || selectedLangId === null) {
          setItemOptions([]);
          setSelectedItem(null);
          setSelectedItemId(null);
          return;
        }
        fetchItems(newInputValue);
      }}
      renderInput={(params) => <TextField {...params} label="Term..." />}
      options={itemOptions}
      filterOptions={(x) => x}
      getOptionLabel={(option) =>
        typeof option === "string" ? option : option.item.term
      }
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

export default ItemSearch;
