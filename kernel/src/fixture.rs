//! # Fixture Engine
//!
//! This module provides the core actor-based [`FixtureEngine`] responsible for
//! managing fixture instances, tracking DMX channel reservations across universes,
//! and processing asynchronous commands (such as spawning, moving, or updating properties).
mod fixture_engine;
mod fixture_command;

pub use fixture_engine::{get_fixture_type, move_fixture, new_fixture, remove_fixture, set_property, FixtureEngine};