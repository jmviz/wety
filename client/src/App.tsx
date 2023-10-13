import SearchPane from "./search/SearchPane";
import {
  Etymology,
  InterLangDescendants,
  ItemOption,
  LangOption,
} from "./search/responses";
import EtymologyTree from "./ety/EtymologyTree";
import DescendantsTree from "./ety/DescendantsTree";

import { useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

export enum TreeKind {
  Etymology,
  Descendants,
}

export default function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedItem, setSelectedItem] = useState<ItemOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);
  const [tree, setTree] = useState<Etymology | InterLangDescendants | null>(
    null
  );
  const [treeKind, setTreeKind] = useState<TreeKind>(TreeKind.Etymology);
  const [lastRequest, setLastRequest] = useState<string | null>(null);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedItem={selectedItem}
        setSelectedItem={setSelectedItem}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
        setTree={setTree}
        setTreeKind={setTreeKind}
        lastRequest={lastRequest}
        setLastRequest={setLastRequest}
      />
      {treeKind === TreeKind.Etymology ? (
        <EtymologyTree
          selectedLang={selectedLang}
          selectedItem={selectedItem}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          tree={tree}
          setTree={setTree}
          setTreeKind={setTreeKind}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      ) : (
        <DescendantsTree
          selectedLang={selectedLang}
          selectedItem={selectedItem}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          tree={tree}
          setTree={setTree}
          setTreeKind={setTreeKind}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      )}
    </ThemeProvider>
  );
}
