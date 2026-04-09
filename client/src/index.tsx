import "./index.scss";
import { render } from "solid-js/web";
import { RouterProvider } from "@tanstack/solid-router";
import { router } from "./router";

render(() => <RouterProvider router={router} />, document.getElementById("root")!);
