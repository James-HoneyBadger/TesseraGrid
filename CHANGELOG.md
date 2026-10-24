# Changelog

All notable changes to LifeGrid are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [6.0.0] — 2026-10-24

### Changed
- Renamed the project to **Tessera Grid**. The crate is now `tessera-grid` and
  the binary/CLI is `tessera`; the window title, app id and release assets use
  the new name.
- Configuration now lives at `tessera-grid/config.json` and honours
  `TESSERA_CONFIG_DIR`. An existing
  `~/.config/lifegrid/lifegrid_config.json` is imported automatically on first
  launch, and `LIFEGRID_CONFIG_DIR` is still accepted.
- Saved grids use the `.tessera` extension; legacy `.lifegrid` files remain
  loadable.
- The crate directory is now `tessera-grid-rs/`.
- The GitHub repository is now `James-HoneyBadger/TesseraGrid`; GitHub redirects
  the previous `LifeGrid` URLs.

### Documentation
- README rewritten around the new name, with a migration guide, theme usage, a
  full CLI reference, updated project layout and refreshed screenshots.
- CONTRIBUTING now covers adding color themes, changing the versioned config,
  running the headless CLI, and keeping README/in-app help/`--help` in sync.
- The About dialog links to the repository, the help dialog links to the docs,
  and `tessera --help` includes usage examples.
- Added module-level documentation across the engine, UI and theme modules.

## [5.2.0] — 2026-10-24

### Added
- Eight color themes: **Dark**, **Light**, **Nord**, **Solarized Dark**,
  **Gruvbox**, **Dracula**, **High Contrast**, and **Paper**. Choose them from
  the View menu or the sidebar, or press `T` to cycle.
- Each theme defines its own canvas palette, grid line colour, panel/card
  colours, and status colours; light themes automatically darken mode accents
  for readable text.
- The pattern gallery previews and selection overlays follow the active theme.
- The theme is persisted in the config (`theme`, schema v2) and migrates from
  the legacy `dark_mode` flag; `dark_mode` is still written for older builds.

## [5.1.0] — 2026-10-24

### Added
- Headless CLI (`--headless`, `--mode`, `--rule`, `--pattern`, `--steps`,
  `--randomize`, `--seed`, `--png`, `--gif`, `--rle`, `--stats`, `--list-modes`).
- Pattern gallery with search, thumbnails, and descriptions; `Empty` is now
  available in every automaton.
- Seeded **Randomize** (button, sidebar seed field, `N`) with reproducible soups
  for every mode, including mode-specific densities.
- Brush sizes 1–8 with interpolated strokes so fast drags leave no gaps.
- Menu bar (File / Simulation / View / Help), About dialog, and a single
  shortcut table shared by the help window.
- Population chart with axes (egui_plot) and CSV export; full undo history now
  tracks the generation counter.
- RLE import/export from the menu and sidebar, with multi-state letters and
  correct per-mode rule headers.
- Persisted view mode, tool mode, brush size, isometric height, and panel width;
  config schema versioning with range clamping and readable load errors.
- `LIFEGRID_CONFIG_DIR` override plus proper macOS and Windows config paths.
- Test suite: automaton rule fixtures, boundary maths, engine undo/resize/frame
  caps, RLE and config round-trips, and end-to-end CLI export tests.

### Changed
- Upgraded `eframe`/`egui` 0.31 → 0.37 (`App::logic`/`App::ui`, `Panel`,
  `global_style`, glow backend with persistence), `rand` 0.8 → 0.10,
  `image` 0.25.10, `rayon` 1.12, `rfd` 0.17, `arboard` 3.7.
- Minimum supported Rust version is now **1.95**, pinned by `rust-toolchain.toml`.
- The simulation engine moved to `sim::Simulation` and is shared by the GUI and
  CLI; `app.rs` split into `app/{canvas,panels,toolbar,statusbar,dialogs,theme}`.
- Fit-to-window centers grids smaller than the viewport and respects the
  isometric view; `Ctrl+0` resets zoom.
- Paint state is clamped to each mode’s available states, and the boundary
  control is disabled for modes that always wrap.
- Run-N executes in short slices with a progress bar and cancel button instead of
  freezing the window.
- GIF buffering is bounded by total bytes (256 MB) rather than a fixed frame
  count; recording reports its memory footprint.
- CI and release workflows rewritten for Rust, including fmt, clippy
  (`-D warnings`), a three-OS test matrix, and per-OS release binaries.

### Fixed
- Simulation speeds above ~60 steps/s were unreachable because only one step ran
  per frame.
- `Ctrl+S` was a dead stub; it now saves state, `Ctrl+O` loads, and `Ctrl+E`
  exports PNG.
- Painting in Langton’s Ant mode had no visible effect until the next step.
- RLE exports always claimed `B3/S23` regardless of the active mode.
- Undo did not restore the generation and missed several mutation paths.
- Rainbow’s “Rainbow Mix” pattern was identical to random soup.
- High Life’s wildcard match made the `Empty` option unavailable.
- Panel width was persisted as a constant and never restored.
- Light-theme contrast on the canvas and noisy hover tooltips at small cell sizes.

