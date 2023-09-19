import "./Ety.css";
import Tree from "./Tree";
import Tooltip from "./Tooltip";
import { TooltipState } from "./Tooltip";
import { ExpandedItem, Item } from "../search/responses";

import { useRef, useState } from "react";

export interface EtyData {
  headProgenitorTree: ExpandedItem | null;
  selectedItem: Item | null;
}

export default function Ety(data: EtyData) {
  const [tooltipState, setTooltipState] = useState<TooltipState>({
    itemNode: null,
    svgElement: null,
    positionType: "hover",
  });
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  return (
    <div className="ety">
      <div className="tree-container">
        <Tree
          etyData={data}
          setTooltipState={setTooltipState}
          tooltipRef={tooltipRef}
          tooltipShowTimeout={tooltipShowTimeout}
          tooltipHideTimeout={tooltipHideTimeout}
        />
      </div>
      <Tooltip
        state={tooltipState}
        divRef={tooltipRef}
        showTimeout={tooltipShowTimeout}
        hideTimeout={tooltipHideTimeout}
      />
    </div>
  );
}
