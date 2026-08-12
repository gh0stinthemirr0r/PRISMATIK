/**
 * Svelte action that applies style properties through CSSOM
 * (`el.style.setProperty`), which the strict Tauri CSP
 * (`style-src 'self'`, no inline style attributes) permits.
 */

import type { Action } from 'svelte/action';

function toCssName(prop: string): string {
  return prop.startsWith('--') ? prop : prop.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());
}

export const cssProps: Action<HTMLElement, Record<string, string>> = (node, props) => {
  const apply = (p: Record<string, string>) => {
    for (const [k, v] of Object.entries(p)) node.style.setProperty(toCssName(k), v);
  };
  apply(props);
  return { update: apply };
};
