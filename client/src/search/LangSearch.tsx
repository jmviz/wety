import { LangOption } from "./responses";

import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import { debounce } from "@mui/material/utils";
import { useMemo, useState } from "react";

interface LangSearchProps {
  setSelectedLangId: (langId: number | null) => void;
}

function LangSearch({ setSelectedLangId }: LangSearchProps) {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [langOptions, setLangOptions] = useState<LangOption[]>([]);

  const fetchLangs = useMemo(
    () =>
      debounce(async (input: string) => {
        const response = await fetch(
          `${process.env.REACT_APP_API_BASE_URL}/langs/${input}`
        );
        const newOptions = (await response.json()) as LangOption[];
        setLangOptions(newOptions);
      }, 500),
    []
  );

  return (
    <Autocomplete
      sx={{ width: 200 }}
      freeSolo
      value={selectedLang}
      onChange={(event, newValue) => {
        if (typeof newValue === "string" || !newValue) {
          return;
        }
        setSelectedLang(newValue);
        setSelectedLangId(newValue.id);
      }}
      blurOnSelect
      onInputChange={(event, newInputValue) => {
        if (newInputValue === "") {
          setLangOptions([]);
          setSelectedLang(null);
          setSelectedLangId(null);
          return;
        }
        fetchLangs(newInputValue);
      }}
      renderInput={(params) => <TextField {...params} label="Language..." />}
      options={langOptions}
      filterOptions={(x) => x}
      getOptionLabel={(option) =>
        typeof option === "string" ? option : option.name
      }
    />
  );
}

export default LangSearch;
