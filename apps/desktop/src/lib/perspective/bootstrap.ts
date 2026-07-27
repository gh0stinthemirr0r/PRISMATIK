/**
 * Perspective WASM bootstrap for Vite (Wave 2.5 / P2-EX-04).
 * Mirrors the official vite-example init_server + init_client pattern.
 */

import perspective from "@finos/perspective";
import perspectiveViewer from "@finos/perspective-viewer";
import "@finos/perspective-viewer-datagrid";
import "@finos/perspective-viewer/dist/css/pro-dark.css";

import SERVER_WASM from "@finos/perspective/dist/wasm/perspective-server.wasm?url";
import CLIENT_WASM from "@finos/perspective-viewer/dist/wasm/perspective-viewer.wasm?url";

let boot: Promise<typeof perspective> | null = null;

export async function ensurePerspective(): Promise<typeof perspective> {
  if (!boot) {
    boot = (async () => {
      await Promise.all([
        perspective.init_server(fetch(SERVER_WASM)),
        perspectiveViewer.init_client(fetch(CLIENT_WASM)),
      ]);
      return perspective;
    })();
  }
  return boot;
}

export type PerspectiveRow = Record<string, string | number | boolean | null>;
