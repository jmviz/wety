import ItemSearch from "./ItemSearch";
import LangSearch from "./LangSearch";
import MultiLangSearch from "./MultiLangSearch";
import EtyButton from "./EtyButton";
import {
  LangOption,
  ItemOption,
  Etymology,
  InterLangDescendants,
} from "./responses";
import { TreeKind } from "../App";

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
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  setTreeKind: (treeKind: TreeKind) => void;
  lastRequest: string | null;
  setLastRequest: (request: string | null) => void;
}

export default function SearchPane({
  selectedLang,
  setSelectedLang,
  selectedItem,
  setSelectedItem,
  selectedDescLangs,
  setSelectedDescLangs,
  setTree,
  setTreeKind,
  lastRequest,
  setLastRequest,
}: SearchPaneProps) {
  const langSearchInputRef = useRef<HTMLInputElement>(null);
  const itemSearchInputRef = useRef<HTMLInputElement>(null);
  const descLangsSearchInputRef = useRef<HTMLInputElement>(null);

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
          setTree={setTree}
          setTreeKind={setTreeKind}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      </Stack>
    </Container>
  );
}
