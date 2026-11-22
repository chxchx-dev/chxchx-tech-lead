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

// The browser bar takes the colour of the chosen theme (the same background the page uses), also
// when the person changes it: the identity style sheet is read again and the colour follows.
const paintBar = () => {
  const bg = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
  const metas = document.querySelectorAll('meta[name="theme-color"]');
  if (!bg || (metas.length === 1 && metas[0].content === bg)) return;
  metas.forEach((meta) => meta.remove());
  const meta = document.createElement("meta");
  meta.name = "theme-color";
  meta.content = bg;
  document.head.appendChild(meta);
};
addEventListener("load", paintBar);
new MutationObserver(paintBar).observe(document.head, { childList: true, subtree: true, attributeFilter: ["href"], attributes: true });
window.chxchxPaintBar = paintBar;

// Brings the PC's current theme in while the page stays as it is: the new sheet loads next to the
// old one, and only when it is ready does it take its place (no flash of unstyled page).
window.chxchxReloadTheme = () =>
  new Promise((done) => {
    const old = document.querySelector('link[href^="/identity/theme.css"]');
    if (!old) return done();
    const fresh = document.createElement("link");
    fresh.rel = "stylesheet";
    fresh.href = "/identity/theme.css?v=" + Date.now();
    const finish = () => {
      old.remove();
      paintBar();
      done();
    };
    fresh.onload = finish;
    fresh.onerror = () => {
      fresh.remove();
      done();
    };
    old.after(fresh);
  });

// Installable and able to start without a network. The API is never served from the cache.
if ("serviceWorker" in navigator) {
  addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js", { scope: "/" }).catch(() => {});
  });
}
