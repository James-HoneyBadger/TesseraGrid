//! Engine primitives: grid, boundary conditions, undo history and config.

pub mod boundary;
pub mod config;
pub mod grid;
pub mod undo;

pub use boundary::BoundaryMode;
pub use config::AppConfig;
pub use grid::Grid;
pub use undo::{Snapshot, UndoManager};
