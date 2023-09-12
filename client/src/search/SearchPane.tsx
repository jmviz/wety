import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import { LangOption, ItemOption } from "./responses";

import Stack from "@mui/material/Stack";

interface SearchPaneProps {
  selectedLang: LangOption | null;
  setSelectedLang: (lang: LangOption | null) => void;
  selectedItem: ItemOption | null;
  setSelectedItem: (item: ItemOption | null) => void;
  selectedDescLangs: LangOption[];
  setSelectedDescLangs: (langs: LangOption[]) => void;
}

function SearchPane(props: SearchPaneProps) {
  return (
    <Stack sx={{ padding: 2 }} direction={"row"} spacing={2}>
      <LangSearch
        selectedLang={props.selectedLang}
        setSelectedLang={props.setSelectedLang}
        selectedDescLangs={props.selectedDescLangs}
        setSelectedDescLangs={props.setSelectedDescLangs}
      />
      <ItemSearch
        selectedLang={props.selectedLang}
        selectedItem={props.selectedItem}
        setSelectedItem={props.setSelectedItem}
      />
      <MultiLangSearch
        label="Descendant language(s)"
        selectedLangs={props.selectedDescLangs}
        setSelectedLangs={props.setSelectedDescLangs}
      />
    </Stack>
  );
}

export default SearchPane;
