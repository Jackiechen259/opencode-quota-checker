import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles/tokens.css";
import "./styles/global.css";
import "./styles/main-window.css";
import "./styles/float-window.css";
import { MainWindow } from "./App";
import { FloatWindow } from "./windows/FloatWindow";

const label = getCurrentWindow().label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "float" ? <FloatWindow /> : <MainWindow />}</React.StrictMode>,
);
