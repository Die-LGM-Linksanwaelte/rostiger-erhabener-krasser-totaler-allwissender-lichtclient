//! # Common Crate
//!
//! This crate serves as the shared foundation for the Rektal lighting control system,
//! providing the core data structures, protocols, and utilities utilized by both the
//! server engine and connected clients. By centralizing these definitions, it ensures
//! strict synchronization and compatibility across the entire application ecosystem.
//!
//! Additionally, this crate compiles into a lightweight Test-Client binary (`main.rs`)
//! used for debugging network connections, protocol validations, and raw server interactions.
//!
//! **Core Modules:**
//!
//! * **`cli_actions`**: Defines the structured representations of user commands and their expected responses, bridging
//! raw input to actionable engine logic.
//! * **`fixture`**: Contains the foundational lighting models, including DMX channel mappings, fixture profiles
//! (`fixture_type`), and color management structures.
//! * **`logging`**: Provides a robust, thread-safe, and asynchronous logging infrastructure with customizable sinks
//! (e.g., terminal and file output) and distinct severity levels.
//! * **`networking`**: Outlines the comprehensive TCP communication protocol. This includes session management,
//! client/server message definitions, and the real-time data subscription system.
#[macro_use]
pub mod logging;

pub mod fixture;
pub mod networking;
pub mod cli_actions;