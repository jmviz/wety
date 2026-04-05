import { TreeKind } from "./types";
import { selectedTreeKind, setSelectedTreeKind } from "../state";

import { For } from "solid-js";
import { Select, createListCollection } from "@ark-ui/solid";

const treeKindCollection = createListCollection({
  items: Object.values(TreeKind),
  itemToString: (item) => item,
  itemToValue: (item) => item,
});

export default function TreeKindSelect() {
  return (
    <Select.Root
      collection={treeKindCollection}
      value={[selectedTreeKind()]}
      onValueChange={(details) => {
        if (details.value[0]) {
          setSelectedTreeKind(details.value[0] as TreeKind);
        }
      }}
    >
      <Select.Label>Mode</Select.Label>
      <Select.Control>
        <Select.Trigger>
          <Select.ValueText placeholder="Select mode" />
        </Select.Trigger>
      </Select.Control>
      <Select.Positioner>
        <Select.Content>
          <For each={treeKindCollection.items}>
            {(item) => (
              <Select.Item item={item}>
                <Select.ItemText>{item}</Select.ItemText>
              </Select.Item>
            )}
          </For>
        </Select.Content>
      </Select.Positioner>
    </Select.Root>
  );
}
