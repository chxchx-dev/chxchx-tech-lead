// Starts the app. It lives in its own file because the page's policy forbids inline scripts.
import init from "/pkg/app.js";

init({ module_or_path: "/pkg/app_bg.wasm" }).catch((error) => {
  const main = document.getElementById("main");
  main.className = "boot";
  main.textContent = "No se pudo iniciar la app: " + error;
});

// Follows the part of the screen that is really visible. With the keyboard open, Chrome shrinks the
// page by itself, but Safari on iOS only shrinks what the person sees and scrolls the page to the
// field: the conversation then sizes itself to this visible part and sits at its top, so its header
// and the agents never slide away. (Setting properties from a script is allowed by the page's policy;
// inline style attributes are not.)
if (window.visualViewport) {
  const viewport = window.visualViewport;
  const root = document.documentElement;
  const follow = () => {
    root.style.setProperty("--vvh", viewport.height + "px");
    root.style.setProperty("--vvtop", viewport.offsetTop + "px");
    // Short means "the keyboard is up" (or a phone on its side): the tabs give way.
    root.classList.toggle("kbd-open", viewport.height < 520);
  };
  viewport.addEventListener("resize", follow);
  viewport.addEventListener("scroll", follow);
  follow();
}

// Installable and able to start without a network. The API is never served from the cache.
if ("serviceWorker" in navigator) {
  addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js", { scope: "/" }).catch(() => {});
  });
}
