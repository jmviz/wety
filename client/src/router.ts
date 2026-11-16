import {
  createRouter,
  createRootRoute,
  createRoute,
} from "@tanstack/solid-router";
import App from "./App";
import { VIEW_ORDER, kindPath } from "./search/types";

const rootRoute = createRootRoute({
  component: App,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
});

const aboutRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/about",
});

// `/<view>/<lang code>/<term>` for each view in `VIEWS`; the term is a splat
// because it may contain "/". App reads the location itself (see
// `parseWordHref`), so these routes only need to match.
const wordRoutes = VIEW_ORDER.map((kind) =>
  createRoute({
    getParentRoute: () => rootRoute,
    path: `/${kindPath(kind)}/$lang/$`,
  })
);

const routeTree = rootRoute.addChildren([indexRoute, aboutRoute, ...wordRoutes]);

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
