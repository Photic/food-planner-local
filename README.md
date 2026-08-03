# Food Planner

A household food planner — recipes, a weekly meal plan, a generated shopping
list and a pantry — served from one machine on the local network and installable
as a PWA on any phone, tablet or laptop in the house.

## Architecture

Dioxus fullstack: the UI compiles to WebAssembly and runs in the browser, and
the same crate compiles again natively into an Axum server that renders the
first page, answers the server functions and owns the SQLite file.

There is no Tauri here on purpose. Tauri produces a native desktop binary for
one machine; it cannot serve a PWA to other devices on the network, which is the
thing that makes this usable from a phone in the kitchen.

```text
├─ index.html      # Custom shell: PWA manifest link, iOS meta tags, SW registration
├─ Dioxus.toml     # asset_dir = "public"
├─ migrations/     # SQL schema, applied automatically on first run
├─ public/         # Copied verbatim to the web root — manifest, service worker, icons
├─ assets/         # Fingerprinted by the asset!() macro — stylesheet, favicon
└─ src/
   ├─ main.rs      # Routes and app root
   ├─ models.rs    # Types shared across the client/server boundary
   ├─ api.rs       # Server functions
   ├─ camera.rs    # Device camera; the only module that touches web-sys
   ├─ db.rs        # SQLite pool and migrations (server only)
   └─ views/       # One module per page
```

Note that `asset_dir` puts `public/` at the **web root**, not under `/assets/`:
`public/sw.js` is served as `/sw.js`. This matters — a service worker only
controls the scope it is served from, so a worker under `/assets/` could not
control the app. (The upstream Dioxus PWA example uses `/assets/` paths and is
broken for this reason.)

## Running it

Requires the Dioxus CLI (`cargo binstall dioxus-cli`) and the wasm target
(`rustup target add wasm32-unknown-unknown`).

Bound to localhost, for development:

```bash
dx serve
```

Bound to every interface, so the rest of the network can reach it:

```bash
dx serve --addr 0.0.0.0 --port 8080
```

Then open `http://<this-machine-ip>:8080` from any device on the network. On
iOS use Share → Add to Home Screen; on Android Chrome offers an install prompt.

> **Service workers and the camera need a secure context.** Browsers treat
> `localhost` as secure but a plain-HTTP LAN address is not, so the offline
> caching, the Android install prompt and the in-app viewfinder all stay
> inactive over `http://192.168.x.x`. The app itself works fine, photos can
> still be added (see below), and iOS will still add it to the home screen. To
> get the full behaviour on other devices, put it behind HTTPS — a reverse proxy
> with a certificate from your own CA, or a hostname from a service like
> Tailscale.

For a release build:

```bash
dx build --release --platform web
```

## Photos

A recipe can carry one photo, added when it is written down or attached to it
later, and there are two ways to provide one:

- **Take photo** opens a viewfinder in the page and captures a frame. This needs
  `getUserMedia`, so the button only appears in a secure context — on the
  machine running `dx serve`, or behind HTTPS. It is absent over a plain-HTTP
  LAN address, which is the usual case for a phone in the kitchen.
- **The file input** is always present. On a phone it offers the camera through
  the operating system as well as the photo library, and it works over plain
  HTTP because the handoff is the OS's rather than a browser API. This is what
  makes photos usable on the network without HTTPS.

Frames captured through the viewfinder are stored at the camera's own
resolution, encoded as JPEG. Files chosen from the device are stored exactly as
they were picked. Neither is scaled down, so the database grows roughly by the
size of the originals — `api::MAX_PHOTO_BYTES` caps a single photo at 15 MB.

## Data

SQLite, in `food-planner.db` in the working directory. Override the location
with `FOOD_PLANNER_DB`. Migrations in `migrations/` run automatically when the
first query opens the pool. Back it up by copying the file.

## Status

- **Recipes** — working end to end: list, create with ingredients, delete, and
  photos by camera or file on both new and existing recipes.
- **Weekly plan**, **shopping list**, **pantry** — schema in place, UI still
  stubbed out.
