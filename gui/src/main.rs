use gtk::prelude::*;
use gtk::Label;
use libadwaita::{Application as AdwApplication, ApplicationWindow};
use libpanel::prelude::*;
use libpanel::{Dock, Frame, Grid, Widget as PanelWidget, Position, Area, FrameTabBar};

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

    center_grid.connect_create_frame(|_| {
        let frame = Frame::new();
        let tab_bar = FrameTabBar::new();
        frame.set_header(Some(&tab_bar));
        frame
    });

    let center_frame: Frame = builder.object("center_frame").expect("center_frame fehlt");
    let left_frame: Frame = builder.object("left_frame").expect("left_frame fehlt");
    let bottom_frame: Frame = builder.object("bottom_frame").expect("bottom_frame fehlt");
    let right_frame: Frame = builder.object("right_frame").expect("right_frame fehlt");
    let top_frame: Frame = builder.object("top_frame").expect("top_frame fehlt");

    // Header für alle Frames setzen, damit Tabs angezeigt werden und Drop funktioniert
    for frame in [&center_frame, &left_frame, &bottom_frame, &right_frame, &top_frame] {
        let tab_bar = FrameTabBar::new();
        // Falls FrameTabBar reorderable-Methoden hat, hier setzen. 
        // In libpanel 0.6 ist meist das Widget selbst verantwortlich, 
        // aber wir stellen sicher, dass der Header korrekt gesetzt ist.
        frame.set_header(Some(&tab_bar));
    }

    // Tabs erstellen und hinzufügen
    let editor_widget = PanelWidget::builder()
        .title("Editor")
        .reorderable(true)
        .child(&Label::new(Some("MITTE")))
        .build();
    center_frame.add(&editor_widget);

    let explorer_widget = PanelWidget::builder()
        .title("Explorer")
        .reorderable(true)
        .child(&Label::new(Some("LINKS")))
        .build();
    left_frame.add(&explorer_widget);

    let outline_widget = PanelWidget::builder()
        .title("Outline")
        .reorderable(true)
        .child(&Label::new(Some("RECHTS")))
        .build();
    right_frame.add(&outline_widget);

    let terminal_widget = PanelWidget::builder()
        .title("Terminal")
        .reorderable(true)
        .child(&Label::new(Some("UNTEN")))
        .build();
    bottom_frame.add(&terminal_widget);

    let tools_widget = PanelWidget::builder()
        .title("Tools")
        .reorderable(true)
        .child(&Label::new(Some("OBEN")))
        .build();
    top_frame.add(&tools_widget);

    // --- WICHTIG: Die Panels explizit ausklappen ---
    dock.set_reveal_start(true);
    dock.set_reveal_end(true);
    dock.set_reveal_bottom(true);
    dock.set_reveal_top(true);

    window.present();
}
