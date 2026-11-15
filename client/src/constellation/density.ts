export interface DensityNode {
  id: string;
  width: number;
  compact: boolean;
  children: DensityNode[];
}

// Decide from ordinary card widths, not the already compressed layout,
// so resizing cannot alternate between expanding and shrinking the cards.
export function compactBranches(roots: DensityNode[], availableWidth: number): Set<string> {
  const selected = new Set<string>();
  const span = (node: DensityNode): number => node.width + Math.max(0, ...node.children.map((c) => 44 + span(c)));
  const depth = (node: DensityNode): number => 1 + Math.max(0, ...node.children.map(depth));
  const select = (node: DensityNode) => {
    if (node.compact) selected.add(node.id);
    node.children.forEach(select);
  };
  for (const root of roots) {
    if (depth(root) >= 4 || span(root) > Math.max(800, availableWidth)) select(root);
  }
  return selected;
}
