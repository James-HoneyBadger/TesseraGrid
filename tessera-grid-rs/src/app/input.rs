use super::*;

impl TesseraGridApp {
    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Text fields own the keyboard while focused (e.g. the custom rule input).
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // A sensitivity experiment drives the grid itself; any shortcut (undo,
        // step, randomize) would desynchronise the control from its probe.
        if self
            .sensitivity
            .as_ref()
            .is_some_and(|experiment| !experiment.finished)
        {
            return;
        }
        // Space would also activate a focused button, so let the widget win.
        let widget_focused = ctx.memory(|memory| memory.focused().is_some());

        // Ctrl/Cmd shortcuts run first and consume their key, so a chord such as
        // Ctrl+S can never also trigger the plain `S` step binding below.
        if ctx.input_mut(|i| self.handle_shortcuts(i)) {
            return;
        }
        ctx.input(|i| self.handle_keys(i, widget_focused));
    }

    fn any_modal_open(&self) -> bool {
        self.show_help
            || self.show_about
            || self.show_gallery
            || self.show_editor
            || self.show_run_n_dialog
            || self.show_resize_dialog
    }

    /// Close the topmost open modal. Returns `false` when none was open.
    fn close_top_modal(&mut self) -> bool {
        for open in [
            &mut self.show_about,
            &mut self.show_help,
            &mut self.show_gallery,
            &mut self.show_editor,
            &mut self.show_run_n_dialog,
            &mut self.show_resize_dialog,
        ] {
            if *open {
                *open = false;
                return true;
            }
        }
        false
    }

    /// Handle Ctrl/Cmd chords. Returns `true` when one was handled.
    ///
    /// `Modifiers::COMMAND` maps to Ctrl on Linux/Windows and ⌘ on macOS, so one
    /// table serves every platform. Using `modifiers.ctrl` directly left every
    /// shortcut dead on macOS.
    fn handle_shortcuts(&mut self, i: &mut egui::InputState) -> bool {
        use egui::{Key, KeyboardShortcut as Shortcut, Modifiers};

        let cmd = |key| Shortcut::new(Modifiers::COMMAND, key);

        if i.consume_shortcut(&cmd(Key::Z)) {
            if self.sim.undo() {
                self.running = false;
            }
            return true;
        }
        if i.consume_shortcut(&Shortcut::new(
            Modifiers::COMMAND | Modifiers::SHIFT,
            Key::Z,
        )) || i.consume_shortcut(&cmd(Key::Y))
        {
            if self.sim.redo() {
                self.running = false;
            }
            return true;
        }
        if i.consume_shortcut(&cmd(Key::S)) {
            self.save_state();
            return true;
        }
        if i.consume_shortcut(&cmd(Key::O)) {
            self.load_state();
            return true;
        }
        if i.consume_shortcut(&cmd(Key::E)) {
            self.export_png_dialog();
            return true;
        }
        if i.consume_shortcut(&cmd(Key::P)) {
            self.show_editor = !self.show_editor;
            return true;
        }
        if i.consume_shortcut(&cmd(Key::C)) {
            self.copy_selection();
            return true;
        }
        if i.consume_shortcut(&cmd(Key::X)) {
            self.cut_selection();
            return true;
        }
        if i.consume_shortcut(&cmd(Key::V)) {
            self.paste_clipboard();
            return true;
        }
        if i.consume_shortcut(&Shortcut::new(Modifiers::COMMAND, Key::Num0)) {
            self.pending_fit = true;
            return true;
        }
        false
    }

    fn handle_keys(&mut self, i: &egui::InputState, widget_focused: bool) {
        // Plain-key bindings must not fire while a chord is held (Ctrl+N must
        // not randomize, for example).
        let plain = !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt;

        if self.any_modal_open() {
            // Modals own the keyboard: Escape closes them and nothing else may
            // mutate the simulation underneath.
            if plain && i.key_pressed(egui::Key::Escape) {
                self.close_top_modal();
            }
            return;
        }
        if !plain {
            return;
        }

        if i.key_pressed(egui::Key::Space) && !widget_focused {
            self.running = !self.running;
            if self.running {
                self.last_step = Instant::now();
            }
        }
        if i.key_pressed(egui::Key::S) {
            self.push_undo();
            self.do_step();
        }
        if i.key_pressed(egui::Key::N) {
            self.randomize();
        }
        if i.key_pressed(egui::Key::T) {
            self.theme = self.theme.next();
        }
        if i.key_pressed(egui::Key::R) {
            self.do_reset();
            let pat = self.selected_pattern.clone();
            self.sim.load_pattern(&pat);
        }
        if i.key_pressed(egui::Key::G) {
            self.show_grid = !self.show_grid;
        }

        // Zoom
        let plus = i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals);
        let minus = i.key_pressed(egui::Key::Minus);
        if plus {
            self.cell_size = (self.cell_size + 1.0).min(64.0);
        }
        if minus {
            self.cell_size = (self.cell_size - 1.0).max(1.0);
        }

        // Zoom to fit
        if i.key_pressed(egui::Key::F) {
            self.pending_fit = true;
        }

        // Arrow-key panning
        let pan = (self.cell_size * 5.0).max(10.0);
        if i.key_pressed(egui::Key::ArrowLeft) {
            self.viewport_offset.x -= pan;
        }
        if i.key_pressed(egui::Key::ArrowRight) {
            self.viewport_offset.x += pan;
        }
        if i.key_pressed(egui::Key::ArrowUp) {
            self.viewport_offset.y -= pan;
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            self.viewport_offset.y += pan;
        }

        // Help
        if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::F1) {
            self.show_help = !self.show_help;
        }

        // Toggle 3-D view
        if i.key_pressed(egui::Key::V) {
            self.view_mode = match self.view_mode {
                ViewMode::Flat => ViewMode::Isometric,
                ViewMode::Isometric => ViewMode::Flat,
            };
            self.pending_fit = true;
        }

        // Selection shortcuts
        if i.key_pressed(egui::Key::Escape) {
            self.selection = None;
            self.selection_anchor = None;
        }
        if i.key_pressed(egui::Key::Delete) {
            self.clear_selection_cells();
        }
    }
}
