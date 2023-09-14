import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import { LangOption, ItemOption, ExpandedItem } from "./responses";

import Stack from "@mui/material/Stack";
import { useRef } from "react";
import { Container } from "@mui/material";

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
  let itemSearchInputRef = useRef<HTMLInputElement>(null);
  let descLangsSearchInputRef = useRef<HTMLInputElement>(null);

  return (
    <Container>
      <Stack
        sx={{ padding: 2 }}
        direction={{ xs: "column", sm: "row" }}
        spacing={2}
      >
        <LangSearch
          selectedLang={selectedLang}
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          itemSearchInputRef={itemSearchInputRef}
          selectedDescLangs={selectedDescLangs}
          setSelectedDescLangs={setSelectedDescLangs}
        />
        <ItemSearch
          selectedLang={selectedLang}
          selectedItem={selectedItem}
          setSelectedItem={setSelectedItem}
          inputRef={itemSearchInputRef}
          selectedDescLangs={selectedDescLangs}
          descLangsSearchInputRef={descLangsSearchInputRef}
        />
        <MultiLangSearch
          label="Descendant language(s)"
          selectedLangs={selectedDescLangs}
          setSelectedLangs={setSelectedDescLangs}
          inputRef={descLangsSearchInputRef}
        />
        <EtyButton
          selectedLang={selectedLang}
          selectedItem={selectedItem}
          selectedDescLangs={selectedDescLangs}
          setEtyData={setEtyData}
        />
      </Stack>
    </Container>
  );
}

export default SearchPane;
