import SearchPane from "./search/SearchPane";
import { EtyData } from "./ety/Ety";
import Ety from "./ety/Ety";

import { useState } from "react";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

function App() {
  const [etyData, setEtyData] = useState<EtyData>({
    headProgenitorTree: null,
    selectedItem: null,
  });

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane setEtyData={setEtyData} />
      <Ety data={etyData} />
    </ThemeProvider>
  );
}

export default App;
