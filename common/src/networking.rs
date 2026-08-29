//! # Networking Module
//!
//! This module defines the core communication protocol and data structures used for
//! network interactions between the client and the kernel². It contains the fundamental
//! message definitions for TCP communication ([`TcpClientMessage`](messages::TcpClientMessage) and
//! [`TcpServerMessage`](messages::TcpServerMessage)), handshake procedures for version and protocol
//! verification, session management, and the robust subscription system used to stream real-time
//! engine state updates (such as visual DMX configurations) to connected GUIs.
pub mod messages;
pub mod subscription_objects;
