// Starts the app. It lives in its own file because the page's policy forbids inline scripts.
import init from "/pkg/app.js";

init({ module_or_path: "/pkg/app_bg.wasm" }).catch((error) => {
  const main = document.getElementById("main");
  main.className = "boot";
  main.textContent = "No se pudo iniciar la app: " + error;
});

// Installable and able to start without a network. The API is never served from the cache.
if ("serviceWorker" in navigator) {
  addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js", { scope: "/" }).catch(() => {});
  });
}
