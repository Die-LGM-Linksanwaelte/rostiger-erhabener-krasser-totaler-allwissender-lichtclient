//! # Kernel Networking Module
//!
//! This module acts as the central communication hub and TCP server management layer for the
//! Rektal lighting control kernel. It is responsible for accepting incoming client connections,
//! enforcing protocol version compatibility, handling secure authentication handshakes, and
//! maintaining persistent user sessions.
//!
//! **Submodule Architecture:**
//!
//! * [`connection_engine`] - Manages global thread-safe server state, client session tracking,
//!   unique connection identifiers, and broadcast routines (such as graceful shutdown announcements).
//! * [`subscriptions`] - Implements the publish-subscribe pattern, maintaining continuous state
//!   buffers and dispatching targeted updates (such as DMX configurations) to active clients based
//!   on their preferred update modes.
//! * [`server_socket`] - Binds the network socket, orchestrates individual client worker threads,
//!   and splits streams into independent, asynchronous read and write loops to handle client RPCs,
//!   CLI commands, and real-time state synchronization.
mod connection_engine;
mod server_sockets;
mod subscriptions;

pub(crate) use server_sockets::activate_socket;
pub(crate) use subscriptions::on_dmx_config_update;
pub(crate) use connection_engine::announce_shutdown;