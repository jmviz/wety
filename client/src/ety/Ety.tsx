import "./Ety.css";
import { Item } from "../search/responses";
import { EtyData, treeSVG } from "./tree";

import { RefObject, useRef } from "react";

interface EtyProps {
  data: EtyData;
  containerRef: RefObject<HTMLDivElement>;
  setTooltipItem: (item: Item | null) => void;
  // tooltipRef: RefObject<HTMLDivElement>;
}

function Ety({ data, containerRef, setTooltipItem }: EtyProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  const fontSize = containerRef.current
    ? parseFloat(window.getComputedStyle(containerRef.current).fontSize)
    : 13;

  treeSVG(svgRef, data, fontSize);

  return <svg className="tree" ref={svgRef} />;
}

export default Ety;
