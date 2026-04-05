import { Etymology, InterLangDescendants } from "../search/types";

export function filterEtymologyTree(
  tree: Etymology,
  disabledModes: Set<string>
): Etymology {
  if (disabledModes.size === 0) return tree;
  return {
    ...tree,
    parents: tree.parents
      .filter((p) => !p.etyMode || !disabledModes.has(p.etyMode))
      .map((p) => filterEtymologyTree(p, disabledModes)),
  };
}

export function filterDescendantsTree(
  tree: InterLangDescendants,
  disabledModes: Set<string>
): InterLangDescendants {
  if (disabledModes.size === 0) return tree;
  return {
    ...tree,
    children: tree.children
      .filter((c) => !c.etyMode || !disabledModes.has(c.etyMode))
      .map((c) => filterDescendantsTree(c, disabledModes)),
  };
}
