import "./EtyButton.css";
import { TreeRequest } from "./types";
import {
  selectedLang,
  selectedItem,
  selectedDescLangs,
  selectedTreeKind,
  locationPath,
  navigate,
  debounce,
} from "../signals";

import { Ref } from "preact";
import { useMemo } from "preact/hooks";

interface EtyButtonProps {
  buttonRef: Ref<HTMLButtonElement>;
}

export default function EtyButton({ buttonRef }: EtyButtonProps) {
  const onClick = useMemo(
    () =>
      debounce(() => {
        const btn = (buttonRef as { current: HTMLButtonElement | null })
          .current;
        btn?.blur();

        const lang = selectedLang.value;
        const item = selectedItem.value;
        const descLangs = selectedDescLangs.value;
        const treeKind = selectedTreeKind.value;

        if (!lang || !item || descLangs.length === 0) return;

        const request = new TreeRequest(lang, item, descLangs, treeKind);
        const path = request.apiPath();
        if (path === locationPath.value) return;

        navigate(path);
      }, 0),
    [buttonRef]
  );

  const disabled =
    !selectedLang.value ||
    !selectedItem.value ||
    selectedDescLangs.value.length === 0;

  return (
    <button
      ref={buttonRef}
      class="ety-button"
      aria-label="search"
      disabled={disabled}
      onClick={onClick}
    >
      <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor">
        <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" />
      </svg>
    </button>
  );
}
