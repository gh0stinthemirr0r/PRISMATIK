//! # prismatik-prismatik-renderer
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — renderer contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

/// Surface kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// Volatility surface.
    Volatility,
    /// Monte Carlo path cloud.
    MonteCarloPaths,
    /// Correlation matrix.
    CorrelationMatrix,
}

/// Render backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderBackend {
    /// GPU backend.
    Gpu,
    /// CPU fallback backend.
    CpuFallback,
}

/// Render request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderRequest {
    /// Surface kind.
    pub kind: SurfaceKind,
    /// Backend preference.
    pub backend: RenderBackend,
}

/// Renderer contract.
pub trait Renderer: Send + Sync {
    /// Render a surface.
    fn render(&self, request: &RenderRequest) -> Result<(), String>;
}
