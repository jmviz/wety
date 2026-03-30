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

import { useState, useRef, useEffect, useCallback } from "react";
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

  const cache = useRef<Map<string, Etymology | InterLangDescendants[]>>(
    new Map()
  );
  const isFromNavigation = useRef(false);
  const lastPushedPath = useRef<string | null>(null);

  const loadFromPath = useCallback(async (path: string) => {
    const parsed = TreeRequest.parsePath(path);
    if (!parsed) return;

    const makeRequest = (rootItem: Item, rootLang: Lang) => {
      const dummyDescLangs: Lang[] = parsed.descLangIds.map((id) => ({
        id,
        name: "",
      }));
      return new TreeRequest(rootLang, rootItem, dummyDescLangs, parsed.kind);
    };

    const cached = cache.current.get(path);
    if (cached) {
      let rootItem: Item;
      if (parsed.kind === TreeKind.Etymology) {
        rootItem = (cached as Etymology).item;
      } else {
        const trees = cached as InterLangDescendants[];
        rootItem =
          trees.length > 0
            ? trees[0].item
            : {
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
              };
      }
      isFromNavigation.current = true;
      setTree(cached);
      setLastRequest(makeRequest(rootItem, rootItem.lang));
      return;
    }

    try {
      const apiUrl = `${process.env.REACT_APP_API_BASE_URL}/api${path}`;
      const response = await fetch(apiUrl);
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
          const trees = treeResult as InterLangDescendants[];
          rootItem =
            trees.length > 0
              ? trees[0].item
              : {
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
                };
          break;
        }
      }

      cache.current.set(path, treeResult);
      isFromNavigation.current = true;
      setTree(treeResult);
      setLastRequest(makeRequest(rootItem, rootItem.lang));
    } catch (error) {
      console.log(error);
    }
  }, []);

  // When a new search result arrives, cache it and push its URL to history.
  useEffect(() => {
    if (lastRequest === null || tree === null) return;
    if (isFromNavigation.current) {
      isFromNavigation.current = false;
      return;
    }
    const path = lastRequest.apiPath();
    if (path === lastPushedPath.current) return;
    cache.current.set(path, tree);
    lastPushedPath.current = path;
    window.history.pushState(null, "", "#" + path);
  }, [lastRequest, tree]);

  // Handle browser back/forward and initial URL.
  useEffect(() => {
    const handlePopstate = () => {
      const hash = window.location.hash;
      if (!hash || hash === "#") return;
      loadFromPath(hash.slice(1));
    };

    window.addEventListener("popstate", handlePopstate);

    const initialHash = window.location.hash;
    if (initialHash && initialHash !== "#") {
      loadFromPath(initialHash.slice(1));
    }

    return () => {
      window.removeEventListener("popstate", handlePopstate);
    };
  }, [loadFromPath]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <SearchPane
        selectedLang={selectedLang}
        setSelectedLang={setSelectedLang}
        selectedItem={selectedItem}
        setSelectedItem={setSelectedItem}
        selectedDescLangs={selectedDescLangs}
        setSelectedDescLangs={setSelectedDescLangs}
        setTree={setTree}
        selectedTreeKind={selectedTreeKind}
        setSelectedTreeKind={setSelectedTreeKind}
        lastRequest={lastRequest}
        setLastRequest={setLastRequest}
      />
      {lastRequest?.kind === TreeKind.Etymology ? (
        <EtymologyTree
          setSelectedLang={setSelectedLang}
          setSelectedItem={setSelectedItem}
          selectedDescLangs={selectedDescLangs}
          setSelectedTreeKind={setSelectedTreeKind}
          tree={tree}
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
          tree={tree}
          setTree={setTree}
          lastRequest={lastRequest}
          setLastRequest={setLastRequest}
        />
      )}
    </ThemeProvider>
  );
}
