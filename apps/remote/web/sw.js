// Service worker. The build script puts a hash of the app in BUILD and the list of files that
// make the app start in SHELL. A new build is a new cache, and the old one is dropped.
const BUILD = "__BUILD__";
const CACHE = `chxchx-${BUILD}`;
const SHELL = __SHELL__;

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => key.startsWith("chxchx-") && key !== CACHE).map((key) => caches.delete(key))),
      )
      .then(() => self.clients.claim()),
  );
});

// The app files come from the cache and are refreshed in the background. Everything else, above
// all the API (jobs, devices, tokens), always goes to the network and is never stored.
self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== location.origin || url.pathname.startsWith("/api/")) {
    return;
  }
  // What the person made their own (picture, theme): the network first, so a change shows on the next
  // load, and the last copy when there is no network.
  if (url.pathname.startsWith("/identity/")) {
    event.respondWith(
      (async () => {
        const cache = await caches.open(CACHE);
        try {
          const response = await fetch(request);
          if (response.ok) cache.put(url.origin + url.pathname, response.clone()); // one copy, however it was asked for
          return response;
        } catch {
          return (await cache.match(request, { ignoreSearch: true })) || new Response("", { status: 503 });
        }
      })(),
    );
    return;
  }
  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      const cached = await cache.match(request, { ignoreSearch: true });
      const fresh = fetch(request)
        .then((response) => {
          if (response.ok) cache.put(request, response.clone());
          return response;
        })
        .catch(() => null);
      return cached || (await fresh) || new Response("Sin conexión", { status: 503 });
    })(),
  );
});
