import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Global error handlers for diagnosing white screen issues
window.addEventListener("error", (event) => {
  console.error("[Global Error]", event.error?.message || event.message, event.error?.stack);
  const div = document.createElement("div");
  div.id = "xhs-global-error";
  div.style.cssText = "position:fixed;top:0;left:0;right:0;z-index:9999;background:#fee;color:#c00;padding:16px;font-size:14px;font-family:monospace;border-bottom:2px solid #c00;max-height:200px;overflow:auto;";
  div.textContent = `[JS Error] ${event.error?.message || event.message}`;
  document.body.prepend(div);
});

window.addEventListener("unhandledrejection", (event) => {
  console.error("[Unhandled Promise]", event.reason);
  const div = document.createElement("div");
  div.id = "xhs-promise-error";
  div.style.cssText = "position:fixed;top:0;left:0;right:0;z-index:9999;background:#fee;color:#c00;padding:16px;font-size:14px;font-family:monospace;border-bottom:2px solid #c00;max-height:200px;overflow:auto;";
  div.textContent = `[Promise Error] ${event.reason?.message || String(event.reason)}`;
  document.body.prepend(div);
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
