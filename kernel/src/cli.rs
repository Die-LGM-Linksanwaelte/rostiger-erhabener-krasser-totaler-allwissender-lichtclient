//! # Command Line Interface (CLI) Subcrate
//!
//! This subcrate provides the command-line parsing, action execution, and interactive 
//! console logic for the application. It translates raw user input and network commands 
//! into structured [`CliAction`](common::cli_actions::CliAction) representations and handles their execution, logging, 
//! and error responses.
mod cli_executing;
mod command_parsing;

pub(crate) use cli_executing::execute_implicit_cli_action;

pub(crate) use command_parsing::run_command;