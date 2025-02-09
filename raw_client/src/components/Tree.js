import React from "react";
import styles from "../styles/Tree.module.css";

function getLangDistanceClass(distance) {
  if (distance === null) return styles.langUnrelated;
  if (distance < 0) return styles.langDistance0;
  if (distance > 11) return styles.langDistance11;
  return styles[`langDistance${distance}`];
}

const TreeNode = ({ data }) => {
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

const Tree = ({ data }) => {
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
