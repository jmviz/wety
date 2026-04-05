import SearchPane from "./search/SearchPane";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeRequest,
  TreeKind,
} from "./search/types";
import EtymologyTree from "./ety/EtymologyTree";
import DescendantsTree, {
  interLangDescendants,
} from "./ety/DescendantsTree";
import SettingsSidebar from "./settings/SettingsSidebar";
import {
  filterEtymologyTree,
  filterDescendantsTree,
} from "./settings/filterTree";

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useLocation } from "react-router-dom";
import { CssBaseline, ThemeProvider, createTheme } from "@mui/material";

const theme = createTheme({
  // todo
});

export default function App() {
  const [selectedLang, setSelectedLang] = useState<Lang | null>(null);
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [selectedDescLangs, setSelectedDescLangs] = useState<Lang[]>([]);
  const [selectedTreeKind, setSelectedTreeKind] = useState<TreeKind>(
    TreeKind.Cognates
  );
  const [tree, setTree] = useState<Etymology | InterLangDescendants[] | null>(
    null
  );
  const [lastRequest, setLastRequest] = useState<TreeRequest | null>(null);
  const [disabledEtyModes, setDisabledEtyModes] = useState<Set<string>>(
    new Set()
  );

  const location = useLocation();
  const cache = useRef<Map<string, Etymology | InterLangDescendants[]>>(
    new Map()
  );

  const loadFromPath = useCallback(async (path: string) => {
    const parsed = TreeRequest.parsePath(path);
    if (!parsed) return;

    const makeRequest = (rootItem: Item) => {
      const descLangs: Lang[] = parsed.descLangIds.map((id) => ({
        id,
        name: "",
      }));
      return new TreeRequest(rootItem.lang, rootItem, descLangs, parsed.kind);
    };

    const dummyItem = (): Item => ({
      id: parsed.itemId,
      etyNum: 0,
      lang: { id: parsed.distLangId, name: "" },
      term: "",
      imputed: false,
      reconstructed: false,
      url: null,
      pos: null,
      gloss: null,
      romanization: null,
    });

    const cached = cache.current.get(path);
    if (cached) {
      const rootItem =
        parsed.kind === TreeKind.Etymology
          ? (cached as Etymology).item
          : (cached as InterLangDescendants[])[0]?.item ?? dummyItem();
      setTree(cached);
      setLastRequest(makeRequest(rootItem));
      return;
    }

    try {
      const response = await fetch(
        `${process.env.REACT_APP_API_BASE_URL}${path}`
      );
      const data = await response.json();

      let treeResult: Etymology | InterLangDescendants[];
      let rootItem: Item;

      switch (parsed.kind) {
        case TreeKind.Etymology: {
          treeResult = data as Etymology;
          rootItem = (treeResult as Etymology).item;
          break;
        }
        case TreeKind.Descendants: {
          treeResult = [interLangDescendants(data as Descendants)];
          rootItem = (treeResult as InterLangDescendants[])[0].item;
          break;
        }
        case TreeKind.Cognates: {
          treeResult = (data as Descendants[]).map((t) =>
            interLangDescendants(t)
          );
          rootItem =
            (treeResult as InterLangDescendants[])[0]?.item ?? dummyItem();
          break;
        }
      }

      cache.current.set(path, treeResult);
      setTree(treeResult);
      setLastRequest(makeRequest(rootItem));
    } catch (error) {
      console.log(error);
    }
  }, []);

  useEffect(() => {
    const path = location.pathname + location.search;
    if (path === "/") return;
    loadFromPath(path);
  }, [location, loadFromPath]);

  const filteredTree = useMemo(() => {
    if (tree === null || disabledEtyModes.size === 0) return tree;
    if (lastRequest?.kind === TreeKind.Etymology) {
      return filterEtymologyTree(tree as Etymology, disabledEtyModes);
    }
    return (tree as InterLangDescendants[]).map((t) =>
      filterDescendantsTree(t, disabledEtyModes)
    );
  }, [tree, disabledEtyModes, lastRequest]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SettingsSidebar
        disabledEtyModes={disabledEtyModes}
        setDisabledEtyModes={setDisabledEtyModes}
      />
      <SearchPane
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedItem={selectedItem}
        setSelectedItem={setSelectedItem}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
        selectedTreeKind={selectedTreeKind}
        setSelectedTreeKind={setSelectedTreeKind}
      />
      {lastRequest?.kind === TreeKind.Etymology ? (
        <EtymologyTree
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          setSelectedTreeKind={setSelectedTreeKind}
          tree={filteredTree}
          setTree={setTree}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      ) : (
        <DescendantsTree
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          setSelectedTreeKind={setSelectedTreeKind}
          tree={filteredTree}
          setTree={setTree}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      )}
    </ThemeProvider>
  );
}
