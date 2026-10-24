//! Isometric 3-D canvas: state columns, brush and selection interactions.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_canvas_iso(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        self.canvas_rect = rect;
        self.hover_cell = None;
        if std::mem::take(&mut self.pending_fit) {
            self.iso_fit();
        }

        // Mouse-wheel zoom (no cursor centering in iso view)
        if response.hovered() {
            let scroll_y = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.1 {
                self.cell_size = (self.cell_size * (1.0 + scroll_y * 0.04)).clamp(1.0, 64.0);
            }
        }

        // Middle-click pan
        if response.dragged_by(egui::PointerButton::Middle) {
            self.iso_offset += response.drag_delta();
        }

        let cs = self.cell_size;
        let iso_h = self.iso_height;
        let colors = self.theme.colors();

        let (gw, gh, cells_snapshot, age_snapshot) = self.sim.render_snapshot();
        let paint_state = self.paint_state.min(self.sim.max_state());

        // Isometric projection: world (wx, wy, wz) → screen.
        // Moving +wx → screen right+down; +wy → screen left+down; +wz → screen up.
        let base_x =
            rect.center().x + self.iso_offset.x - (gw as f32 / 2.0 - gh as f32 / 2.0) * cs * 0.5;
        let base_y = rect.min.y + rect.height() * 0.38 + self.iso_offset.y
            - (gw as f32 / 2.0 + gh as f32 / 2.0) * cs * 0.25;

        let ipt = |wx: f32, wy: f32, wz: f32| -> egui::Pos2 {
            egui::pos2(
                base_x + (wx - wy) * cs * 0.5,
                base_y + (wx + wy) * cs * 0.25 - wz * cs * 0.5 * iso_h,
            )
        };

        // Helper: darken a colour by factor 0–1.
        let shade = |c: Color32, f: f32| -> Color32 {
            Color32::from_rgb(
                (c.r() as f32 * f) as u8,
                (c.g() as f32 * f) as u8,
                (c.b() as f32 * f) as u8,
            )
        };

        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, CornerRadius::ZERO, colors.background());

        // Ground-grid lines
        if self.show_grid && cs >= 3.0 {
            let lc = colors.grid;
            let stroke = egui::Stroke::new(0.5_f32, lc);
            for gx in 0..=gw {
                let p1 = ipt(gx as f32, 0.0, 0.0);
                let p2 = ipt(gx as f32, gh as f32, 0.0);
                painter.line_segment([p1, p2], stroke);
            }
            for gy in 0..=gh {
                let p1 = ipt(0.0, gy as f32, 0.0);
                let p2 = ipt(gw as f32, gy as f32, 0.0);
                painter.line_segment([p1, p2], stroke);
            }
        }

        // Collect live cells and sort back→front: ascending (gx+gy), then ascending gx.
        let mut live: Vec<(usize, usize, u8)> = cells_snapshot
            .iter()
            .enumerate()
            .filter(|(_, &s)| s > 0)
            .map(|(i, &s)| (i % gw, i / gw, s))
            .collect();
        live.sort_unstable_by_key(|&(gx, gy, _)| (gx + gy, gx));

        for (gx, gy, state) in &live {
            let x = *gx as f32;
            let y = *gy as f32;
            let h = *state as f32;

            let base_c = if self.show_aging && age_snapshot.len() > gy * gw + gx {
                aging_color(colors.cell(*state), age_snapshot[gy * gw + gx])
            } else {
                colors.cell(*state)
            };

            // Right face (x+1 side — lower right of block)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x + 1.0, y, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, 0.0),
                    ipt(x + 1.0, y, 0.0),
                ],
                shade(base_c, 0.52),
                egui::Stroke::NONE,
            ));
            // Left face (y+1 side — lower left of block)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, 0.0),
                    ipt(x, y + 1.0, 0.0),
                ],
                shade(base_c, 0.72),
                egui::Stroke::NONE,
            ));
            // Top face
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x, y, h),
                    ipt(x + 1.0, y, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x, y + 1.0, h),
                ],
                base_c,
                egui::Stroke::NONE,
            ));
        }

        // Selection overlay (flat diamond outline on ground plane)
        if let Some((x0, y0, x1, y1)) = self.selection {
            let x0f = x0 as f32;
            let y0f = y0 as f32;
            let x1f = (x1 + 1) as f32;
            let y1f = (y1 + 1) as f32;
            let sel_pts = vec![
                ipt(x0f, y0f, 0.0),
                ipt(x1f, y0f, 0.0),
                ipt(x1f, y1f, 0.0),
                ipt(x0f, y1f, 0.0),
            ];
            painter.add(egui::Shape::closed_line(
                sel_pts,
                egui::Stroke::new(2.0_f32, Color32::from_rgb(120, 180, 255)),
            ));
        }

        // Hover highlight using inverse iso projection (ground plane, z=0)
        let screen_to_grid = |pos: egui::Pos2| -> (i64, i64) {
            let dx = (pos.x - base_x) / (cs * 0.5);
            let dy = (pos.y - base_y) / (cs * 0.25);
            (
                ((dx + dy) / 2.0).floor() as i64,
                ((dy - dx) / 2.0).floor() as i64,
            )
        };

        let accent = self.theme.accent(&self.selected_mode);
        if let Some(cursor) = response.hover_pos() {
            let (hx, hy) = screen_to_grid(cursor);
            if hx >= 0 && hx < gw as i64 && hy >= 0 && hy < gh as i64 {
                let state = cells_snapshot[hy as usize * gw + hx as usize];
                let hxf = hx as f32;
                let hyf = hy as f32;
                painter.add(egui::Shape::closed_line(
                    vec![
                        ipt(hxf, hyf, 0.0),
                        ipt(hxf + 1.0, hyf, 0.0),
                        ipt(hxf + 1.0, hyf + 1.0, 0.0),
                        ipt(hxf, hyf + 1.0, 0.0),
                    ],
                    egui::Stroke::new(1.5_f32, accent),
                ));
                self.hover_cell = Some((hx, hy, state));
            }
        }

        // Paint tool
        if self.tool_mode == ToolMode::Paint {
            if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
                    if gx >= 0 && gx < gw as i64 && gy >= 0 && gy < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.paint_line(self.last_paint_cell, (gx, gy), paint_state);
                        self.last_paint_cell = Some((gx, gy));
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
                    if gx >= 0 && gx < gw as i64 && gy >= 0 && gy < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.paint_line(self.last_paint_cell, (gx, gy), 0);
                        self.last_paint_cell = Some((gx, gy));
                    }
                }
            }
            if !response.dragged() {
                self.drag_undo_pushed = false;
                self.last_paint_cell = None;
            }
        }

        // Select tool
        if self.tool_mode == ToolMode::Select {
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
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
            if response.clicked() || response.secondary_clicked() {
                self.selection = None;
            }
        }
    }
}
