import "./App.css";
import SearchPane from "./search/SearchPane";
import { LangOption, ItemOption } from "./search/responses";

import { useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

function App() {
  const [selectedLang, setSelectedLang] = useState<LangOption | null>(null);
  const [selectedItem, setSelectedItem] = useState<ItemOption | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<LangOption[]>([]);

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
          />
        </header>
      </div>
    </ThemeProvider>
  );
}

export default App;
