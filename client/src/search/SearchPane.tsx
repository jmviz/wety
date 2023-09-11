import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";

import Stack from "@mui/material/Stack";

interface SearchPaneProps {
  selectedLangId: number | null;
  setSelectedLangId: (langId: number | null) => void;
  selectedItemId: number | null;
  setSelectedItemId: (itemId: number | null) => void;
}

function SearchPane({
  selectedLangId,
  setSelectedLangId,
  selectedItemId,
  setSelectedItemId,
}: SearchPaneProps) {
  return (
    <Stack sx={{ padding: 2 }} direction={"row"} spacing={2}>
      <LangSearch setSelectedLangId={setSelectedLangId} />
      <ItemSearch
        selectedLangId={selectedLangId}
        setSelectedItemId={setSelectedItemId}
      />
    </Stack>
  );
}

export default SearchPane;
