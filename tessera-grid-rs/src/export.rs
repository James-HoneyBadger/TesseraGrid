//! PNG snapshot and animated GIF rendering.

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageBuffer, Rgb, Rgba, RgbaImage};

use crate::core::Grid;

/// Largest image, in pixels, that an export may produce.
///
/// A 1000×1000 grid at 64 px per cell would be 64000×64000 pixels — about 12 GB
/// for the RGB buffer, which aborts the process on allocation failure. Refusing
/// up front turns that into an actionable message.
pub const MAX_EXPORT_PIXELS: u64 = 64 * 1024 * 1024;

/// Validate the requested image size before any buffer is allocated.
fn export_dimensions(grid_w: usize, grid_h: usize, cell_size: u32) -> Result<(u32, u32), String> {
    if cell_size == 0 {
        return Err("cell size must be at least 1 pixel".to_owned());
    }
    let w = u64::try_from(grid_w).unwrap_or(u64::MAX) * u64::from(cell_size);
    let h = u64::try_from(grid_h).unwrap_or(u64::MAX) * u64::from(cell_size);
    let pixels = w.saturating_mul(h);
    if pixels > MAX_EXPORT_PIXELS {
        return Err(format!(
            "{}×{} pixels is over the {} MP export limit; \
             use a smaller cell size ({}×{} grid at {} px/cell)",
            w,
            h,
            MAX_EXPORT_PIXELS / (1024 * 1024),
            grid_w,
            grid_h,
            cell_size
        ));
    }
    Ok((
        u32::try_from(w).map_err(|_| "image width is too large".to_owned())?,
        u32::try_from(h).map_err(|_| "image height is too large".to_owned())?,
    ))
}

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
    let (w, h) = export_dimensions(grid.width, grid.height, cell_size)?;
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

    let (w, h) = export_dimensions(grid_w, grid_h, cell_size)?;

    let file = File::create(path).map_err(|e| e.to_string())?;
    let writer = BufWriter::new(file);
    let mut encoder = GifEncoder::new_with_speed(writer, 10);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| e.to_string())?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_images_are_rejected_before_allocating() {
        // 1000x1000 at 64 px/cell is 12 GB of pixels.
        let err = export_dimensions(1000, 1000, 64).expect_err("must be rejected");
        assert!(err.contains("export limit"), "{err}");

        // The engine maximum at the maximum cell size is far past any budget.
        let err = export_dimensions(4096, 4096, 64).expect_err("must be rejected");
        assert!(err.contains("export limit"), "{err}");
    }

    #[test]
    fn reasonable_images_are_accepted() {
        assert_eq!(export_dimensions(100, 80, 4).unwrap(), (400, 320));
        assert_eq!(export_dimensions(1, 1, 1).unwrap(), (1, 1));
        assert!(export_dimensions(10, 10, 0).is_err(), "zero cell size");
    }
}
