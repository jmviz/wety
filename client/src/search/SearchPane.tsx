import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import { LangOption, ItemOption, ExpandedItem } from "./responses";

import Stack from "@mui/material/Stack";

interface SearchPaneProps {
  selectedLang: LangOption | null;
  setSelectedLang: (lang: LangOption | null) => void;
  selectedItem: ItemOption | null;
  setSelectedItem: (item: ItemOption | null) => void;
  selectedDescLangs: LangOption[];
  setSelectedDescLangs: (langs: LangOption[]) => void;
  setEtyData: (data: ExpandedItem | null) => void;
}

function SearchPane({
  selectedLang,
  setSelectedLang,
  selectedItem,
  setSelectedItem,
  selectedDescLangs,
  setSelectedDescLangs,
  setEtyData,
}: SearchPaneProps) {
  return (
    <Stack sx={{ padding: 2 }} direction={"row"} spacing={2}>
      <LangSearch
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
      />
      <ItemSearch
        selectedLang={selectedLang}
        selectedItem={selectedItem}
        setSelectedItem={setSelectedItem}
      />
      <MultiLangSearch
        label="Descendant language(s)"
        selectedLangs={selectedDescLangs}
        setSelectedLangs={setSelectedDescLangs}
      />
      <EtyButton
        selectedLang={selectedLang}
        selectedItem={selectedItem}
        selectedDescLangs={selectedDescLangs}
        setEtyData={setEtyData}
      />
    </Stack>
  );
}

export default SearchPane;
