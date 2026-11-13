import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import TrayPanel from "./TrayPanel";
import "./styles.css";

// The menu bar popup is the same bundle loaded with ?view=tray.
const isTray = new URLSearchParams(location.search).get("view") === "tray";
if (isTray) document.documentElement.classList.add("tray-window");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isTray ? <TrayPanel /> : <App />}
  </React.StrictMode>,
);
