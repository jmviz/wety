import { useState } from "react";
import { useRouter } from "next/router";
import styles from "../styles/SearchBar.module.css";

export default function SearchBar({ initialValue = "" }) {
  const router = useRouter();
  const [searchTerm, setSearchTerm] = useState(initialValue);

  const handleSearch = (e) => {
    e.preventDefault();
    if (searchTerm.trim()) {
      router.push(`/search/${encodeURIComponent(searchTerm)}`, undefined, {
        shallow: true,
      });
    }
  };

  return (
    <div className={styles.searchContainer}>
      <form onSubmit={handleSearch} className={styles.searchForm}>
        <input
          type="text"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          placeholder="Search for a word..."
          className={styles.searchInput}
        />
      </form>
    </div>
  );
}
