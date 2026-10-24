//! Pattern editor dialog: draw a pattern, then insert it or copy its RLE.

use crate::app::*;

/// Size of one editor cell in points.
const EDITOR_CELL: f32 = 13.0;
/// Selectable editor canvas sizes.
const EDITOR_SIZES: [usize; 4] = [16, 24, 32, 48];

impl TesseraGridApp {
    pub(crate) fn ui_editor(&mut self, ctx: &egui::Context) {
        if !self.show_editor {
            return;
        }
        let colors = self.theme.colors();
        let mut open = true;
        egui::Window::new("Pattern Editor")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                self.ui_editor_canvas(ui, colors);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    let mut size = self.editor.size();
                    egui::ComboBox::from_id_salt("editor_size")
                        .selected_text(format!("{size}×{size}"))
                        .show_ui(ui, |ui| {
                            for option in EDITOR_SIZES {
                                ui.selectable_value(
                                    &mut size,
                                    option,
                                    format!("{option}×{option}"),
                                );
                            }
                        });
                    if size != self.editor.size() {
                        self.editor = patterns::PatternDraft::new(size);
                    }
                    ui.separator();
                    ui.weak(format!("{} cells", self.editor.population()));
                });
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.editor.clear();
                    }
                    if ui.button("Invert").clicked() {
                        self.editor.invert();
                    }
                    if ui.button("Rotate").clicked() {
                        self.editor.rotate_cw();
                    }
                    if ui.button("Flip H").clicked() {
                        self.editor.flip_horizontal();
                    }
                    if ui.button("Flip V").clicked() {
                        self.editor.flip_vertical();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.editor.is_empty(),
                            egui::Button::new("Insert at centre"),
                        )
                        .on_hover_text("Stamp the pattern into the middle of the grid")
                        .clicked()
                    {
                        self.insert_draft_at_centre();
                    }
                    if ui
                        .add_enabled(!self.editor.is_empty(), egui::Button::new("Copy RLE"))
                        .on_hover_text("Copy the pattern to the clipboard as RLE")
                        .clicked()
                    {
                        self.copy_draft_rle();
                    }
                });
                ui.weak("Drag to draw · right-drag to erase");
            });
        self.show_editor = open;
    }

    fn ui_editor_canvas(&mut self, ui: &mut egui::Ui, colors: crate::theme::ThemeColors) {
        let size = self.editor.size();
        let side = size as f32 * EDITOR_CELL + 2.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(4), colors.background());

        let grid_stroke = egui::Stroke::new(1.0, colors.grid);
        for step in 0..=size {
            let offset = step as f32 * EDITOR_CELL + 1.0;
            painter.line_segment(
                [
                    egui::pos2(rect.left() + offset, rect.top() + 1.0),
                    egui::pos2(rect.left() + offset, rect.bottom() - 1.0),
                ],
                grid_stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 1.0, rect.top() + offset),
                    egui::pos2(rect.right() - 1.0, rect.top() + offset),
                ],
                grid_stroke,
            );
        }

        for y in 0..size {
            for x in 0..size {
                if self.editor.get(x, y) {
                    let min = egui::pos2(
                        rect.left() + 1.0 + x as f32 * EDITOR_CELL,
                        rect.top() + 1.0 + y as f32 * EDITOR_CELL,
                    );
                    painter.rect_filled(
                        Rect::from_min_size(min, Vec2::splat(EDITOR_CELL)),
                        CornerRadius::ZERO,
                        colors.cell(1),
                    );
                }
            }
        }

        if response.hovered() {
            let (pointer, erasing) = ui.input(|i| {
                (
                    i.pointer.interact_pos(),
                    i.pointer.button_down(egui::PointerButton::Secondary),
                )
            });
            let drawing = response.dragged() || response.clicked();
            if drawing || erasing {
                if let Some(pos) = pointer {
                    if rect.contains(pos) {
                        let local = pos - rect.min - Vec2::splat(1.0);
                        let x = (local.x / EDITOR_CELL).max(0.0) as usize;
                        let y = (local.y / EDITOR_CELL).max(0.0) as usize;
                        self.editor.set(x, y, !erasing);
                    }
                }
            }
        }
    }

    fn insert_draft_at_centre(&mut self) {
        let (grid_w, grid_h) = (self.sim.width(), self.sim.height());
        let size = self.editor.size();
        let x0 = grid_w.saturating_sub(size) / 2;
        let y0 = grid_h.saturating_sub(size) / 2;
        self.sim.push_undo();
        let editor = &self.editor;
        self.sim
            .edit_cells(|cells| editor.stamp_into(cells, grid_w, grid_h, x0, y0));
        self.set_status(format!("Inserted {} cells", editor.population()));
    }

    fn copy_draft_rle(&mut self) {
        let encoded = rle::encode(&self.editor.to_grid(), &self.sim.rule_string());
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(encoded)) {
            Ok(()) => self.set_status("Pattern copied as RLE"),
            Err(err) => self.set_status(format!("Clipboard error: {err}")),
        }
    }
}
