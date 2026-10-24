use super::*;

impl TesseraGridApp {
    pub(super) fn copy_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width as i64, g.height as i64)
            };
            let x0c = x0.max(0) as usize;
            let y0c = y0.max(0) as usize;
            let x1c = x1.min(gw - 1) as usize;
            let y1c = y1.min(gh - 1) as usize;
            if x0c > x1c || y0c > y1c {
                return;
            }
            let cw = x1c - x0c + 1;
            let ch = y1c - y0c + 1;
            let mut cells = vec![0u8; cw * ch];
            let grid = self.sim.grid();
            for dy in 0..ch {
                for dx in 0..cw {
                    cells[dy * cw + dx] = grid.get(y0c + dy, x0c + dx);
                }
            }
            self.clipboard = Some((cells, cw, ch));
            self.set_status(format!("Copied {}×{} region", cw, ch));
        }
    }

    pub(super) fn cut_selection(&mut self) {
        self.copy_selection();
        self.clear_selection_cells();
    }

    pub(super) fn paste_clipboard(&mut self) {
        if let Some((cells, cw, ch)) = self.clipboard.clone() {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width, g.height)
            };
            let (px, py) = if let Some((x0, y0, _, _)) = self.selection {
                (x0.max(0) as usize, y0.max(0) as usize)
            } else {
                (gw.saturating_sub(cw) / 2, gh.saturating_sub(ch) / 2)
            };
            self.push_undo();
            self.sim.edit_cells(|grid_cells| {
                for dy in 0..ch {
                    for dx in 0..cw {
                        let gx = px + dx;
                        let gy = py + dy;
                        if gx < gw && gy < gh {
                            grid_cells[gy * gw + gx] = cells[dy * cw + dx];
                        }
                    }
                }
            });
            self.set_status(format!("Pasted {}×{} region", cw, ch));
        }
    }

    pub(super) fn clear_selection_cells(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width as i64, g.height as i64)
            };
            let x0c = x0.max(0) as usize;
            let y0c = y0.max(0) as usize;
            let x1c = x1.min(gw - 1) as usize;
            let y1c = y1.min(gh - 1) as usize;
            self.push_undo();
            let width = self.sim.width();
            self.sim.edit_cells(|grid_cells| {
                for y in y0c..=y1c {
                    for x in x0c..=x1c {
                        grid_cells[y * width + x] = 0;
                    }
                }
            });
            self.set_status("Selection cleared");
        }
    }
}
