import { Lang, Item, TreeRequest } from "./types";
import { TreeKind } from "./types";

import Button from "@mui/material/Button";
import SearchIcon from "@mui/icons-material/Search";
import { debounce } from "@mui/material/utils";
import { RefObject, useMemo } from "react";
import { useNavigate, useLocation } from "react-router-dom";

interface EtyButtonProps {
  selectedLang: Lang | null;
  selectedItem: Item | null;
  selectedDescLangs: Lang[];
  selectedTreeKind: TreeKind;
  buttonRef: RefObject<HTMLButtonElement>;
}

export default function EtyButton({
  selectedLang,
  selectedItem,
  selectedDescLangs,
  buttonRef,
  selectedTreeKind,
}: EtyButtonProps) {
  const navigate = useNavigate();
  const location = useLocation();

  const onClick = useMemo(
    () =>
      debounce(() => {
        buttonRef.current?.blur();

        if (!selectedLang || !selectedItem || selectedDescLangs.length === 0) {
          return;
        }

        const request = new TreeRequest(
          selectedLang,
          selectedItem,
          selectedDescLangs,
          selectedTreeKind
        );
        const path = request.apiPath();
        if (path === location.pathname + location.search) {
          return;
        }

        navigate(path);
      }, 0),
    [
      buttonRef,
      selectedLang,
      selectedItem,
      selectedDescLangs,
      selectedTreeKind,
      navigate,
      location,
    ]
  );

  return (
    <Button
      ref={buttonRef}
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
