import "./Tooltip.css";
import { Item } from "../search/responses";

import { RefObject } from "react";

interface TooltipProps {
  item: Item | null;
  ref: RefObject<HTMLDivElement>;
}

function Tooltip({ item, ref }: TooltipProps) {
  return (
    <div className="tooltip" ref={ref}>
      {item?.term}
    </div>
  );
}

export default Tooltip;
