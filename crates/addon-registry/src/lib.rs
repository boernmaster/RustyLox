pub mod model;
pub mod persistence;
pub mod proxy;
pub mod registry;

pub use model::{AddonInstance, AddonInstanceView, RegisterRequest};
pub use registry::Registry;
