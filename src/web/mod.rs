//! Web server for configuration UI

mod handlers;
pub mod server;
mod static_files;
mod types;

pub use server::start_server;
pub use types::ConfigChangeEvent;
