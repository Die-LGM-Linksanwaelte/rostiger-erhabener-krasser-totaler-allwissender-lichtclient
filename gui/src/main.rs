use gtk::prelude::*;
use gtk::Label;
use libadwaita::{Application as AdwApplication, ApplicationWindow};
use libpanel::prelude::*;
use libpanel::{Dock, Frame, Grid, Widget as PanelWidget};

fn main() {
    let app = AdwApplication::builder()
        .application_id("com.github.properdocking")
        .build();

    app.connect_startup(|_| {
        libpanel::init();
    });

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &AdwApplication) {
    let ui_src = include_str!("window.ui");
    let builder = gtk::Builder::from_string(ui_src);

    let window: ApplicationWindow = builder.object("window").expect("window fehlt");
    window.set_application(Some(app));

    let dock: Dock = builder.object("dock").expect("dock fehlt");
    let center_grid: Grid = builder.object("center_grid").expect("center_grid fehlt");

    center_grid.connect_create_frame(|_| Frame::new());

    let center_frame: Frame = builder.object("center_frame").expect("center_frame fehlt");
    let left_frame: Frame = builder.object("left_frame").expect("left_frame fehlt");
    let bottom_frame: Frame = builder.object("bottom_frame").expect("bottom_frame fehlt");
    let right_frame: Frame = builder.object("right_frame").expect("right_frame fehlt");
    let top_frame: Frame = builder.object("top_frame").expect("top_frame fehlt");

    // Tabs erstellen und hinzufügen
    center_frame.add(
        &PanelWidget::builder()
            .title("Editor")
            .child(&Label::new(Some("MITTE")))
            .build(),
    );
    left_frame.add(
        &PanelWidget::builder()
            .title("Explorer")
            .child(&Label::new(Some("LINKS")))
            .build(),
    );
    right_frame.add(
        &PanelWidget::builder()
            .title("Outline")
            .child(&Label::new(Some("RECHTS")))
            .build(),
    );
    bottom_frame.add(
        &PanelWidget::builder()
            .title("Terminal")
            .child(&Label::new(Some("UNTEN")))
            .build(),
    );
    top_frame.add(
        &PanelWidget::builder()
            .title("Tools")
            .child(&Label::new(Some("OBEN")))
            .build(),
    );

    // --- WICHTIG: Die Panels explizit ausklappen ---
    dock.set_reveal_start(true);
    dock.set_reveal_end(true);
    dock.set_reveal_bottom(true);
    dock.set_reveal_top(true);

    window.present();
}
