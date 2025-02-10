import { useRouter } from "next/router";
import React from "react";
import useSWR from "swr";
import Tree from "../../src/components/Tree";
import SearchBar from "../../src/components/SearchBar";

const fetcher = async (url) => {
  console.log("Fetching from network:", url);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error("Search failed");
  }
  const data = await response.json();
  return data.length > 0 ? data[0] : null;
};

export default function SearchPage() {
  const router = useRouter();
  const { id } = router.query;

  const { data, error, isLoading } = useSWR(
    id ? `/api/search?id=${encodeURIComponent(id)}` : null,
    fetcher,
    {
      keepPreviousData: true,
    }
  );

  return (
    <div>
      <SearchBar initialValue={id} />
      <div>
        {isLoading && <div>Loading...</div>}
        {error && <div>Error: {error.message}</div>}
        {!isLoading && !error && !data && <div>No results found</div>}
        {data && <Tree data={data} />}
      </div>
    </div>
  );
}
