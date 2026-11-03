# Changelog

All notable changes to Tessera Grid are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Elementary 1D mode**: all 256 Wolfram rules rendered as a space-time diagram,
  with a rule slider, presets (30, 90, 110, 184) and `--rule W110` on the CLI.
  Rule 110 is Turing complete, which gives the simulator a self-contained
  universal-computation demonstration. Only the newest row counts as population
  and carries an age, so the statistics stay meaningful.
- **`core::analysis`**: Shannon entropy, Hamming distance, bounding box, a
  bounded repeated-state detector (transient + period), and `Simulation::metrics`.
- **Analysis tooling**: the Statistics card shows entropy, bounds and cycle
  period; `--analyze` prints a summary, `--analysis-csv` writes a per-generation
  time series, and `--divergence N` measures how fast a one-cell perturbation
  spreads (a Lyapunov-style growth rate).
- `NeighborBuffers`: neighbour-counting scratch buffers that are reused across
  generations instead of allocating several grid-sized vectors per step.
- `cargo bench`: a dependency-free throughput table for the neighbour passes,
  Hexagonal Life and a full `Simulation::step`.
- CI jobs for the declared MSRV (`1.95`), a release-mode test run (different
  overflow semantics), and a benchmark smoke run.
- `Automaton::marker`, a render-only overlay hook, plus `Simulation::display_cells`
  / `display_grid` for drawing and image export.
- `Automaton::active_rows`, so space-time automata can declare which rows are live
  state rather than finished history.

### Changed
- **Stepping is faster.** The per-generation age/population pass is now parallel,
  neighbour buffers are reused, life-like rule membership is a bitmask lookup, and
  Hexagonal Life uses precomputed wrap tables (measured at 1000×1000: full step
  2.59 ms → 1.41 ms, Hexagonal Life 4.90 ms → 1.55 ms).
- Timeline snapshots are opt-in (`Simulation::record_timeline`) and off for CLI
  runs; the GUI enables them for the scrubber.
- The isometric view draws every block into one `egui::Mesh`, culls off-screen
  blocks and only re-sorts live cells when the grid changes.
- Editing a custom rule updates the automaton in place: the grid and undo history
  are no longer discarded on every keystroke. The resize dialog validates its
  input instead of substituting `100`, and grid limits are one pair of constants
  (`1`–`4096`) shared by the config, the CLI and the UI.
- A corrupt `config.json` is copied to `config.json.bak` before defaults are used.
- `tool_mode` is restored from the config, and an unknown `automaton_mode` falls
  back to Conway instead of leaving the UI on a mode the engine never built.

### Fixed
- **Saving and loading a Langton's Ant game crashed the next step.** State files
  stored the display grid (with the ant drawn as state 2) and dropped the ant's
  position, so loading wrote the marker into the simulation; `1 - current`
  underflowed. Saved files now store canonical cells plus the ant's position and
  heading, and `Automaton::set_cells`/`edit_cells`/`paint` clamp every state to
  `max_state()`.
- **Langton's Ant population was off by one** and the aging overlay highlighted
  the ant, because the render marker counted as a live cell.
- RLE export no longer writes the ant marker as a cell.
- `--cell-size`, `--gif-fps` and `--steps 0` now run headless instead of opening
  the window and ignoring the flag.
- PNG/GIF export refuses images over 64 megapixels instead of aborting on a
  multi-gigabyte allocation.

## [6.2.0] — 2026-10-31

### Added
- A `lib.rs` target, so the engine can be exercised from integration tests and
  benchmarks without spawning the `tessera` binary.
- `tests/engine.rs` covering population tracking, undo/redo, timeline scrubbing,
  render-frame reuse and grid-size clamping.
- CLI tests for headless detection, default geometry and boundary parsing.

### Fixed
- **Undo, redo and the timeline corrupted Langton's Ant.** Snapshots stored the
  *display* grid, which draws the ant as state 2; restoring one wrote that marker
  into the simulation and the next step underflowed `1 - 2`. Snapshots now store
  the automaton's canonical state and the ant position/heading, so replaying from
  an undone state retraces the same history.
- **Copy/cut/clear could panic** after dragging a selection outside the grid:
  negative coordinates were cast to `usize`, producing a region of ~1.8e19 cells.
  Selections are now clamped to the grid, and the region helpers clamp again
  before indexing.
- **`Ctrl+S` stepped the simulation** and **`Ctrl+V` also toggled the 3-D view**,
  because egui's `key_pressed` ignores modifiers. Chords now consume their key
  before the plain-key bindings run.
