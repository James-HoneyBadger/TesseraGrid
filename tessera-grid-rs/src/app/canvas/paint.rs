//! Brush painting shared by the flat and isometric canvases.

use super::*;

/// Cells along a Bresenham line, used to avoid gaps during fast drags.
fn line_cells(from: (i64, i64), to: (i64, i64)) -> Vec<(i64, i64)> {
    let (mut x, mut y) = from;
    let (x1, y1) = to;
    let dx = (x1 - x).abs();
    let dy = (y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut out = Vec::new();
    loop {
        out.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    out
}

impl TesseraGridApp {
    /// Paint a brush stroke along the line between two cells.
    pub(super) fn paint_line(&mut self, from: Option<(i64, i64)>, to: (i64, i64), state: u8) {
        let (gw, gh) = (self.sim.width() as i64, self.sim.height() as i64);
        let radius = (self.brush_size.max(1) as i64 - 1) / 2;
        for (cx, cy) in line_cells(from.unwrap_or(to), to) {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && y >= 0 && x < gw && y < gh {
                        self.sim.paint(y as usize, x as usize, state);
                    }
                }
            }
        }
    }
}
