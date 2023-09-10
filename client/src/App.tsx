import "./App.css";
import SearchPane from "./SearchPane";

import { Fragment, useState } from "react";
import CssBaseline from "@mui/material/CssBaseline";

function App() {
  const [langId, setLangId] = useState<number | null>(null);
  // const [includeLangIds, setIncludeLangIds] = useState<number[]>([]);
  // const [termId, setTermId] = useState<number | null>(null);

  return (
    <Fragment>
      <CssBaseline />
      <div className="App">
        <header className="App-header">
          <SearchPane
            langId={langId}
            setLangId={setLangId}
            // setIncludeLangIds={setIncludeLangIds}
            // setTermId={setTermId}
          />
        </header>
      </div>
    </Fragment>
  );
}

export default App;
