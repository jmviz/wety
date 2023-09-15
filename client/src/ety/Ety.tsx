import "./Ety.css";
import { EtyData, treeSVG } from "./tree";

import { RefObject, useRef } from "react";

interface EtyProps {
  data: EtyData;
  containerRef: RefObject<HTMLDivElement>;
}

function Ety({ data, containerRef }: EtyProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  const fontSize = containerRef.current
    ? parseFloat(window.getComputedStyle(containerRef.current).fontSize)
    : 13;

  treeSVG(svgRef, data, fontSize);

  return <svg ref={svgRef} />;
}

export default Ety;
