use std::cell::{Cell, Ref, RefCell};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use std::rc::Rc;
use gtk4::gio::ffi::g_simple_async_result_get_source_tag;
use crate::gui_actions::GuiAction;

/// Run the R.E.K.T.A.Launcher GUI application
/// This function initializes the GTK application, sets up the main window,
/// and starts the event loop.
pub(crate) fn run() {
    let app = Application::new(
        Some("R.E.K.T.A.Launcher"),
        Default::default(),
    );

    app.connect_activate(move |app| {
        build_ui(app);
    });
    app.run();
}

/// Build the GUI for the R.E.K.T.A.Launcher application
/// # Arguments
/// * `app` - The GTK application instance
///
/// This function creates a window with a dropdown menu for selecting a role
/// and a button to start the R.E.K.T.A.L. application with the selected role.
/// Every Gui Action is handled via the GuiAction enum in gui_actions.rs
fn build_ui(app: &Application) {
    let selected_role = Rc::new(RefCell::new(String::new())) ;
    let window = ApplicationWindow::new(app);
    window.set_title(Some("R.E.K.T.A.Launcher"));
    window.set_default_size(400, 300);

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    window.set_child(Some(&main_box));

    let drop_down_label = gtk4::Label::new(Some("Please select a role"));
    drop_down_label.set_margin_top(10);
    main_box.append(&drop_down_label);

    let combo = gtk4::ComboBoxText::new();
    combo.append_text("Programmer");
    combo.append_text("blind Programmer");
    combo.append_text("Operator");
    combo.append_text("Interface");

    combo.set_active(Some(0));

    // ---- Dropdown ----
    combo.connect_changed(move |c| {
        if let Some(text) = c.active_text() {
            let action = GuiAction::SaveRole(
                match text.as_str() {
                    "Programmer" => crate::roles::Roles::Programmer,
                    "blind Programmer" => crate::roles::Roles::BlindProgrammer,
                    "Operator" => crate::roles::Roles::Operator,
                    "Interface" => crate::roles::Roles::Interface,
                    _ => crate::roles::Roles::Programmer,
                }
            );
            println!("Selected role: {}", text);
            action.execute();
        }
    });

    main_box.append(&combo);

    // ---- Button ----
    let button = gtk4::Button::with_label("Start R.E.K.T.A.L.");
    button.connect_clicked(move |_| {
        let action = GuiAction::StartRektal;
        action.execute();
    });

    main_box.append(&button);


    window.show();
}
