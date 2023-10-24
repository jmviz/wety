import SearchPane from "./search/SearchPane";
import {
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeRequest,
} from "./search/types";
import { TreeKind } from "./search/types";
import EtymologyTree from "./ety/EtymologyTree";
import DescendantsTree from "./ety/DescendantsTree";

import { useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

export default function App() {
  const [selectedLang, setSelectedLang] = useState<Lang | null>(null);
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<Lang[]>([]);
  const [selectedTreeKind, setSelectedTreeKind] = useState<TreeKind>(
    TreeKind.Etymology
  );
  const [tree, setTree] = useState<Etymology | InterLangDescendants | null>(
    null
  );
  const [lastRequest, setLastRequest] = useState<TreeRequest | null>(null);

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
        selectedTreeKind={selectedTreeKind}
        setSelectedTreeKind={setSelectedTreeKind}
        lastRequest={lastRequest}
        setLastRequest={setLastRequest}
      />
      {lastRequest?.kind === TreeKind.Etymology ? (
        <EtymologyTree
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          setSelectedTreeKind={setSelectedTreeKind}
          tree={tree}
          setTree={setTree}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      ) : (
        <DescendantsTree
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          setSelectedTreeKind={setSelectedTreeKind}
          tree={tree}
          setTree={setTree}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      )}
    </ThemeProvider>
  );
}
