# ADR-0020 — Tauri capability policy and CSP

- Status: Accepted
- Date: 2026-07-26
- Wave: P0-SS-01 / P0-EX-01

## Context

The WebView boundary is the highest-value attack surface in the desktop shell.
v1.0 Architecture §24.1 requires default-deny capabilities and a strict CSP
with no `unsafe-inline` / `unsafe-eval`. CVE-2026-42184 further requires
Tauri ≥ 2.12 when that release is on crates.io.

## Decision

1. Pin `tauri = "=2.11.5"` (latest published as of 2026-07-26) and matching
   plugins in `apps/desktop/src-tauri`. Bump to ≥2.12 on publish.
2. Capability file `capabilities/default.json` identifier `prismatik-main`
   grants only `core:default`, window drag, and window-state restore.
3. Explicitly absent and must remain absent: `fs`, `http`, `process`,
   `shell:execute`. Scoped `shell:allow-open` (docs only) is deferred until
   `tauri-plugin-shell` is deliberately added.
4. CSP is set in `tauri.conf.json` with no inline script/style and no eval.

## Consequences

All filesystem and network I/O must go through registered Tauri commands in
the trusted core. Loosening this policy later is easy; tightening after
features depend on looseness is not — so we start closed.
