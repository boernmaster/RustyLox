//! Plugin Manager - Plugin lifecycle management
//!
//! This crate handles:
//! - Plugin database management
//! - ZIP extraction
//! - plugin.cfg parsing
//! - Lifecycle hook execution
//! - Directory isolation

pub mod config_parser;
pub mod database;
pub mod directory_manager;
pub mod environment;
pub mod executor;
pub mod installer;
pub mod lifecycle;

pub use config_parser::PluginConfig;
pub use database::{PluginDatabase, PluginEntry};
pub use directory_manager::DirectoryManager;
pub use environment::{build_plugin_env, build_system_env};
pub use executor::PluginExecutor;
pub use installer::{InstallAction, InstallRequest, PluginInstaller};
pub use lifecycle::{LifecycleHook, LifecycleManager};
