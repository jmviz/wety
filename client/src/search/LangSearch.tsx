import { ItemOption, LangOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { RefObject, useCallback, useMemo, useState } from "react";

interface LangSearchProps {
  selectedLang: LangOption | null;
  setSelectedLang: (lang: LangOption | null) => void;
  setSelectedItem: (item: ItemOption | null) => void;
  itemSearchInputRef: RefObject<HTMLInputElement>;
  selectedDescLangs: LangOption[];
  setSelectedDescLangs: (langs: LangOption[]) => void;
}

function LangSearch({
  selectedLang,
  setSelectedLang,
  setSelectedItem,
  itemSearchInputRef,
  selectedDescLangs,
  setSelectedDescLangs,
}: LangSearchProps) {
  const [langOptions, setLangOptions] = useState<LangOption[]>([]);

  const clearSelectedLangAndOptions = useCallback(() => {
    setLangOptions([]);
    setSelectedLang(null);
    setSelectedItem(null);
  }, [setSelectedLang, setSelectedItem]);

  const setSelectedLangAndMaybeDescLangs = useCallback(
    (lang: LangOption | null) => {
      setSelectedLang(lang);
      setSelectedItem(null);
      if (lang !== null) {
        itemSearchInputRef.current?.focus();
        if (selectedDescLangs.length === 0) {
          setSelectedDescLangs([lang]);
        }
      }
    },
    [
      setSelectedLang,
      setSelectedItem,
      itemSearchInputRef,
      selectedDescLangs.length,
      setSelectedDescLangs,
    ]
  );

  const fetchLangs = useMemo(
    () =>
      debounce(async (input: string) => {
        try {
          const response = await fetch(
            `${process.env.REACT_APP_API_BASE_URL}/langs/${input}`
          );
          const newOptions = (await response.json()) as LangOption[];
          setLangOptions(newOptions);
        } catch (error) {
          console.log(error);
          clearSelectedLangAndOptions();
        }
      }, 500),
    [clearSelectedLangAndOptions]
  );

  return (
    <Autocomplete
      sx={{ width: "25ch" }}
      freeSolo
      value={selectedLang}
      onChange={(event, newValue) => {
        if (typeof newValue === "string") {
          const match = langOptions.find(
            (lo) => lo.name.toLowerCase() === newValue.trim().toLowerCase()
          );
          if (match) {
            setSelectedLangAndMaybeDescLangs(match);
            return;
          }
          clearSelectedLangAndOptions();
          return;
        }
        setSelectedLangAndMaybeDescLangs(newValue);
      }}
      blurOnSelect
      onInputChange={(event, newInputValue) => {
        if (newInputValue === "") {
          clearSelectedLangAndOptions();
          return;
        }
        fetchLangs(newInputValue);
      }}
      renderInput={(params) => (
        <TextField {...params} label="Language" placeholder="Language..." />
      )}
      options={langOptions}
      filterOptions={(x) => x}
      getOptionLabel={(option) =>
        typeof option === "string" ? option : option.name
      }
      isOptionEqualToValue={(option, value) => option.id === value.id}
    />
  );
}

export default LangSearch;
