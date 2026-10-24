use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageBuffer, Rgb, Rgba, RgbaImage};

use crate::core::Grid;

/// RGB colour for each cell state (used for image export).
pub fn cell_color_rgb(state: u8) -> [u8; 3] {
    match state {
        0 => [18, 18, 26],
        1 => [220, 220, 255],
        2 => [255, 80, 80],
        3 => [255, 160, 0],
        4 => [240, 240, 0],
        5 => [0, 220, 100],
        6 => [60, 140, 255],
        7 => [200, 80, 255],
        8 => [140, 140, 160],
        _ => [100, 100, 120],
    }
}

/// Export the grid as a PNG file at `path`.
pub fn export_png(grid: &Grid, cell_size: u32, path: &std::path::Path) -> Result<(), String> {
    let w = (grid.width as u32) * cell_size;
    let h = (grid.height as u32) * cell_size;
    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(w, h);

    for gy in 0..grid.height {
        for gx in 0..grid.width {
            let color = cell_color_rgb(grid.get(gy, gx));
            let rgb = Rgb(color);
            let px = (gx as u32) * cell_size;
            let py = (gy as u32) * cell_size;
            for dy in 0..cell_size {
                for dx in 0..cell_size {
                    img.put_pixel(px + dx, py + dy, rgb);
                }
            }
        }
    }

    img.save(path).map_err(|e| e.to_string())
}

/// Export a sequence of grid snapshots as an animated GIF.
///
/// `frames`    — each element is a flat `Vec<u8>` of cell states (row-major).
/// `delay_ms`  — frame delay in milliseconds (e.g. 100 = 10 fps).
pub fn export_gif(
    frames: &[Vec<u8>],
    grid_w: usize,
    grid_h: usize,
    cell_size: u32,
    delay_ms: u32,
    path: &std::path::Path,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::BufWriter;

    if frames.is_empty() {
        return Err("No frames to export".into());
    }

    let file = File::create(path).map_err(|e| e.to_string())?;
    let writer = BufWriter::new(file);
    let mut encoder = GifEncoder::new_with_speed(writer, 10);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| e.to_string())?;

    let w = (grid_w as u32) * cell_size;
    let h = (grid_h as u32) * cell_size;
    let delay = Delay::from_numer_denom_ms(delay_ms, 1);

    for cells in frames {
        let mut img = RgbaImage::new(w, h);
        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let state = cells.get(gy * grid_w + gx).copied().unwrap_or(0);
                let [r, g, b] = cell_color_rgb(state);
                let px = (gx as u32) * cell_size;
                let py = (gy as u32) * cell_size;
                for dy in 0..cell_size {
                    for dx in 0..cell_size {
                        img.put_pixel(px + dx, py + dy, Rgba([r, g, b, 255]));
                    }
                }
            }
        }
        encoder
            .encode_frame(Frame::from_parts(img, 0, 0, delay))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