### Removed
- Python-era CI, PyPI publishing, the Docker publish workflow (no Dockerfile
  existed), and the stale `.flake8`, `.pylintrc`, `.codecov.yml`, and
  `.dockerignore` files.

## [5.0.0] — 2026-09-24

### Added
- Native Rust/egui release build (`cargo build --release`).
- `run-simulation.sh` helper script at the repository root.

### Changed
- Documentation restored and enhanced as Markdown files.
- Version bumped to 5.0.0.

## [4.0.0] — 2026-05-15

### Changed
- Full rewrite in Rust. The entire codebase has been ported from Python/Tkinter to Rust using `eframe` and `egui` for the native GUI.
- All 10 automaton modes re-implemented as Rust structs behind a `Box<dyn Automaton>` trait object interface.
- Neighbour counting ported to pure Rust nested loops (no NumPy/SciPy dependency).
- Persistent config now stored as JSON at `~/.config/lifegrid/lifegrid_config.json`.
- Conway patterns hardcoded in `src/patterns.rs` (replaces the previous `patterns.json`).
- PNG export implemented with the `image` crate; GIF export records up to 500 frames.
- Undo/redo retained (100-state `VecDeque` in `core/undo.rs`).
- Three boundary modes retained: Wrap, Fixed, and Reflect.

### Removed
- Python source, CLI, REST/WebSocket API, plugin system, GPU acceleration, autosave, Sphinx docs, Makefile, Dockerfile, and all Python packaging files.

## [3.2.0] — 2026-02-19 (Python, archived)

### Added
- **Run N Steps** dialog (`Simulation → Run N Steps…`, shortcut `N`).
- **F5 Randomize** command fills the grid with a random soup.
- Scroll-wheel zoom adjusts cell size between 2 and 64 px.
- GIF and RLE export menu items wired to `ExportManager` and `RLEEncoder`.
- Boundary mode indicator in the window title bar.
- AutoSave manager started on app init and stopped on close.
- Command palette entries for all new actions.

### Fixed
- Boundary mode propagation across all 10 automata.
- `CellularAutomaton` double `reset()` call in `__init__`.
- `LifeLikeAutomaton` Moore-neighbourhood kernel moved to a class-level constant.
- `LangtonsAnt.get_population_grid()` no longer includes the ant-position marker.
- `copy_selection()` now captures all non-zero states, fixing multi-state copy/paste.
- `step_back()` timeline and population graph now synchronise correctly.
- Drag undo checkpoint creates one entry per drag gesture, not one per cell.
- `SimulationState.export_metrics_csv()` dynamically discovers all metric keys.
- `seen_hashes` memory capped at 2 000 entries with oldest-first eviction.

### Performance
- PIL fast-path renderer constructs a numpy RGB pixel array and issues a single canvas image call.
- `RuleDiscovery.observe_transition()` vectorised using `np.roll` stacking.

## [3.1.0] — 2025-06-17 (Python, archived)

### Added
- Headless CLI (`src/cli.py`).
- Boundary modes: wrap, fixed, reflect.
- Optional GPU acceleration via CuPy (`src/performance/gpu.py`).
- Generation timeline scrubber and population graph.
- Breakpoint system (pause on population/generation/density conditions).
- Rule explorer with 10 named rulesets.
- Command palette (`Ctrl+Shift+P`).
- Theme editor with 3 built-in presets.
- Pattern shape search.
- Collaborative WebSocket sessions.
- Hexagonal Life mode.
- 71-test suite in `tests/test_thorough.py`.

### Fixed
- `Simulator.undo()` and `Simulator.redo()` correctly pass `current_state` to `UndoManager`.
- GUI crash caused by mixing `pack` and `grid` geometry managers.
- `PopulationGraph._w` no longer shadows the internal `tk.Canvas._w` attribute.
- All flake8, mypy, and pylint issues resolved.

## [3.0.0] — 2025-06-01 (Python, archived)

### Added
- Modern Tkinter GUI with toolbar, status bar, and tabbed settings panel.
- 9 built-in automata modes.
- Drawing tools, pattern browser, light/dark themes, undo/redo.
- Statistics, heatmaps, symmetry detection, rule discovery.
- Export to PNG, GIF, MP4, WebM, JSON, CSV.
- Plugin system (`Day & Night` included).
- FastAPI REST + WebSocket API.

## [2.0.0] — 2025-04-15 (Python, archived)

### Added
- Multi-mode automata engine with shared `CellularAutomaton` base class.
- Grid rendering with zoom and pan.
- Pattern loading from JSON.
- Configuration via `SimulatorConfig` dataclass.
- Initial project scaffolding with `pyproject.toml`, `setup.py`, `Makefile`.

## [1.0.0] — 2025-02-01 (Python, archived)

### Added
- Initial release with Conway's Game of Life.
- Simple Tkinter canvas renderer.
- Start, stop, step, and reset controls.
