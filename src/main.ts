import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import FloatWidget from "./lib/components/FloatWidget.svelte";

const target = document.getElementById("app")!;
const view = new URLSearchParams(window.location.search).get("window");

const app =
  view === "float" ? mount(FloatWidget, { target }) : mount(App, { target });

export default app;
