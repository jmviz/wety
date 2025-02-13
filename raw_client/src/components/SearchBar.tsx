import { FC, useState, FormEvent, ChangeEvent } from "react";
import { useRouter } from "next/router";
import styles from "../styles/SearchBar.module.css";

interface SearchBarProps {
  initialValue?: string;
}

const SearchBar: FC<SearchBarProps> = ({ initialValue = "" }) => {
  const router = useRouter();
  const [searchTerm, setSearchTerm] = useState(initialValue);

  const handleSearch = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (searchTerm.trim()) {
      router.push(`/search/${encodeURIComponent(searchTerm)}`, undefined, {
        shallow: true,
      });
    }
  };

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(e.target.value);
  };

  return (
    <div className={styles.searchContainer}>
      <form onSubmit={handleSearch} className={styles.searchForm}>
        <input
          type="text"
          value={searchTerm}
          onChange={handleChange}
          placeholder="Search for a word..."
          className={styles.searchInput}
        />
      </form>
    </div>
  );
};

export default SearchBar;
