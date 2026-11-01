//! Run-length encoded (RLE) pattern import and export.
//!
//! Supports the standard 2-state format (`b`, `o`, `.`) used by Golly and
//! LifeWiki as well as the multi-state extension (`A`..`X` for states 1..=24).

use crate::core::Grid;

/// Largest number of live cells a single document may decode to.
///
/// RLE is extremely compact — `2000000000o!` is a valid 13-byte document that
/// names two billion cells — so the parser must bound expansion. 8 Mi cells is
/// 96 MiB of `RlePattern` entries and far beyond any hand-drawn pattern.
pub const MAX_CELLS: usize = 8 * 1024 * 1024;

/// Largest run length accepted for a single token.
///
/// Longer runs almost always mean a malformed or hostile document; clamping
/// keeps the per-token work bounded as well.
pub const MAX_RUN: u32 = 1_000_000;

/// A parsed RLE pattern.
#[derive(Debug, Default, Clone)]
pub struct RlePattern {
    /// `(x, y, state)` for every live cell.
    pub cells: Vec<(i32, i32, u8)>,
    pub width: usize,
    pub height: usize,
    pub rule: Option<String>,
    /// `true` when the document asked for more cells than [`MAX_CELLS`] and was
    /// truncated, so callers can warn instead of silently dropping content.
    pub truncated: bool,
}

impl RlePattern {
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// Parse an RLE document. Comments (`#`) and the `x = …, y = …, rule = …` header
/// are optional; whitespace inside the body is ignored.
pub fn parse(source: &str) -> RlePattern {
    let mut width = None;
    let mut height = None;
    let mut rule = None;
    let mut body = String::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if (line.starts_with('x') || line.starts_with('X')) && line.contains('=') {
            for field in line.split(',') {
                let Some((key, value)) = field.split_once('=') else {
                    continue;
                };
                let value = value.trim();
                match key.trim().to_ascii_lowercase().as_str() {
                    "x" => width = value.parse().ok(),
                    "y" => height = value.parse().ok(),
                    "rule" if !value.is_empty() => rule = Some(value.to_owned()),
                    _ => {}
                }
            }
            continue;
        }
        body.push_str(line);
    }

    let mut cells = Vec::new();
    let mut x = 0i32;
    let mut y = 0i32;
    let mut count = 0u32;
    let mut has_count = false;
    let mut truncated = false;

    'document: for ch in body.chars() {
        match ch {
            '0'..='9' => {
                count = count
                    .saturating_mul(10)
                    .saturating_add(ch as u32 - '0' as u32);
                has_count = true;
            }
            'b' | '.' => {
                let run = take_count(&mut count, &mut has_count);
                if run > MAX_RUN {
                    truncated = true;
                    break 'document;
                }
                x = x.saturating_add(run as i32);
            }
            'o' | 'A'..='X' => {
                let run = take_count(&mut count, &mut has_count);
                if run > MAX_RUN {
                    truncated = true;
                    break 'document;
                }
                let state = if ch == 'o' { 1 } else { ch as u8 - b'A' + 1 };
                for _ in 0..run {
                    if cells.len() >= MAX_CELLS {
                        truncated = true;
                        break 'document;
                    }
                    cells.push((x, y, state));
                    x = x.saturating_add(1);
                }
            }
            '$' => {
                let run = take_count(&mut count, &mut has_count);
                if run > MAX_RUN {
                    truncated = true;
                    break 'document;
                }
                y = y.saturating_add(run as i32);
                x = 0;
            }
            '!' => break,
            _ => {
                count = 0;
                has_count = false;
            }
        }
    }

    let computed_w = cells.iter().map(|&(x, _, _)| x + 1).max().unwrap_or(0) as usize;
    let computed_h = cells.iter().map(|&(_, y, _)| y + 1).max().unwrap_or(0) as usize;

    RlePattern {
        cells,
        width: width.unwrap_or(computed_w),
        height: height.unwrap_or(computed_h),
        rule,
        truncated,
    }
}

/// Consume the pending run length.
///
/// Returns the raw count so callers can reject a run longer than [`MAX_RUN`].
/// A previous version cast it straight to `i32`, so a large count either
/// exploded the cell list or (past `i32::MAX`) wrapped negative.
fn take_count(count: &mut u32, has_count: &mut bool) -> u32 {
    let n = if *has_count { *count } else { 1 };
    *count = 0;
    *has_count = false;
    n.max(1)
}

