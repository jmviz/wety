import "./App.css";
import SearchPane from "./search/SearchPane";
import { LangOption, ItemOption } from "./search/responses";
import Ety from "./ety/Ety";
import { EtyData } from "./ety/tree";

import { useRef, useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedItem, setSelectedItem] = useState<ItemOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);
  const [etyData, setEtyData] = useState<EtyData>({
    headProgenitorTree: null,
    selectedItem: null,
  });
  const etyRef = useRef<HTMLDivElement>(null);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <div className="App">
        <header className="App-header">
          <SearchPane
            selectedLang={selectedLang}
            setSelectedLang={setSelectedLang}
            selectedItem={selectedItem}
            setSelectedItem={setSelectedItem}
            selectedDescLangs={selectedDescLangs}
            setSelectedDescLangs={setSelectedDescLangs}
            setEtyData={setEtyData}
          />
        </header>
        <div className="App-ety-container" ref={etyRef}>
          <Ety data={etyData} containerRef={etyRef} />
        </div>
      </div>
    </ThemeProvider>
  );
}

export default App;
