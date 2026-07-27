/// <reference types="@sveltejs/kit" />
/// <reference types="vite/client" />

import type { HTMLAttributes } from "svelte/elements";

declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }

  interface HTMLPerspectiveViewerElement extends HTMLElement {
    load(table: unknown): Promise<void>;
    restore(config: Record<string, unknown>): Promise<void>;
    delete(): Promise<void>;
  }

  interface HTMLElementTagNameMap {
    "perspective-viewer": HTMLPerspectiveViewerElement;
  }

  namespace svelteHTML {
    interface IntrinsicElements {
      "perspective-viewer": HTMLAttributes<HTMLPerspectiveViewerElement>;
    }
  }
}

export {};
