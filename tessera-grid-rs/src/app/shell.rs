use super::*;

impl eframe::App for TesseraGridApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);

        // FPS tracking (exponential moving average)
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }
        self.last_frame_time = now;

        self.handle_keyboard(ctx);

        // Simulation tick: fixed timestep with catch-up so fast speeds are real.
        if self.running {
            let step_interval = self.step_duration();
            let now = Instant::now();
            let mut steps = 0;
            while now.duration_since(self.last_step) >= step_interval && steps < MAX_STEPS_PER_FRAME
            {
                self.sim.step();
                self.last_step += step_interval;
                steps += 1;
            }
            // Drop the backlog instead of spiralling if we hit the cap.
            if now.duration_since(self.last_step) >= step_interval {
                self.last_step = now;
            }
            ctx.request_repaint_after(step_interval);
        }

        // Run-N executes in short slices so the UI keeps repainting.
        if self.run_n_pending > 0 {
            let slice_start = Instant::now();
            let done = self.sim.run_steps(self.run_n_pending, || {
                slice_start.elapsed() >= Duration::from_millis(8)
            });
            self.run_n_pending -= done;
            if self.run_n_pending > 0 {
                ctx.request_repaint();
            } else {
                self.set_status(format!("Ran {} steps", self.run_n_total));
            }
        }

        // Generation-rate sampling (half-second window).
        let elapsed = now.duration_since(self.gen_sample_time).as_secs_f32();
        if elapsed >= 0.5 {
            let generations = self
                .sim
                .generation
                .saturating_sub(self.gen_sample_generation);
            self.gen_rate = generations as f32 / elapsed;
            self.gen_sample_time = now;
            self.gen_sample_generation = self.sim.generation;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Modal dialogs (rendered above all panels)
        self.ui_run_n_dialog(&ctx);
        self.ui_resize_dialog(&ctx);
        self.ui_help_modal(&ctx);
        self.ui_gallery(&ctx);
        self.ui_about(&ctx);

        let colors = self.theme.colors();
        let bar_fill = colors.card;
        let bar_stroke = colors.stroke;
        let bar_margin = if self.compact_mode {
            egui::Margin::symmetric(10, 6)
        } else {
            egui::Margin::symmetric(12, 8)
        };
        let bar_frame = egui::Frame::default()
            .fill(bar_fill)
            .stroke(egui::Stroke::new(1.0_f32, bar_stroke))
            .corner_radius(CornerRadius::same(0))
            .inner_margin(bar_margin);

        // Menu bar
        egui::Panel::top("menubar").frame(bar_frame).show(ui, |ui| {
            self.ui_menu_bar(ui);
        });

        // Toolbar
        egui::Panel::top("toolbar").frame(bar_frame).show(ui, |ui| {
            self.ui_toolbar(ui);
        });

        // Status bar
        egui::Panel::bottom("statusbar")
            .frame(bar_frame)
            .show(ui, |ui| {
                self.ui_statusbar(ui);
            });

        // Controls side panel
        let panel_min_width = if self.compact_mode { 160.0 } else { 180.0 };
        let panel_default_width = if self.compact_mode { 188.0 } else { 220.0 };
        let panel_target = self.panel_width.clamp(panel_min_width, 420.0);
        let panel = egui::Panel::left("controls")
            .resizable(true)
            .min_size(panel_min_width)
            .max_size(420.0)
            .default_size(panel_target.max(panel_default_width))
            .show(ui, |ui| {
                // Bound child widgets so they cannot stretch the panel.
                ui.set_max_width((panel_target - 20.0).max(120.0));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ui_controls(ui);
                });
            });
        self.panel_width = panel.response.rect.width().clamp(panel_min_width, 420.0);

        // Canvas
        egui::CentralPanel::default().show(ui, |ui| {
            self.ui_canvas(ui);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
}
