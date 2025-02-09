import React, { useState } from "react";
import { useRouter } from "next/router";

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
    <form onSubmit={handleSearch}>
      <input
        type="text"
        value={searchTerm}
        onChange={(e) => setSearchTerm(e.target.value)}
        placeholder="Search for a word..."
      />
      <button type="submit">Search</button>
    </form>
  );
}
