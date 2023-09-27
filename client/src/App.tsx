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
  selectedItem: Item | null;
  selectedDescLangs: LangOption[];
}

export default function App() {
  const [treeData, setTreeData] = useState<TreeData>({
    tree: null,
    treeKind: TreeKind.Etymology,
    selectedItem: null,
    selectedDescLangs: [],
  });

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane setTreeData={setTreeData} />
      {treeData.treeKind === TreeKind.Etymology ? (
        <EtymologyTree treeData={treeData} setTreeData={setTreeData} />
      ) : (
        <DescendantsTree {...treeData} />
      )}
    </ThemeProvider>
  );
}
