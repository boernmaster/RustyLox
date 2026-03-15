//! Plugin Manager - Plugin lifecycle management
//!
//! This crate handles:
//! - Plugin database management
//! - ZIP extraction
//! - plugin.cfg parsing
//! - Lifecycle hook execution
//! - Directory isolation

pub mod database;
pub mod installer;
pub mod lifecycle;
pub mod config_parser;
pub mod directory_manager;

pub use database::{PluginDatabase, PluginEntry};
pub use installer::{PluginInstaller, InstallRequest, InstallAction};
pub use lifecycle::{LifecycleManager, LifecycleHook};
pub use config_parser::PluginConfig;
pub use directory_manager::DirectoryManager;
