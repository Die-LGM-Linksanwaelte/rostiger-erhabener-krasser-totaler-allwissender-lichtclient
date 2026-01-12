use std::cell::{Cell, Ref, RefCell};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use std::rc::Rc;

pub(crate) fn run(run_rektal_callback: Rc<dyn Fn(&str)>) {
    let app = Application::new(
        Some("de.loetgott.rektal_launcher"),
        Default::default(),
    );

    let callback_clone = run_rektal_callback.clone();
    app.connect_activate(move |app| {
        build_ui(app, callback_clone.clone());
    });
    app.run();
}

fn build_ui(app: &Application, run_rektal_callback: Rc<dyn Fn(&str)>) {
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
    let selected_role_clone = selected_role.clone();
    combo.connect_changed(move |c| {
        if let Some(text) = c.active_text() {
            *selected_role_clone.borrow_mut() = text.to_string();
        }
    });

    main_box.append(&combo);

    // ---- Button ----
    let selected_role_clone = selected_role.clone();
    let callback_clone = run_rektal_callback.clone();

    let button = gtk4::Button::with_label("Start R.E.K.T.A.L.");
    button.connect_clicked(move |_| {
        println!("Button clicked!");
        callback_clone(selected_role_clone.borrow().as_str());
    });

    main_box.append(&button);


    window.show();
}
