import "./Ety.css";
import Tree from "./Tree";
import Tooltip from "./Tooltip";
import { ExpandedItem, Item } from "../search/responses";

import { HierarchyPointNode } from "d3";
import { useRef, useState } from "react";

interface EtyProps {
  data: EtyData;
}

export default function Ety({ data }: EtyProps) {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [tooltipItemNode, setTooltipItemNode] =
    useState<HierarchyPointNode<ExpandedItem> | null>(null);
  const [positionType, setPositionType] = useState<string>("hover");
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  return (
    <div className="ety">
      <div className="tree-container">
        <Tree
          etyData={data}
          tooltipRef={tooltipRef}
          setTooltipItem={setTooltipItemNode}
          setPositionType={setPositionType}
          tooltipShowTimeout={tooltipShowTimeout}
          tooltipHideTimeout={tooltipHideTimeout}
        />
      </div>
      <Tooltip
        itemNode={tooltipItemNode}
        positionType={positionType}
        divRef={tooltipRef}
        showTimeout={tooltipShowTimeout}
        hideTimeout={tooltipHideTimeout}
      />
    </div>
  );
}

export interface EtyData {
  headProgenitorTree: ExpandedItem | null;
  selectedItem: Item | null;
}
