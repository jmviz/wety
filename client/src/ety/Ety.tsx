import { useRef } from "react";
import { ExpandedItem, ItemOption } from "../search/responses";
import { headProgenitorTreeSVG } from "./tree";

interface EtyProps {
  etyData: ExpandedItem | null;
  selectedItem: ItemOption | null;
}

function Ety({ etyData, selectedItem }: EtyProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  headProgenitorTreeSVG(svgRef, etyData, selectedItem, 12);

  return <svg ref={svgRef} />;
}

export default Ety;
