#!/usr/bin/env node
/**
 * NOTICE generator — Wave 1 (P1-OD-05).
 * Reads package.json workspaces + Cargo.lock name list (best-effort) and writes NOTICE.
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const lines = [
  "PRISMATIK NOTICE",
  "================",
  "",
  "Proprietary © Mythos Systems. Third-party components retain their own licenses.",
  "",
  "JavaScript / pnpm workspaces",
  "----------------------------",
];

try {
  const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  lines.push(`Root package: ${pkg.name ?? "prismatik"}`);
} catch {
  lines.push("(package.json unreadable)");
}

const lock = join(root, "pnpm-lock.yaml");
if (existsSync(lock)) {
  lines.push("See pnpm-lock.yaml for the full JS dependency graph.");
}

lines.push(
  "",
  "Rust crates",
  "-----------",
  "Workspace members under crates/ and apps/desktop/src-tauri.",
  "See Cargo.lock for the full Rust dependency graph.",
  "",
  "Notable open-source dependencies (non-exhaustive)",
  "-------------------------------------------------",
  "- Tauri 2 (Apache-2.0 / MIT)",
  "- SvelteKit (MIT)",
  "- Lightweight Charts (Apache-2.0)",
  "- rusqlite / SQLite (blessing / public domain)",
  "- Apache Arrow / Parquet (Apache-2.0)",
  "- governor (Apache-2.0 / MIT)",
  "- reqwest / rustls (Apache-2.0 / MIT / ISC)",
  "",
  `Generated: ${new Date().toISOString()}`,
  "",
);

writeFileSync(join(root, "NOTICE"), lines.join("\n"), "utf8");
console.log("Wrote NOTICE");
