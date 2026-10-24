# Contributing to Tessera Grid

Thanks for your interest in contributing! This guide covers setup, workflow, and standards for Tessera Grid (crate `tessera-grid`, binary `tessera`).

---

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) **1.95 or newer** (pinned by
  [`rust-toolchain.toml`](rust-toolchain.toml); the GUI stack requires it)
- A C linker (`gcc` or `clang`)
- Linux GUI headers: `sudo apt-get install libxkbcommon-dev libwayland-dev
  libx11-dev libudev-dev libgl1-mesa-dev pkg-config build-essential`
- macOS: Xcode Command Line Tools
- A display server (X11 or Wayland) to run the GUI; the CLI runs headless

### Clone and Build

```bash
git clone https://github.com/James-HoneyBadger/TesseraGrid.git
cd TesseraGrid/tessera-grid-rs
cargo build
```

Run in development mode:

```bash
cargo run
```

Run the headless CLI from the crate (everything after `--` goes to `tessera`):

```bash
cargo run -- --headless --steps 100 --stats
```

Build an optimised release binary:

```bash
cargo build --release
./target/release/tessera
```

---

## Workflow

1. **Fork** the repository and clone your fork.
2. **Create a branch** from `master`:

   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make your changes** – keep commits focused and well-described.
4. **Run checks** before submitting:

   ```bash
   cargo fmt --all --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all-targets
   ```

5. **Open a pull request** against `master`.

---

## Code Standards

### Formatting

- Run `cargo fmt` before committing.
- Use the default rustfmt style. No unformatted code is accepted.

### Linting

- CI fails on any warning: `cargo clippy --all-targets -- -D warnings`.
- Use `#[allow(...)]` only with a comment explaining why the lint is suppressed.

### Tests

- Automaton rules go in `src/automata/tests.rs`; engine behaviour in the
  `#[cfg(test)]` module of `src/sim.rs`; parsing/serialisation next to the code
  (`rle.rs`, `core/undo.rs`, `core/config.rs`).
- End-to-end CLI coverage lives in `tests/headless.rs` and runs the real binary.
- Run everything with `cargo test --all-targets`.

### Safety

- Avoid `unsafe` blocks. The codebase does not use any.

### Documentation

- Add doc comments (`///`) for public items and `//!` module summaries for new modules.
- Update `README.md`, `CHANGELOG.md`, and this file when behaviour changes.
- Keep shortcut text in sync: `SHORTCUTS` in `src/app/dialogs/help.rs`, the
  in-app help dialog, and the keyboard table in `README.md`.
- CLI flags live in `src/cli.rs`; `tessera --help` is generated, so keep the
  README flag table aligned with it.
- Screenshots live in `docs/screenshots/` and are referenced from `README.md`.

---

## Adding an Automaton Mode

1. Create `tessera-grid-rs/src/automata/<name>.rs` implementing the `Automaton` trait:

   ```rust
   use crate::core::{BoundaryMode, Grid};
   use super::Automaton;

   pub struct MyAutomaton {
       grid: Grid,
       boundary: BoundaryMode,
   }

   impl MyAutomaton {
       pub fn new(width: usize, height: usize) -> Self {
           Self { grid: Grid::new(width, height), boundary: BoundaryMode::Wrap }
       }
   }

   impl Automaton for MyAutomaton {
       fn name(&self) -> &'static str { "My Automaton" }
       fn step(&mut self) { /* implement rules via self.grid */ }
       fn reset(&mut self) { self.grid.clear(); }
       fn get_grid(&self) -> &Grid { &self.grid }
       fn get_grid_mut(&mut self) -> &mut Grid { &mut self.grid }
       fn load_pattern(&mut self, pattern: &str) {
           self.grid.clear();
           if pattern == "Empty" { return; }
           // otherwise fill a soup or a named pattern
       }
       fn available_patterns(&self) -> &'static [&'static str] {
           &["Random Soup", "Empty"]
       }
       fn boundary(&self) -> BoundaryMode { self.boundary }
       fn set_boundary(&mut self, mode: BoundaryMode) { self.boundary = mode; }

       // Optional metadata used by RLE export and the UI:
       fn rule_string(&self) -> String { "B3/S23".to_owned() }
       fn max_state(&self) -> u8 { 1 }
       fn supports_boundary(&self) -> bool { true }
   }
   ```

   Grid mutation from the UI always goes through `set_cells`, `edit_cells`, and
   `paint`. Override those only if the automaton keeps auxiliary state (see
   `Langton's Ant`); the default implementations are correct for plain grids.

2. Add `mod <name>;` and `pub use <name>::MyAutomaton;` in `automata/mod.rs`.
3. Add the display name to `ALL_MODES`, a match arm in `make_automaton`, and a
   short description in `description()`.
4. Add a next-generation test in `src/automata/tests.rs` and make sure the
   generic "Empty + deterministic randomize" test still passes.
5. Update `README.md` and `CHANGELOG.md`.

---

## Adding a Color Theme

1. Add a variant to `Theme` in `tessera-grid-rs/src/theme/mod.rs`.
2. Handle it in `name()`, `from_name()` (add aliases if useful), `is_dark()` and
   `theme/palettes.rs::colors()`.
3. The palette needs window/panel/card/bar, stroke, faint, extreme, grid and
   success colours, plus a nine-entry cell palette where index 0 is the canvas
   background and 1–8 are the cell states.
4. Run the theme tests: they check unique names, name round-tripping,
   dark/light accent contrast, and that every palette stays readable.
5. Update the theme list in `README.md` and, if the visual set changed,
   regenerate `docs/screenshots/themes.png`.

---

## Changing Configuration

`AppConfig` in `tessera-grid-rs/src/core/config.rs` is versioned and must keep
loading older files:

1. Add fields with `#[serde(default)]` (or a default function).
2. Normalise and clamp them in `sanitize()`.
3. Bump `CONFIG_VERSION` and add a migration test next to the existing ones.
4. Update the configuration section of `README.md`.

---

## Project Layout

| Path | Purpose |
|------|---------|
| `tessera-grid-rs/src/main.rs` | CLI dispatch and GUI launch |
| `tessera-grid-rs/src/cli.rs` | Headless command-line interface |
| `tessera-grid-rs/src/app/` | egui shell, actions, input, I/O, canvas, sidebar, dialogs |
| `tessera-grid-rs/src/sim.rs` | Headless simulation controller (engine + undo + stats) |
| `tessera-grid-rs/src/rle.rs` | RLE import/export |
| `tessera-grid-rs/src/theme/` | Theme enum, accents and per-theme palettes |
| `tessera-grid-rs/src/automata/` | Automaton trait, implementations, rule tests |
| `tessera-grid-rs/src/core/` | Grid, boundary modes, undo manager, config |
| `tessera-grid-rs/src/export.rs` | PNG and GIF rendering |
| `tessera-grid-rs/src/patterns.rs` | Built-in Conway pattern data |
| `tessera-grid-rs/tests/` | End-to-end CLI tests |
| `docs/screenshots/` | Screenshots embedded in the README |

---

## Reporting Issues

Open a GitHub issue with:

- A clear title and description
- Steps to reproduce (for bugs)
- Expected vs. actual behaviour
- Rust version (`rustc --version`) and OS
- Screenshots or GIFs if they help illustrate the problem

---

## Pull Request Checklist

- [ ] Branch is up to date with `master`
- [ ] `cargo test --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes
- [ ] Documentation is updated where necessary
- [ ] CHANGELOG.md is updated for user-facing changes

---

## License

By contributing you agree that your contributions will be licensed under the [MIT License](LICENSE).
