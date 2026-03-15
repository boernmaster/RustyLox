//! LoxBerry Core - Common types and utilities
//!
//! This crate provides core types, traits, and utilities used across all LoxBerry crates.

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::{LoxBerryPaths, PluginPaths};
