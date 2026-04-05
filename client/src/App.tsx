import SearchPane from "./search/SearchPane";
import { TreeKind } from "./search/types";
import EtymologyTree from "./ety/EtymologyTree";
import DescendantsTree from "./ety/DescendantsTree";
import SettingsSidebar from "./settings/SettingsSidebar";
import { filteredTree, lastRequest, loadFromPath } from "./state";

import { Show, createEffect } from "solid-js";
import { useLocation } from "@tanstack/solid-router";

export default function App() {
  const location = useLocation();

  createEffect(() => {
    const path = location().pathname + location().search;
    if (path === "/") return;
    loadFromPath(path);
  });

  return (
    <>
      <SettingsSidebar />
      <SearchPane />
      <Show when={lastRequest()?.kind === TreeKind.Etymology}>
        <EtymologyTree tree={filteredTree()} />
      </Show>
      <Show when={lastRequest() && lastRequest()?.kind !== TreeKind.Etymology}>
        <DescendantsTree tree={filteredTree()} />
      </Show>
    </>
  );
}
