import { LangOption, ItemOption, Etymology } from "./responses";
import { TreeData, TreeKind } from "../App";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { useMemo, useRef } from "react";

interface EtyButtonProps {
  selectedLang: LangOption | null;
  selectedItem: ItemOption | null;
  selectedDescLangs: LangOption[];
  setEtyData: (data: TreeData) => void;
}

export default function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  setEtyData,
}: EtyButtonProps) {
  const lastRequest = useRef<string | null>(null);

  const onClick = useMemo(
    () =>
      debounce(async () => {
        // const currentRequest = `${
        //   process.env.REACT_APP_API_BASE_URL
        // }/headProgenitorTree/${selectedItem?.item.id}?${selectedDescLangs
        //   .map((lang) => `lang=${lang.id}`)
        //   .join("&")}`;

        const currentRequest = `${process.env.REACT_APP_API_BASE_URL}/etymology/${selectedItem?.item.id}`;

        if (
          !selectedLang ||
          !selectedItem ||
          selectedDescLangs.length === 0 ||
          lastRequest.current === currentRequest
        ) {
          return;
        }

        try {
          const response = await fetch(currentRequest);
          const tree = (await response.json()) as Etymology;
          console.log(tree);
          setEtyData({
            tree: tree,
            treeKind: TreeKind.Etymology,
            selectedItem: selectedItem.item,
            selectedDescLangs: selectedDescLangs,
          });
          lastRequest.current = currentRequest;
        } catch (error) {
          console.log(error);
        }
      }, 0),
    [selectedLang, selectedItem, selectedDescLangs, setEtyData]
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
