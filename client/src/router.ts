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
});

declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof router;
  }
}
