import { LangOption, ItemOption, Etymology } from "./responses";
import { TreeData, TreeKind } from "../App";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { useMemo } from "react";

interface EtyButtonProps {
  selectedLang: LangOption | null;
  selectedItem: ItemOption | null;
  selectedDescLangs: LangOption[];
  setTreeData: (data: TreeData) => void;
  lastRequest: string | null;
  setLastRequest: (request: string | null) => void;
}

export default function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  setTreeData,
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
          setTreeData({
            tree: tree,
            treeKind: TreeKind.Etymology,
            selectedItem: selectedItem.item,
            selectedLang: selectedLang,
            selectedDescLangs: selectedDescLangs,
          });
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [
      selectedLang,
      selectedItem,
      selectedDescLangs,
      setTreeData,
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
