//! Tessera Grid — engine and UI for a cellular-automaton simulator.
//!
//! Everything except [`main`](../main.rs) lives here so that the engine can be
//! driven directly from unit tests, integration tests and benchmarks instead of
//! only through the `tessera` binary.
//!
//! The simulation itself ([`sim`], [`core`], [`automata`], [`rle`], [`export`])
//! has no UI dependency; [`app`] and [`cli`] are the two front-ends.

pub mod app;
pub mod automata;
pub mod cli;
pub mod core;
pub mod export;
pub mod patterns;
pub mod rle;
pub mod sim;
pub mod theme;
