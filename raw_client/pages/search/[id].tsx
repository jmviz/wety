import { useRouter } from "next/router";
import { FC } from "react";
import useSWR from "swr";
import Tree from "../../src/components/Tree";
import SearchBar from "../../src/components/SearchBar";
import { TreeNodeData, ResponseData } from "../../src/types";

const fetcher = async (url: string): Promise<TreeNodeData | null> => {
  console.log("Fetching from network:", url);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error("Search failed");
  }
  const data: ResponseData = await response.json();
  return data.length > 0 ? data[0] : null;
};

const SearchPage: FC = () => {
  const router = useRouter();

  const id = Array.isArray(router.query.id)
    ? router.query.id[0]
    : router.query.id ?? "";

  const { data, error, isLoading } = useSWR(
    router.isReady && id ? `/api/search?id=${encodeURIComponent(id)}` : null,
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
};

export default SearchPage;
