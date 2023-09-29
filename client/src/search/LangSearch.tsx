import { ItemOption, LangOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { RefObject, useCallback, useEffect, useMemo, useState } from "react";

interface LangSearchProps {
  selectedLang: LangOption | null;
  setSelectedLang: (lang: LangOption | null) => void;
  inputRef: RefObject<HTMLInputElement>;
  setSelectedItem: (item: ItemOption | null) => void;
  itemSearchInputRef: RefObject<HTMLInputElement>;
  selectedDescLangs: LangOption[];
  setSelectedDescLangs: (langs: LangOption[]) => void;
}

export default function LangSearch({
  selectedLang,
  setSelectedLang,
  inputRef,
  setSelectedItem,
  itemSearchInputRef,
  selectedDescLangs,
  setSelectedDescLangs,
}: LangSearchProps) {
  const getStoredLastLang = useCallback(async () => {
    const lastLangStr = window.localStorage.getItem("lastLang");
    if (lastLangStr === null) {
      inputRef.current?.focus();
      return;
    }
    try {
      const lastLang = JSON.parse(lastLangStr) as LangOption;
      console.log(`Attempting to use stored last language ${lastLang.name}...`);
      const response = await fetch(
        `${process.env.REACT_APP_API_BASE_URL}/search/lang?name=${lastLang.name}`
      );
      const options = (await response.json()) as LangOption[];
      const lang = options[0];
      if (lang.name === lastLang.name) {
        console.log(
          `Using stored last language ${lang.name} with id ${lang.id}.`
        );
        if (lang.id !== lastLang.id) {
          console.log(`The previous id for ${lang.name} was ${lastLang.id}.`);
        }
        setSelectedLang(lang);
        setSelectedDescLangs([lang]);
        itemSearchInputRef.current?.focus();
        return;
      }
      throw new Error("Unable to use stored last language.");
    } catch (error) {
      console.log(error);
      window.localStorage.removeItem("lastLang");
      inputRef.current?.focus();
    }
  }, [inputRef, setSelectedLang, itemSearchInputRef, setSelectedDescLangs]);

  useEffect(() => {
    getStoredLastLang();
  }, [getStoredLastLang]);

  const [langOptions, setLangOptions] = useState<LangOption[]>([]);

  const clearSelectedLangAndOptions = useCallback(() => {
    setLangOptions([]);
    setSelectedLang(null);
    window.localStorage.removeItem("lastLang");
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
        window.localStorage.setItem("lastLang", JSON.stringify(lang));
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
            `${process.env.REACT_APP_API_BASE_URL}/search/lang?name=${input}`
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
        <TextField
          {...params}
          label="Language"
          placeholder="Language..."
          inputRef={inputRef}
        />
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
