# Desktop app — Wave 2.5 notes

## Perspective (`P2-EX-04`)

`@finos/perspective` **3.8.0**, `@finos/perspective-viewer`, and `@finos/perspective-viewer-datagrid` install cleanly on the current pnpm/Vite/SvelteKit toolchain.

Integration follows the official Vite WASM bootstrap (`init_server` + `init_client` with `?url` assets) in `src/lib/perspective/`. Live widgets:

- Filings → 13F ownership datagrid
- Options → chain explorer + flow prints datagrids

If a future toolchain break prevents install, fall back to `@prismatik/ui` `VirtualList` dense grids and record the failure here.

## IV surface (`P3-EX-02` deepen)

Volatility lab prefers **WebGPU** in the Tauri webview (`navigator.gpu`) with a structured **Canvas term×strike mesh** fallback (axes + color scale). Feature flag: `VITE_IV_SURFACE_FORCE_CANVAS=1`.

Native `wgpu` via `crates/prismatik-renderer` remains deferred (stub only); no new Tauri GPU commands in this wave.
