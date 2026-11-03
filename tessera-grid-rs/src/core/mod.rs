//! Engine primitives: grid, boundary conditions, undo history, measures and config.

pub mod analysis;
pub mod boundary;
pub mod config;
pub mod grid;
pub mod undo;

pub use boundary::BoundaryMode;
pub use config::AppConfig;
pub use grid::{Grid, NeighborBuffers, PARALLEL_CELL_THRESHOLD};
pub use undo::{Snapshot, UndoManager};