/// Encode the grid as RLE using `rule` in the header.
pub fn encode(grid: &Grid, rule: &str) -> String {
    let Some((x0, y0, x1, y1)) = bounds(grid) else {
        return format!("x = 0, y = 0, rule = {rule}\n!");
    };

    let width = x1 - x0 + 1;
    let height = y1 - y0 + 1;
    let mut tokens: Vec<String> = Vec::new();
    let mut pending_empty_rows = 0usize;

    for y in y0..=y1 {
        let mut runs: Vec<(u8, u32)> = Vec::new();
        for x in x0..=x1 {
            let state = grid.get(y, x);
            match runs.last_mut() {
                Some((last_state, count)) if *last_state == state => *count += 1,
                _ => runs.push((state, 1)),
            }
        }
        while matches!(runs.last(), Some((0, _))) {
            runs.pop();
        }

        if runs.is_empty() {
            pending_empty_rows += 1;
            continue;
        }
        if pending_empty_rows > 0 {
            tokens.push(if pending_empty_rows == 1 {
                "$".to_owned()
            } else {
                format!("{pending_empty_rows}$")
            });
            pending_empty_rows = 0;
        }
        for (state, count) in runs {
            let ch = state_char(state);
            tokens.push(if count > 1 {
                format!("{count}{ch}")
            } else {
                ch.to_string()
            });
        }
        tokens.push("$".to_owned());
    }

    if tokens.last().is_some_and(|token| token == "$") {
        tokens.pop();
    }

    let mut out = format!("x = {width}, y = {height}, rule = {rule}\n");
    let mut line_len = 0usize;
    for token in tokens {
        if line_len + token.len() > 70 {
            out.push('\n');
            line_len = 0;
        }
        out.push_str(&token);
        line_len += token.len();
    }
    out.push('!');
    out
}

fn bounds(grid: &Grid) -> Option<(usize, usize, usize, usize)> {
    let mut found = false;
    let (mut x0, mut y0, mut x1, mut y1) = (grid.width, grid.height, 0, 0);
    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(y, x) != 0 {
                found = true;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    found.then_some((x0, y0, x1, y1))
}

fn state_char(state: u8) -> char {
    match state {
        0 => 'b',
        1 => 'o',
        2..=24 => (b'A' + (state - 1)) as char,
        _ => 'X',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_counts_and_rule() {
        let pattern = parse("#N Glider\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!");
        assert_eq!(pattern.width, 3);
        assert_eq!(pattern.height, 3);
        assert_eq!(pattern.rule.as_deref(), Some("B3/S23"));
        assert_eq!(
            pattern.cells,
            vec![(1, 0, 1), (2, 1, 1), (0, 2, 1), (1, 2, 1), (2, 2, 1)]
        );
    }

    #[test]
    fn parses_row_runs_and_multistate() {
        let pattern = parse("x = 4, y = 4, rule = B3/S23\n2$2.A$B2.!");
        assert_eq!(pattern.cells, vec![(2, 2, 1), (0, 3, 2)]);
    }

    #[test]
    fn encodes_glider_round_trip() {
        let mut grid = Grid::new(3, 3);
        for &(x, y, state) in &[(0, 0, 1), (1, 1, 1), (2, 2, 1), (1, 2, 1), (0, 2, 1)] {
            grid.set(y, x, state);
        }
        let encoded = encode(&grid, "B3/S23");
        assert!(encoded.starts_with("x = 3, y = 3, rule = B3/S23\n"));
        let parsed = parse(&encoded);
        assert_eq!(parsed.cells.len(), 5);
        assert_eq!(parsed.width, 3);
        assert_eq!(parsed.height, 3);
    }

    #[test]
    fn encodes_empty_grid() {
        let grid = Grid::new(5, 5);
        assert_eq!(encode(&grid, "B3/S23"), "x = 0, y = 0, rule = B3/S23\n!");
    }

    #[test]
    fn huge_run_lengths_are_capped() {
        // 13 bytes that used to request two billion cells.
        let pattern = parse("x = 1, y = 1, rule = B3/S23\n2000000000o!");
        assert!(
            pattern.cells.len() <= MAX_CELLS,
            "expanded to {} cells",
            pattern.cells.len()
        );
        assert!(pattern.truncated, "an oversized run must be reported");
    }

    #[test]
    fn run_lengths_that_would_wrap_negative_are_capped() {
        // A count beyond i32::MAX used to wrap to a negative run length.
        let pattern = parse("10000000000o!");
        assert!(pattern.truncated);
        assert!(pattern.cells.len() <= MAX_CELLS);
    }

    #[test]
    fn rows_longer_than_the_run_cap_are_truncated() {
        let pattern = parse("9999999999$o!");
        assert!(pattern.truncated);
    }

    #[test]
    fn many_legal_runs_still_cannot_exceed_the_cell_cap() {
        // Nine runs of MAX_RUN live cells each: legal individually, 9M in total.
        let run = format!("{MAX_RUN}o");
        let document = format!("{}{}", run.repeat(9), "!");
        let pattern = parse(&document);
        assert!(pattern.truncated);
        assert_eq!(pattern.cells.len(), MAX_CELLS);
    }

    #[test]
    fn normal_documents_are_not_marked_truncated() {
        let pattern = parse("x = 3, y = 3, rule = B3/S23\nbo$2bo$3o!");
        assert!(!pattern.truncated);
        assert_eq!(pattern.cells.len(), 5);
    }
}
