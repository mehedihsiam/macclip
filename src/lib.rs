//! MacClip — clipboard history for macOS.
//!
//! The core (config, privacy, store) is platform independent and unit tested;
//! the AppKit integration lives in the `mac` modules used by the binary.

pub mod config;
pub mod install;
pub mod monitor;
pub mod privacy;
pub mod store;
