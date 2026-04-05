import SearchPane from "./search/SearchPane";
import { TreeKind } from "./search/types";
import EtymologyTree from "./ety/EtymologyTree";
import DescendantsTree from "./ety/DescendantsTree";
import SettingsSidebar from "./settings/SettingsSidebar";
import { filteredTree, lastRequest, loadFromPath, locationPath } from "./signals";

import { useEffect } from "preact/hooks";

export default function App() {
  useEffect(() => {
    const path = locationPath.value;
    if (path === "/") return;
    loadFromPath(path);
  }, [locationPath.value]);

  return (
    <>
      <SettingsSidebar />
      <SearchPane />
      {lastRequest.value?.kind === TreeKind.Etymology ? (
        <EtymologyTree tree={filteredTree.value} />
      ) : (
        <DescendantsTree tree={filteredTree.value} />
      )}
    </>
  );
}
