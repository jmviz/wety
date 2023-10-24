import { Lang } from "./types";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { RefObject, useCallback, useMemo, useState } from "react";

interface MultiLangSearchProps {
  label: string;
  selectedLangs: Lang[];
  setSelectedLangs: (langs: Lang[]) => void;
  inputRef: RefObject<HTMLInputElement>;
}

function MultiLangSearch({
  label,
  selectedLangs,
  setSelectedLangs,
  inputRef,
}: MultiLangSearchProps) {
  const [Langs, setLangs] = useState<Lang[]>([]);

  const clearSelectedLangAndOptions = useCallback(() => {
    setLangs([]);
    setSelectedLangs([]);
  }, [setSelectedLangs]);

  const fetchLangs = useMemo(
    () =>
      debounce(async (input: string) => {
        try {
          const response = await fetch(
            `${process.env.REACT_APP_API_BASE_URL}/search/lang?name=${input}`
          );
          const newOptions = (await response.json()) as Lang[];
          setLangs(newOptions);
        } catch (error) {
          console.log(error);
          clearSelectedLangAndOptions();
        }
      }, 500),
    [clearSelectedLangAndOptions]
  );

  return (
    <Autocomplete
      sx={{ width: "35ch" }}
      multiple
      limitTags={1}
      freeSolo
      value={selectedLangs}
      onChange={(event, newValue) => {
        if (
          newValue.length > 0 &&
          typeof newValue[newValue.length - 1] === "string"
        ) {
          const match = Langs.find(
            (lo) =>
              lo.name.toLowerCase() ===
              (newValue[newValue.length - 1] as string).trim().toLowerCase()
          );
          if (match) {
            setSelectedLangs(
              selectedLangs.concat([match]).reduce((acc, curr) => {
                if (!acc.some((lo) => lo.id === curr.id)) {
                  acc.push(curr);
                }
                return acc;
              }, [] as Lang[])
            );
            return;
          }
          setSelectedLangs(selectedLangs);
          return;
        }
        setSelectedLangs(newValue as Lang[]);
      }}
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
          label={label}
          placeholder="Language(s)..."
          inputRef={inputRef}
        />
      )}
      options={Langs}
      filterOptions={(x) => x}
      getOptionLabel={(option) =>
        typeof option === "string" ? option : option.name
      }
      isOptionEqualToValue={(option, value) => option.id === value.id}
    />
  );
}

export default MultiLangSearch;
