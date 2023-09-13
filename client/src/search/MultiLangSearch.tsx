import { LangOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { useCallback, useMemo, useState } from "react";

interface MultiLangSearchProps {
  label: string;
  selectedLangs: LangOption[];
  setSelectedLangs: (langs: LangOption[]) => void;
}

function MultiLangSearch({
  label,
  selectedLangs,
  setSelectedLangs,
}: MultiLangSearchProps) {
  const [langOptions, setLangOptions] = useState<LangOption[]>([]);

  const clearSelectedLangAndOptions = useCallback(() => {
    setLangOptions([]);
    setSelectedLangs([]);
  }, [setSelectedLangs]);

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
      sx={{ width: 300 }}
      multiple
      limitTags={1}
      freeSolo
      value={selectedLangs}
      onChange={(event, newValue) => {
        if (
          newValue.length > 0 &&
          typeof newValue[newValue.length - 1] === "string"
        ) {
          const match = langOptions.find(
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
              }, [] as LangOption[])
            );
            return;
          }
          setSelectedLangs(selectedLangs);
          return;
        }
        setSelectedLangs(newValue as LangOption[]);
      }}
      onInputChange={(event, newInputValue) => {
        if (newInputValue === "") {
          clearSelectedLangAndOptions();
          return;
        }
        fetchLangs(newInputValue);
      }}
      renderInput={(params) => (
        <TextField {...params} label={label} placeholder="Language(s)..." />
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

export default MultiLangSearch;
