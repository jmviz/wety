import SearchPane from "./search/SearchPane";
import { Descendants, Etymology, Item, LangOption } from "./search/responses";
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
  tree: Etymology | Descendants | null;
  treeKind: TreeKind;
  treeRootItem: Item | null;
  selectedLang: LangOption | null;
  selectedDescLangs: LangOption[];
}

export default function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);
  const [treeData, setTreeData] = useState<TreeData>({
    tree: null,
    treeKind: TreeKind.Etymology,
    treeRootItem: null,
    selectedLang: selectedLang,
    selectedDescLangs: selectedDescLangs,
  });

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
        setTreeData={setTreeData}
      />
      {treeData.treeKind === TreeKind.Etymology ? (
        <EtymologyTree treeData={treeData} setTreeData={setTreeData} />
      ) : (
        <DescendantsTree {...treeData} />
      )}
    </ThemeProvider>
  );
}
