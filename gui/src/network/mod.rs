//! # Network Module
//!
//! This module encapsulates all network communications for the R.E.K.T.A.L. GUI application.
//! It includes TCP client logic for control commands and responses, UDP client logic for
//! high-frequency DMX streams, and central connection state management.

/// Module tracking connection and session states.
pub(crate) mod connection_state;
/// Module implementing TCP client communication and background IO threads.
pub mod tcp_client;
/// Module implementing high-speed UDP reception for DMX universe streams.
pub mod udp_client;
