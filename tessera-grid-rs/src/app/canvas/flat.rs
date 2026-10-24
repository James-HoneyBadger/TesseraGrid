//! Flat top-down canvas: grid, brush and selection interactions.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_canvas_flat(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

        self.canvas_rect = rect;
        self.hover_cell = None;
        if std::mem::take(&mut self.pending_fit) {
            self.zoom_to_fit();
        }

        let cs = self.cell_size;
        let colors = self.theme.colors();
        let show_grid = self.show_grid;
        let show_aging = self.show_aging;
        let rounding = if self.rounded_cells {
            CornerRadius::same(((cs * 0.18).max(1.0)) as u8)
        } else {
            CornerRadius::ZERO
        };

        let (gw, gh, cells_snapshot, age_snapshot) = self.sim.render_snapshot();
        let paint_state = self.paint_state.min(self.sim.max_state());

        // ── Mouse-wheel zoom ─────────────────────────────────────────────────
        if response.hovered() {
            let scroll_y = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.1 {
                let old_cs = self.cell_size;
                let new_cs = (old_cs * (1.0 + scroll_y * 0.04)).clamp(1.0, 64.0);
                if let Some(cursor) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                    let rel = (cursor - rect.min) + self.viewport_offset;
                    self.viewport_offset = rel * (new_cs / old_cs) - (cursor - rect.min);
                }
                self.cell_size = new_cs;
            }
        }

        // ── Middle-click pan ─────────────────────────────────────────────────
        if response.dragged_by(egui::PointerButton::Middle) {
            self.viewport_offset -= response.drag_delta();
        }

        // Center the grid when it is smaller than the viewport, otherwise clamp panning.
        let total_w = gw as f32 * self.cell_size;
        let total_h = gh as f32 * self.cell_size;
        self.viewport_offset.x = if total_w <= avail.x {
            (total_w - avail.x) / 2.0
        } else {
            self.viewport_offset.x.clamp(0.0, total_w - avail.x)
        };
        self.viewport_offset.y = if total_h <= avail.y {
            (total_h - avail.y) / 2.0
        } else {
            self.viewport_offset.y.clamp(0.0, total_h - avail.y)
        };

        let vox = self.viewport_offset.x;
        let voy = self.viewport_offset.y;
        let cs = self.cell_size; // re-read after zoom

        // ── Visible cell culling ─────────────────────────────────────────────
        let start_gx = ((vox / cs) as i64 - 1).max(0) as usize;
        let end_gx = (((vox + avail.x) / cs) as i64 + 2).min(gw as i64) as usize;
        let start_gy = ((voy / cs) as i64 - 1).max(0) as usize;
        let end_gy = (((voy + avail.y) / cs) as i64 + 2).min(gh as i64) as usize;

        let painter = ui.painter_at(rect);

        // ── Background ───────────────────────────────────────────────────────
        let bg = colors.background();
        painter.rect_filled(rect, CornerRadius::ZERO, bg);

        // ── Cells ────────────────────────────────────────────────────────────
        for gy in start_gy..end_gy {
            for gx in start_gx..end_gx {
                let state = cells_snapshot[gy * gw + gx];
                if state == 0 {
                    continue;
                }
                let base = colors.cell(state);
                let color = if show_aging && !age_snapshot.is_empty() {
                    aging_color(base, age_snapshot[gy * gw + gx])
                } else {
                    base
                };
                let cell_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.left() - vox + gx as f32 * cs,
                        rect.top() - voy + gy as f32 * cs,
                    ),
                    Vec2::splat(cs),
                );
                painter.rect_filled(cell_rect, rounding, color);
            }
        }

        // ── Grid lines (clipped to the grid, not the whole viewport) ─────────
        if show_grid && cs >= 4.0 {
            let lc = colors.grid;
            let stroke = egui::Stroke::new(0.5_f32, lc);
            let grid_left = rect.left() - vox;
            let grid_top = rect.top() - voy;
            let x0 = grid_left.max(rect.left());
            let x1 = (grid_left + gw as f32 * cs).min(rect.right());
            let y0 = grid_top.max(rect.top());
            let y1 = (grid_top + gh as f32 * cs).min(rect.bottom());

            for gx in start_gx..=end_gx {
                let px = grid_left + gx as f32 * cs;
                if px >= rect.left() - 1.0 && px <= rect.right() + 1.0 {
                    painter.line_segment([egui::pos2(px, y0), egui::pos2(px, y1)], stroke);
                }
            }
            for gy in start_gy..=end_gy {
                let py = grid_top + gy as f32 * cs;
                if py >= rect.top() - 1.0 && py <= rect.bottom() + 1.0 {
                    painter.line_segment([egui::pos2(x0, py), egui::pos2(x1, py)], stroke);
                }
            }
            // Outline so a grid smaller than the viewport is clearly bounded.
            if x1 > x0 && y1 > y0 {
                painter.rect_stroke(
                    Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1)),
                    CornerRadius::ZERO,
                    egui::Stroke::new(1.0_f32, lc),
                    egui::StrokeKind::Inside,
                );
            }
        }

        // ── Hover highlight ──────────────────────────────────────────────────
        let accent = self.theme.accent(&self.selected_mode);
        if let Some(cursor) = response.hover_pos() {
            let hx = ((cursor.x - rect.left() + vox) / cs) as i64;
            let hy = ((cursor.y - rect.top() + voy) / cs) as i64;
            if hx >= 0 && hx < gw as i64 && hy >= 0 && hy < gh as i64 {
                let hover_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.left() - vox + hx as f32 * cs,
                        rect.top() - voy + hy as f32 * cs,
                    ),
                    Vec2::splat(cs),
                );
                let state = cells_snapshot[hy as usize * gw + hx as usize];
                let hl = Color32::from_rgba_premultiplied(
                    accent.r() / 3,
                    accent.g() / 3,
                    accent.b() / 3,
                    60,
                );
                painter.rect_filled(hover_rect, rounding, hl);
                self.hover_cell = Some((hx, hy, state));
            }
        }

        // ── Selection overlay ────────────────────────────────────────────────
        if let Some((x0, y0, x1, y1)) = self.selection {
            let sx = rect.left() - vox + x0 as f32 * cs;
            let sy = rect.top() - voy + y0 as f32 * cs;
            let ex = rect.left() - vox + (x1 + 1) as f32 * cs;
            let ey = rect.top() - voy + (y1 + 1) as f32 * cs;
            let sel_rect = egui::Rect::from_min_max(egui::pos2(sx, sy), egui::pos2(ex, ey));
            let fill = Color32::from_rgba_premultiplied(100, 160, 255, 25);
            let stroke = egui::Stroke::new(1.5_f32, Color32::from_rgb(120, 180, 255));
            painter.rect_filled(sel_rect, CornerRadius::ZERO, fill);
            painter.rect_stroke(
                sel_rect,
                CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
        }

        // ── Left-click / drag: paint ─────────────────────────────────────────
        if self.tool_mode == ToolMode::Paint {
            if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy_v = ((pos.y - rect.top() + voy) / cs) as i64;
                    if gx >= 0 && gx < gw as i64 && gy_v >= 0 && gy_v < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.paint_line(self.last_paint_cell, (gx, gy_v), paint_state);
                        self.last_paint_cell = Some((gx, gy_v));
                    }
                }
            }

            // ── Right-click / drag: erase ─────────────────────────────────────
            if response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy_v = ((pos.y - rect.top() + voy) / cs) as i64;
                    if gx >= 0 && gx < gw as i64 && gy_v >= 0 && gy_v < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.paint_line(self.last_paint_cell, (gx, gy_v), 0);
                        self.last_paint_cell = Some((gx, gy_v));
                    }
                }
            }

            // Reset drag-undo sentinel when not dragging.
            if !response.dragged() {
                self.drag_undo_pushed = false;
                self.last_paint_cell = None;
            }
        }

        // ── Select tool ──────────────────────────────────────────────────────
        if self.tool_mode == ToolMode::Select {
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy = ((pos.y - rect.top() + voy) / cs) as i64;
                    if self.selection_anchor.is_none() {
                        self.selection_anchor = Some((gx, gy));
                    }
                    let (ax, ay) = self.selection_anchor.unwrap();
                    self.selection = Some((ax.min(gx), ay.min(gy), ax.max(gx), ay.max(gy)));
                }
            }
            if !response.dragged_by(egui::PointerButton::Primary) {
                self.selection_anchor = None;
            }
            // Single click without drag clears selection.
            if response.clicked() {
                self.selection = None;
            }
            // Right-click also clears selection.
            if response.secondary_clicked() {
                self.selection = None;
            }
        }
    }
}
