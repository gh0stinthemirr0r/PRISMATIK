# ADR-0031 — tauri-specta deferred behind MSRV

- Status: Accepted (temporary)
- Date: 2026-07-26
- Wave: P0-EX-02

## Context

Wave 0 requires a `tauri-specta` bindings pipeline with exact pins and a
`bindings-drift` CI gate. Specta / tauri-specta `2.0.0-rc.25` does not compile
on Rust 1.88 (uses unstable `debug_closure_helpers`).

## Decision

1. Ship `packages/api-client` with a hand-maintained *scaffold* bindings file
   and the drift-check scripts.
2. Do not compile `tauri-specta` into `prismatik-desktop` until either:
   - Specta publishes a crate that builds on MSRV 1.88, or
   - An ADR bumps MSRV to a toolchain that builds Specta.
3. CI `bindings-drift` still runs against the generated directory so the gate
   exists; regeneration becomes automatic once Specta is re-enabled.

## Consequences

TypeScript callers use the scaffold until the first successful Specta export.
This is tracked debt, not a permanent exception.
