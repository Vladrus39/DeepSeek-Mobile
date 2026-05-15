//! DeepSeek Mobile Core
//! Переиспользуемая логика agent'а из оригинального DeepSeek-TUI

pub mod agent;
pub mod tools;
pub mod config;
pub mod session;

// Re-exports
pub use agent::Agent;
pub use config::Config;

pub fn version() -> &'static str {
    "0.1.0"
}
