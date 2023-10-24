import {
  Lang,
  Item,
  Etymology,
  InterLangDescendants,
  TreeRequest,
  Descendants,
} from "./types";
import { TreeKind } from "./types";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { useMemo } from "react";
import { interLangDescendants } from "../ety/DescendantsTree";

interface EtyButtonProps {
  selectedLang: Lang | null;
  selectedItem: Item | null;
  selectedDescLangs: Lang[];
  selectedTreeKind: TreeKind;
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  lastRequest: TreeRequest | null;
  setLastRequest: (request: TreeRequest | null) => void;
}

export default function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  setTree,
  selectedTreeKind,
  lastRequest,
  setLastRequest,
}: EtyButtonProps) {
  const onClick = useMemo(
    () =>
      debounce(async () => {
        if (!selectedLang || !selectedItem || selectedDescLangs.length === 0) {
          return;
        }
        const request = new TreeRequest(
          selectedLang,
          selectedItem,
          selectedDescLangs,
          selectedTreeKind
        );
        if (lastRequest && request.equals(lastRequest)) {
          return;
        }

        try {
          const response = await fetch(request.url());
          const tree = (await response.json()) as
            | Etymology
            | InterLangDescendants;
          console.log(tree);
          setLastRequest(request);
          setTree(
            selectedTreeKind === TreeKind.Etymology
              ? (tree as Etymology)
              : interLangDescendants(tree as Descendants)
          );
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [
      selectedLang,
      selectedItem,
      selectedDescLangs,
      selectedTreeKind,
      lastRequest,
      setLastRequest,
      setTree,
    ]
  );

  return (
    <Button
      variant="contained"
      aria-label="search"
      disabled={
        !selectedLang || !selectedItem || selectedDescLangs.length === 0
      }
      onClick={onClick}
    >
      <SearchIcon />
    </Button>
  );
}
