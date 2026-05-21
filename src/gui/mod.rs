// src/gui/mod.rs
//! GUI module for EBSS visualization.
//!
//! This module provides an interactive graphical interface using egui/eframe.
//! It runs the simulation in a separate thread and communicates via channels.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin ebss_gui --features gui
//! ```

pub mod state;
pub mod snapshot;
pub mod app;
pub mod panels;

pub use state::*;
pub use snapshot::*;
pub use app::EbssApp;
