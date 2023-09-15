import "./App.css";
import SearchPane from "./search/SearchPane";
import { LangOption, ItemOption, Item } from "./search/responses";
import Ety from "./ety/Ety";
import { EtyData } from "./ety/tree";

import { useRef, useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";
import Tooltip from "./ety/Tooltip";

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
  const etyContainerRef = useRef<HTMLDivElement>(null);
  const [tooltipItem, setTooltipItem] = useState<Item | null>(null);
  // const tooltipRef = useRef<HTMLDivElement>(null);
  // const tooltipHideTimeout = useRef<number | null>(null);

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
        <div className="ety-container" ref={etyContainerRef}>
          <Ety
            data={etyData}
            containerRef={etyContainerRef}
            setTooltipItem={setTooltipItem}
            // tooltipRef={tooltipRef}
          />
        </div>
        {/* <Tooltip item={tooltipItem} ref={tooltipRef} /> */}
      </div>
    </ThemeProvider>
  );
}

export default App;
