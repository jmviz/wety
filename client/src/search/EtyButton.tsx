import {
  LangOption,
  ItemOption,
  Etymology,
  InterLangDescendants,
} from "./responses";
import { TreeKind } from "../App";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { useMemo } from "react";

interface EtyButtonProps {
  selectedLang: LangOption | null;
  selectedItem: ItemOption | null;
  selectedDescLangs: LangOption[];
  setTree: (tree: Etymology | InterLangDescendants | null) => void;
  setTreeKind: (treeKind: TreeKind) => void;
  lastRequest: string | null;
  setLastRequest: (request: string | null) => void;
}

export default function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  setTree,
  setTreeKind,
  lastRequest,
  setLastRequest,
}: EtyButtonProps) {
  const onClick = useMemo(
    () =>
      debounce(async () => {
        const request = `${process.env.REACT_APP_API_BASE_URL}/etymology/${selectedItem?.item.id}`;
        if (
          !selectedLang ||
          !selectedItem ||
          selectedDescLangs.length === 0 ||
          request === lastRequest
        ) {
          return;
        }

        try {
          const response = await fetch(request);
          const tree = (await response.json()) as Etymology;
          console.log(tree);
          setLastRequest(request);
          setTree(tree);
          setTreeKind(TreeKind.Etymology);
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [
      selectedLang,
      selectedItem,
      selectedDescLangs,
      setTree,
      setTreeKind,
      lastRequest,
      setLastRequest,
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
