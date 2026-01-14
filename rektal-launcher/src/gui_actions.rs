use crate::roles;

/// Enum representing different GUI actions in the R.E.K.T.A.Launcher application
/// that can be executed based on user interactions.
pub enum GuiAction {
    StartRektal,
    SaveRole(roles::Roles),
    SaveName(String),

}


impl GuiAction {

    /// Implementation of the GuiAction enum providing a method to execute the corresponding action.
    /// Each variant of the enum corresponds to a specific user interaction
    /// in the GUI, such as clicking a button or changing a dropdown selection.
    pub fn execute(&self) {
        match self {
            GuiAction::StartRektal => {
                println!("Button wurde geklickt");
                //TODO: Methode zum starten ausführen
            }
            GuiAction::SaveRole(selection) => {
                println!("Dropdown geändert: {}", selection.as_str());
                //TODO: Methode zum speichern der Rolle ausführen
            }
            GuiAction::SaveName(name) => {
                println!("Name geändert: {}", name);
                //TODO: Methode zum speichern des Namens ausführen
            }
        }
    }
}