"use strict";

// Bump this when the caching rules below change. The activate handler deletes
// every cache that does not match, which is what evicts the previous version.
const VERSION = "food-planner-v1";

// The shell is fetched on install so the app opens without a network round
// trip. Hashed wasm/js/css filenames are not listed here because they change
// on every build; the fetch handler picks them up as they are requested.
const SHELL = ["/", "/manifest.json", "/icon-192.png"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(VERSION)
      .then((cache) => cache.addAll(SHELL))
      // A missing shell entry must not block installation, or a single 404
      // leaves the app with no worker at all.
      .catch((err) => console.warn("Shell precache incomplete:", err))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => key !== VERSION).map((key) => caches.delete(key)))
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;

  // Only GET is cacheable. Writes fall through to the network untouched.
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);

  // Requests to other hosts are none of this worker's business.
  if (url.origin !== self.location.origin) {
    return;
  }

  // Server functions must never be answered from cache: a stale recipe list
  // that silently overwrites what another device just added is worse than an
  // error. These fail outright when offline, which the UI reports.
  if (url.pathname.startsWith("/api/")) {
    return;
  }

  // Everything else — the HTML shell, wasm, js, css, icons — is network-first
  // so a rebuilt app is picked up immediately, with the cache as the offline
  // fallback. On a LAN the extra round trip is negligible.
  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok) {
          const copy = response.clone();
          caches.open(VERSION).then((cache) => cache.put(request, copy));
        }
        return response;
      })
      .catch(() =>
        caches.match(request).then((cached) => {
          if (cached) {
            return cached;
          }

          // A client-side route like /planner was never cached under its own
          // URL, so fall back to the shell and let the router sort it out.
          if (request.mode === "navigate") {
            return caches.match("/");
          }

          return new Response("Offline and not cached", {
            status: 503,
            statusText: "Service Unavailable",
            headers: new Headers({ "Content-Type": "text/plain" }),
          });
        })
      )
  );
});
