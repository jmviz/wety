import Autocomplete from "@mui/material/Autocomplete";
import TextField from "@mui/material/TextField";
import Stack from "@mui/material/Stack";
import { debounce } from "@mui/material/utils";
import { useMemo, useState } from "react";

interface LangOption {
  id: number;
  code: string;
  name: string;
  similarity: number;
  items: number;
}

interface SearchPaneProps {
  langId: number | null;
  setLangId: (langId: number | null) => void;
}

function SearchPane({ langId, setLangId }: SearchPaneProps) {
  const [lang, setLang] = useState<LangOption | null>(null);
  const [options, setOptions] = useState<LangOption[]>([]);

  const fetchLangs = useMemo(
    () =>
      debounce(async (input: string) => {
        const response = await fetch(
          `${process.env.REACT_APP_API_BASE_URL}/langs/${input}`
        );
        const newOptions = (await response.json()) as LangOption[];
        setOptions(newOptions);
      }, 500),
    []
  );

  return (
    <Stack direction={"row"} spacing={2}>
      <Autocomplete
        sx={{ width: 300 }}
        id="lang-search"
        className="search-bar"
        freeSolo
        value={lang}
        onChange={(event, newValue) => {
          if (typeof newValue === "string" || !newValue) {
            return;
          }
          setLang(newValue);
          setLangId(newValue.id);
        }}
        onInputChange={(event, newInputValue) => {
          if (newInputValue === "") {
            setOptions([]);
            setLang(null);
            setLangId(null);
            return;
          }
          fetchLangs(newInputValue);
        }}
        renderInput={(params) => <TextField {...params} label="Language..." />}
        options={options}
        filterOptions={(x) => x}
        getOptionLabel={(option) =>
          typeof option === "string" ? option : option.name
        }
        noOptionsText="No languages found"
      />
    </Stack>
  );
}

export default SearchPane;
