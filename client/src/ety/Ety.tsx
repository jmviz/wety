import "./Ety.css";
import Tree from "./Tree";
import Tooltip from "./Tooltip";
import { ExpandedItem, Item } from "../search/responses";

import { useRef, useState } from "react";

interface EtyProps {
  data: EtyData;
}

export default function Ety({ data }: EtyProps) {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [tooltipItem, setTooltipItem] = useState<Item | null>(null);
  const tooltipShowTimeout = useRef<number | null>(null);
  const tooltipHideTimeout = useRef<number | null>(null);

  return (
    <div className="ety">
      <Tree
        etyData={data}
        tooltipRef={tooltipRef}
        setTooltipItem={setTooltipItem}
        tooltipShowTimeout={tooltipShowTimeout}
        tooltipHideTimeout={tooltipHideTimeout}
      />
      {/* <Tooltip
        item={tooltipItem}
        ref={tooltipRef}
        showTimeout={tooltipShowTimeout}
        hideTimeout={tooltipHideTimeout}
      /> */}
    </div>
  );
}

export interface EtyData {
  headProgenitorTree: ExpandedItem | null;
  selectedItem: Item | null;
}
