mod gui;
mod gui_actions;
mod roles;
mod logic;

use std::process::Command;
use std::rc::Rc;

/// Entry point of the R.E.K.T.A.Launcher application
/// This function calls the GUI run function to start the application.
fn main() {
    gui::run();
}