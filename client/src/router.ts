import {
  createRouter,
  createRootRoute,
  createRoute,
} from "@tanstack/solid-router";
import App from "./App";

const rootRoute = createRootRoute({
  component: App,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
});

const etymologyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/etymology/$itemId",
});

type TreeSearchParams = {
  distLang: number;
  descLang: number[];
};

function validateTreeSearch(search: Record<string, unknown>): TreeSearchParams {
  const raw = search.descLang;
  let descLang: number[];
  if (Array.isArray(raw)) {
    descLang = raw.map(Number);
  } else if (raw !== undefined && raw !== null) {
    descLang = [Number(raw)];
  } else {
    descLang = [];
  }
  return {
    distLang: Number(search.distLang) || 0,
    descLang,
  };
}

const descendantsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/descendants/$itemId",
  validateSearch: validateTreeSearch,
});

const cognatesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/cognates/$itemId",
  validateSearch: validateTreeSearch,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  etymologyRoute,
  descendantsRoute,
  cognatesRoute,
]);

export const router = createRouter({
  routeTree,
  defaultPreload: false,
  stringifySearch: (search: Record<string, unknown>) => {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(search)) {
      if (Array.isArray(value)) {
        for (const v of value) {
          params.append(key, String(v));
        }
      } else if (value !== undefined && value !== null) {
        params.set(key, String(value));
      }
    }
    const str = params.toString();
    return str ? `?${str}` : "";
  },
  parseSearch: (searchStr: string) => {
    if (searchStr.startsWith("?")) searchStr = searchStr.substring(1);
    const params = new URLSearchParams(searchStr);
    const result: Record<string, unknown> = {};
    const seen = new Set<string>();
    for (const key of params.keys()) {
      if (seen.has(key)) continue;
      seen.add(key);
      const all = params.getAll(key);
      result[key] = all.length > 1 ? all : all[0];
    }
    return result;
  },
});

declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof router;
  }
}
