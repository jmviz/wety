import { FC } from "react";
import styles from "../styles/Tree.module.css";
import { TreeNodeData } from "../types";

function getLangDistanceClass(distance: number | null): string {
  if (distance === null) return styles.langUnrelated;
  if (distance < 0) return styles.langDistance0;
  if (distance > 11) return styles.langDistance11;
  return styles[`langDistance${distance}`];
}

interface TreeNodeProps {
  data: TreeNodeData;
}

const TreeNode: FC<TreeNodeProps> = ({ data }) => {
  const langDistanceClass = getLangDistanceClass(data.langDistance);

  return (
    <li className={langDistanceClass}>
      <details open>
        <summary>
          <span className={styles.lang}>{data.item.lang.name}</span>{" "}
          <span className={styles.term}>{data.item.term}</span>{" "}
          <span className={styles.romanization}>
            {data.item.romanization ? `(${data.item.romanization})` : ""}
          </span>
        </summary>
        {data.children && data.children.length > 0 && (
          <ul>
            {data.children.map((child, index) => (
              <TreeNode key={index} data={child} />
            ))}
          </ul>
        )}
      </details>
    </li>
  );
};

interface TreeProps {
  data: TreeNodeData | null;
}

const Tree: FC<TreeProps> = ({ data }) => {
  return (
    <div id="tree-container">
      {data ? (
        <ul className={styles.tree}>
          <TreeNode data={data} />
        </ul>
      ) : (
        <p>Loading...</p>
      )}
    </div>
  );
};

export default Tree;
