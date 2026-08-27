//! # Interface Module
//!
//! This module acts as the hardware abstraction layer for the Rektal lighting control system.
//! It is currently responsible for translating the pre-computed internal engine state into
//! standardized output protocols (such as ArtNet) and broadcasting this data to the physical network.
//!
//! **Future Architecture Roadmap:**
//!
//! Currently, the interface operates as an internal library module, receiving fully calculated DMX buffers
//! directly from the engine via `mpsc` channels. In the future, this module will be decoupled into a
//! fully independent, standalone executable with significantly expanded computational responsibilities.
//!
//! Once extracted, the central Rektal kernel will offload the heavy computational lifting. The kernel
//! will act primarily as a state manager—handling structural show data such as fixture positions, stage-views,
//! prefixes, and overall cuelist orchestration. This standalone interface daemon will receive these high-level
//! states over the network and independently calculate the final fixture properties and raw DMX values based
//! on the active programmer, prefixes, and cuelist states.
//!
//! This distributed execution model will ensure seamless scalability, independent crash recovery, and lower
//! latency by allowing the rendering engine to be hosted on remote network nodes closer to the physical rig.
pub mod interfaces;
mod artnet;