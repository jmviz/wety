import "../src/styles/site.css";
import { SWRConfig } from "swr";

function App({ Component, pageProps }) {
  return (
    <SWRConfig
      value={{
        revalidateIfStale: false,
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
        onSuccess: (data, key) => {
          console.log("SWR data received:", key);
        },
        onLoadingSlow: (key) => {
          console.log("SWR loading from network:", key);
        },
        loadingTimeout: 100,
      }}
    >
      <Component {...pageProps} />
    </SWRConfig>
  );
}

export default App;
