import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles/tokens.css";
import "./styles/global.css";
import "./styles/main-window.css";
import "./styles/float-window.css";
import { MainWindow } from "./App";
import { FloatWindow } from "./windows/float";

const label = getCurrentWindow().label;
// Lets float-window.css make the float webview surface transparent (the
// native window region rounds the corners; a painted body background would
// show a gray ring inside them).
document.body.dataset.window = label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "float" ? <FloatWindow /> : <MainWindow />}</React.StrictMode>,
);
