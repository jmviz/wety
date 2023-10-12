import SearchPane from "./search/SearchPane";
import {
  Etymology,
  InterLangDescendants,
  Item,
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

export interface TreeData {
  tree: Etymology | InterLangDescendants | null;
  treeKind: TreeKind;
  selectedItem: Item | null;
  selectedLang: LangOption | null;
  selectedDescLangs: LangOption[];
}

export default function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);
  const [treeData, setTreeData] = useState<TreeData>({
    tree: null,
    treeKind: TreeKind.Etymology,
    selectedItem: null,
    selectedLang: selectedLang,
    selectedDescLangs: selectedDescLangs,
  });
  const [lastRequest, setLastRequest] = useState<string | null>(null);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
        setTreeData={setTreeData}
        lastRequest={lastRequest}
        setLastRequest={setLastRequest}
      />
      {treeData.treeKind === TreeKind.Etymology ? (
        <EtymologyTree
          treeData={treeData}
          setTreeData={setTreeData}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      ) : (
        <DescendantsTree
          treeData={treeData}
          setTreeData={setTreeData}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      )}
    </ThemeProvider>
  );
}
