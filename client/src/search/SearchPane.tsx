import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import { Lang, Item } from "./types";
import TreeKindSelect from "./TreeKindSelect";
import { TreeKind } from "./types";

import Stack from "@mui/material/Stack";
import { useRef } from "react";
import { Container } from "@mui/material";

interface SearchPaneProps {
  selectedLang: Lang | null;
  setSelectedLang: (lang: Lang | null) => void;
  selectedItem: Item | null;
  setSelectedItem: (item: Item | null) => void;
  selectedDescLangs: Lang[];
  setSelectedDescLangs: (langs: Lang[]) => void;
  selectedTreeKind: TreeKind;
  setSelectedTreeKind: (treeKind: TreeKind) => void;
}

export default function SearchPane({
  selectedLang,
  setSelectedLang,
  selectedItem,
  setSelectedItem,
  selectedDescLangs,
  setSelectedDescLangs,
  selectedTreeKind,
  setSelectedTreeKind,
}: SearchPaneProps) {
  const langSearchInputRef = useRef<HTMLInputElement>(null);
  const itemSearchInputRef = useRef<HTMLInputElement>(null);
  const descLangsSearchInputRef = useRef<HTMLInputElement>(null);
  const etyButtonRef = useRef<HTMLButtonElement>(null);

  return (
    <Container>
      <Stack
        sx={{ padding: 2 }}
        direction={{ xs: "column", sm: "row" }}
        spacing={2}
        justifyContent={"center"}
      >
        <LangSearch
          selectedLang={selectedLang}
          setSelectedLang={setSelectedLang}
          inputRef={langSearchInputRef}
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
          etyButtonRef={etyButtonRef}
        />
        <MultiLangSearch
          label="Descendant language(s)"
          selectedLangs={selectedDescLangs}
          setSelectedLangs={setSelectedDescLangs}
          inputRef={descLangsSearchInputRef}
        />
        <TreeKindSelect
          selectedTreeKind={selectedTreeKind}
          setSelectedTreeKind={setSelectedTreeKind}
        />
        <EtyButton
          selectedLang={selectedLang}
          selectedItem={selectedItem}
          selectedDescLangs={selectedDescLangs}
          buttonRef={etyButtonRef}
          selectedTreeKind={selectedTreeKind}
        />
      </Stack>
    </Container>
  );
}