- **Shortcuts were dead on macOS**: bindings used `modifiers.ctrl` instead of
  `Modifiers::COMMAND`, so Cmd+C/S/Z never matched. The help dialog and README
  now show the platform modifier.
- Shortcuts no longer mutate the simulation while a dialog is open; `Escape`
  closes the topmost dialog instead.
- **Loading a `.tessera` file trusted its header.** A file declaring a billion
  rows aborted the process on allocation; width, height and cell count are now
  validated against the engine limit before anything is allocated.
- **RLE import had unbounded expansion**: a 13-byte document such as
  `2000000000o!` decoded to two billion cells (24 GB). Parsing now rejects runs
  longer than a million cells, caps documents at 8 Mi cells, and reports
  truncation. Counts past `i32::MAX` used to wrap negative.
- `--rule` now rejects impossible neighbour counts (above 8) and malformed input
  instead of silently simulating a different rule; the rule editor keeps the
  last valid rule while the text is unparseable.
- **Configuration flags were silently ignored**: `tessera --mode "High Life"`
  opened the GUI without applying the mode, because only output flags triggered
  a headless run. Any option now counts, matching what `--help` promises.

### Changed
- Neighbour counting is ~5× faster. `Grid::neighbor_counts` and
  `neighbor_stats_alive` replace per-cell division, modulo and eight boundary
  lookups with two additive row-major passes (horizontal, then vertical), still
  parallelised across row bands. A 1000×1000 Conway step dropped from 7.5 ms to
  2.5 ms.
- The canvas no longer clones the cell and age grids every frame. Buffers are
  handed to the view from a revision-tagged cache and returned afterwards, and
  the population shown in the status bar is tracked incrementally.
- Recorded GIF frames use a `VecDeque`, so trimming the frame budget is O(1)
  instead of shifting the whole vector.
- Undo history has a 128 MiB byte budget in addition to its snapshot count.
- Langton's Ant updates two cells per step instead of copying the whole grid.
- Grid dimensions are clamped inside `sim::make_automaton`, covering config,
  CLI, resize and state-file paths from one place.
- Mode dispatch goes through a `Mode` enum, so adding an automaton forces every
  `match` to be updated instead of falling back to Conway.
- The duplicated brush/selection handling in the two canvases is shared, and
  grid rows are reset through one `reset_derived_state` helper.

## [6.1.0] — 2026-10-24

### Added
- **Timeline scrubber** below the canvas: the last 512 generations (64 MB
  budget) are recorded as you step, can be inspected by dragging the slider, and
  "Return to live" jumps back to the newest frame. Playing on from a scrubbed
  position forks the history instead of overwriting the future.
- **Pattern editor** (`Ctrl+P`): draw on a 16/24/32/48-cell draft, rotate, flip
  or invert it, then stamp it into the centre of the grid or copy it as RLE.
- `cargo deny` policy (`tessera-grid-rs/deny.toml`) covering advisories,
  licenses, bans and sources, checked by a weekly security-audit workflow.
- Coverage job in CI (`cargo llvm-cov`) publishing an `lcov.info` artifact.
- Issue forms, a pull-request template and a 6.x announcement post
  (`docs/announcing-tessera-grid.md`).

### Changed
- Stepping is multi-threaded end to end: neighbour counting and the per-cell
  update run through Rayon (`Grid::map_cells`, `Grid::update_cells`), and the
  per-step grid copy is gone.
- Keyboard shortcuts keep working after clicking buttons or the timeline
  slider; only text fields capture the keyboard, and `Space` still defers to a
  focused button.

### Fixed
- Updated `crossbeam-epoch` (RUSTSEC-2026-0204) and the yanked
  `clipboard-win` release; unmaintained/advisory exceptions are documented in
  `deny.toml`.

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
- Split the egui layer into focused modules: `app/mod.rs` now only holds shared
  state and constructors, with behaviour in `actions`, `input`, `io`,
  `selection`, `config`, `menu_bar`, `toolbar`, `statusbar` and `shell`.
- Canvas rendering moved to `app/canvas/` (`flat`, `iso`, `paint`, `viewport`),
  the sidebar to `app/sidebar/` (one module per card) and the modal dialogs to
  `app/dialogs/` (one module per dialog).
- Theme palettes moved to `src/theme/palettes.rs`; `Theme` itself stays in
  `src/theme/mod.rs`.
- The headless CLI is split into `src/cli/args.rs` (clap definitions) and
  `src/cli/run.rs` (execution).

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
