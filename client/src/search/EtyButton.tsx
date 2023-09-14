import { LangOption, ItemOption, ExpandedItem } from "./responses";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { useMemo } from "react";

interface EtyButtonProps {
  selectedLang: LangOption | null;
  selectedItem: ItemOption | null;
  selectedDescLangs: LangOption[];
  setEtyData: (data: ExpandedItem | null) => void;
}

function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  setEtyData,
}: EtyButtonProps) {
  const onClick = useMemo(
    () =>
      debounce(async () => {
        if (!selectedLang || !selectedItem || selectedDescLangs.length === 0) {
          return;
        }
        try {
          const response = await fetch(
            `${process.env.REACT_APP_API_BASE_URL}/headProgenitorTree/${
              selectedItem.item.id
            }?${selectedDescLangs.map((lang) => `lang=${lang.id}`).join("&")}`
          );
          const etyData = (await response.json()) as ExpandedItem;
          setEtyData(etyData);
        } catch (error) {
          console.log(error);
        }
      }, 500),
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

export default EtyButton;
