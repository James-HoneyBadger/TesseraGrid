//! Keyboard shortcut reference dialog.

use crate::app::*;

/// Every keyboard shortcut, kept in one place for the help dialog.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("S", "Step one generation"),
    ("R", "Reset to the selected pattern"),
    ("N", "Randomize with the current seed"),
    ("G", "Toggle grid lines"),
    ("T", "Cycle color theme"),
    ("F  /  Ctrl+0", "Zoom to fit"),
    ("V", "Toggle isometric 3-D view"),
    ("+  /  −", "Zoom in / out"),
    ("Mouse wheel", "Zoom at the cursor"),
    ("Middle-drag  /  arrows", "Pan the view"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y  /  Ctrl+Shift+Z", "Redo"),
    (
        "Ctrl+C  /  Ctrl+X  /  Ctrl+V",
        "Copy / cut / paste selection",
    ),
    ("Delete", "Clear selected cells"),
    ("Escape", "Deselect"),
    ("Ctrl+S  /  Ctrl+O", "Save / open state"),
    ("Ctrl+E", "Export PNG"),
    ("H  /  F1", "This help dialog"),
];

impl TesseraGridApp {
    pub(crate) fn ui_help_modal(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (key, desc) in SHORTCUTS {
                            ui.label(egui::RichText::new(*key).monospace().strong());
                            ui.label(*desc);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                ui.hyperlink_to(
                    "Documentation and source on GitHub",
                    "https://github.com/James-HoneyBadger/TesseraGrid",
                );
                ui.add_space(4.0);
                if ui.button("Close").clicked() {
                    self.show_help = false;
                }
            });
        if !open {
            self.show_help = false;
        }
    }
}
