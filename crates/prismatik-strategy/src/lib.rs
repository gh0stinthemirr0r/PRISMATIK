//! # prismatik-prismatik-strategy
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — strategy contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use ir::{SchemaVersion, StrategyCapabilities, StrategyIR, StrategyIRError};
pub use runtime::{RuntimeError, StrategyRuntime};
pub use trait_def::{ExecutionContext, MarketEvent, OrderIntent, Strategy, StrategyContext};

pub mod codegen;
pub mod dsl;
pub mod ir;
pub mod runtime;
pub mod trait_def;

pub mod context;
pub mod data_access;
pub mod strategy;
pub mod strategy_ir;

pub use strategy_ir::STRATEGY_IR_SCHEMA_VERSION;
