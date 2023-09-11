import "./App.css";
import SearchPane from "./search/SearchPane";

import { Fragment, useState } from "react";
import CssBaseline from "@mui/material/CssBaseline";

function App() {
  const [selectedLangId, setSelectedLangId] = useState<number | null>(null);
  const [selectedItemId, setSelectedItemId] = useState<number | null>(null);

  return (
    <Fragment>
      <CssBaseline />
      <div className="App">
        <header className="App-header">
          <SearchPane
            selectedLangId={selectedLangId}
            setSelectedLangId={setSelectedLangId}
            selectedItemId={selectedItemId}
            setSelectedItemId={setSelectedItemId}
          />
        </header>
      </div>
    </Fragment>
  );
}

export default App;
