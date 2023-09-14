import "./App.css";
import SearchPane from "./search/SearchPane";
import { LangOption, ItemOption, ExpandedItem } from "./search/responses";
import Ety from "./ety/Ety";

import { useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedItem, setSelectedItem] = useState<ItemOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);
  const [etyData, setEtyData] = useState<ExpandedItem | null>(null);

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
        <Ety etyData={etyData} selectedItem={selectedItem} />
      </div>
    </ThemeProvider>
  );
}

export default App;
