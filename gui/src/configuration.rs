use crate::panels::TabSaveData;
use common::networking::messages::UserRole;
use eframe::Theme;
use egui::accesskit::Role;
use egui_dock::DockState;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct GuiConfig {
    pub theme: Theme,
    pub last_ip_address: String,
    pub last_username: String,
    pub last_role: UserRole,
    pub last_window_layout: String,
}

#[derive(Serialize, Deserialize)]
pub struct GuiLayout {
    pub role: Role,
    pub name: String,
    pub description: String,
    pub window_layout: DockState<TabSaveData>,
}

