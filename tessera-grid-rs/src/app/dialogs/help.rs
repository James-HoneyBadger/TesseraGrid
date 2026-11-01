//! Keyboard shortcut reference dialog.

use crate::app::*;

/// Platform name for the command modifier, shown in the help dialog.
const CMD: &str = if cfg!(target_os = "macos") {
    "Cmd"
} else {
    "Ctrl"
};

/// Every keyboard shortcut, kept in one place for the help dialog.
///
/// Chords use the platform command modifier (Ctrl elsewhere, Cmd on macOS), and
/// are matched with `Modifiers::COMMAND` so they work on both.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("S", "Step one generation"),
    ("R", "Reset to the selected pattern"),
    ("N", "Randomize with the current seed"),
    ("G", "Toggle grid lines"),
    ("T", "Cycle color theme"),
    ("F  /  {CMD}+0", "Zoom to fit"),
    ("V", "Toggle isometric 3-D view"),
    ("+  /  −", "Zoom in / out"),
    ("Mouse wheel", "Zoom at the cursor"),
    ("Middle-drag  /  arrows", "Pan the view"),
    ("{CMD}+Z", "Undo"),
    ("{CMD}+Y  /  {CMD}+Shift+Z", "Redo"),
    (
        "{CMD}+C  /  {CMD}+X  /  {CMD}+V",
        "Copy / cut / paste selection",
    ),
    ("Delete", "Clear selected cells"),
    ("Escape", "Deselect"),
    ("{CMD}+S  /  {CMD}+O", "Save / open state"),
    ("{CMD}+E", "Export PNG"),
    ("{CMD}+P", "Pattern editor"),
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
                            let key = key.replace("{CMD}", CMD);
                            ui.label(egui::RichText::new(key).monospace().strong());
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
