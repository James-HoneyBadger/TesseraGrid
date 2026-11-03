# Tessera Grid

[![CI](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/ci.yml/badge.svg)](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/ci.yml)
[![Release](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/release.yml/badge.svg)](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/release.yml)
[![Security audit](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/audit.yml/badge.svg)](https://github.com/James-HoneyBadger/TesseraGrid/actions/workflows/audit.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fast, native cellular-automaton simulator built with **Rust** and **egui**.

Tessera Grid runs eleven classic and custom automata in one interactive window — with a
pattern gallery, seeded random soups, custom B/S rules, an isometric 3-D view,
RLE import/export, PNG/GIF capture, a live population chart, a timeline scrubber,
an in-app pattern editor, and a headless CLI for scripted runs.

New here? Read the [6.x announcement](docs/announcing-tessera-grid.md) for the
rename, themes, timeline and pattern editor.

---

## Features

- **11 automata** — Conway, High Life, Hexagonal Life, Elementary 1D (all 256
  Wolfram rules), Immigration, Rainbow, Langton's Ant, Wireworld, Brian's Brain,
  Generations, and any custom B/S rule.
- **Direct manipulation** — paint with a 1–8 cell brush, interpolated strokes,
  right-click erase, region select/copy/cut/paste, wheel zoom, middle-drag pan.
- **Pattern gallery** — searchable list of every pattern for the current mode,
  with thumbnails and descriptions for the Conway set, plus `Empty` and seeded
  `Randomize` in every mode.
- **Custom rules** — type strings such as `B36/S23` or `B2/S`; the rule is
  applied on every keystroke, and unparseable text keeps the last valid rule
  instead of substituting a default. Neighbour counts above 8 are rejected.
- **Boundary modes** — wrap (toroidal), fixed (dead edges), reflect (mirror).
  Modes that ignore boundaries show the control disabled.
- **Isometric 3-D view** — height-mapped blocks with adjustable block height.
- **Aging overlay** — colour long-lived cells by age.
- **RLE** — import from and copy to the clipboard; exports carry the correct
  per-mode rule header and multi-state letters.
- **Export** — PNG snapshot or animated GIF (frames buffered to a 256 MB budget).
- **Timeline scrubbing** — step back through the last 512 generations (64 MB
  budget) and fork history by playing on from an earlier frame.
- **Pattern editor** — draw a draft pattern on a 16–48 cell canvas, rotate, flip
  or invert it, then stamp it into the grid or copy it as RLE.
- **Undo / redo** — 100 snapshots (capped at 128 MB) covering every edit,
  including generation and the ant's position.
- **Live statistics** — population chart with axes, peak/min/average, entropy,
  bounding box, cycle period, and CSV export.
- **Analysis panel** — isolated damage-spreading experiments (two copies one cell
  apart, plotted from generation zero) and cancellable, incremental sweeps of all
  256 elementary rules. Neither experiment changes the working grid.
- **Universal computation** — Rule 110 (Turing complete), plus the Game of Life,
  Wireworld (with a circulating-signal clock) and Langton's Ant, with the tooling
  to study them: seeded runs, cycle detection, parameter sweeps and
  sensitivity-to-perturbation experiments.
- **Session state** — version-3 `.tessera` JSON stores cells, auxiliary state,
  rules and boundary conditions for reproducible continuation. Legacy files use
  default settings with an explicit warning; UI preferences persist separately.
- **Multi-threaded stepping** — neighbour counting and cell updates run on all
  cores via Rayon.
- **Headless CLI** — drive the same engine from scripts and CI.
- **8 color themes** — Dark, Light, Nord, Solarized Dark, Gruvbox, Dracula,
  High Contrast, and Paper; switch instantly with `T`.
- **Cross-platform** — Linux, macOS, and Windows.

---

## Screenshots

| Main window | Isometric 3-D |
|-------------|---------------|
| ![Main window](docs/screenshots/main-window.png) | ![Isometric view](docs/screenshots/isometric-3d.png) |

| Pattern gallery | Statistics |
|-----------------|------------|
| ![Pattern gallery](docs/screenshots/pattern-gallery.png) | ![Population chart](docs/screenshots/statistics.png) |

![Keyboard shortcuts](docs/screenshots/shortcuts.png)

![Themes](docs/screenshots/themes.png)

*Nord, Solarized Dark, Gruvbox, Dracula, High Contrast and Paper.*

---

## Prerequisites

- **Rust 1.95 or newer** — the MSRV declared in `Cargo.toml` and required by
  egui 0.37. `rustup` installs it via [rust-toolchain.toml](rust-toolchain.toml).
- A C linker (`gcc` or `clang`).
- A display server (X11 or Wayland) **only for the GUI**; the CLI is headless.
- Linux build libraries:

  ```bash
  sudo apt-get install libxkbcommon-dev libwayland-dev libx11-dev \
      libudev-dev libgl1-mesa-dev pkg-config build-essential
  ```

  macOS needs Xcode Command Line Tools; Windows needs the MSVC build tools.

---

## Quick Start

```bash
git clone https://github.com/James-HoneyBadger/TesseraGrid.git
cd TesseraGrid
./run-simulation.sh
```

The script builds an optimised release binary before launching. Extra arguments
are forwarded to the binary, so the same script runs headless jobs:

```bash
./run-simulation.sh --steps 250 --png growth.png --stats
```

Or work directly in the crate:

```bash
cd tessera-grid-rs
cargo run --release
cargo build --release && ./target/release/tessera
```

---

## Headless CLI

The binary is `tessera`; run `tessera --help` for the generated reference. Passing
any option at all — `--steps`, an export path, or even just `--mode` — skips the
GUI entirely, which makes scripted runs, CI jobs and reproducible experiments
straightforward. With no arguments the window opens.

| Flag | Description |
|------|-------------|
| `--headless` | Never open a window |
| `--mode <MODE>` | Automaton mode (see `--list-modes`) |
| `--rule <RULE>` | Custom rule (`B36/S23`) or Wolfram code (`W110`); selects the matching mode |
| `--pattern <NAME>` | Starting pattern (defaults to the mode's first) |
| `--width`, `--height` | Grid size in cells (default 100) |
| `--boundary <wrap\|fixed\|reflect>` | Edge behaviour |
| `--steps <N>` | Generations to simulate (default 0; the flag alone implies headless) |
| `--randomize` | Fill with a random soup |
| `--seed <N>` | Seed for `--randomize` (implies it) |
| `--png <PATH>` | Write a PNG snapshot |
| `--gif <PATH>` | Record frames and write a GIF |
| `--gif-fps <N>` | GIF frame rate (default 10) |
| `--cell-size <N>` | Pixels per cell for PNG/GIF (default 4) |
| `--rle <PATH>` | Write the final grid as RLE |
| `--stats` | Print final statistics |
| `--analyze` | Print entropy, bounds, bounding box and cycle information |
| `--analysis-csv <PATH>` | Stream active-state metrics and run metadata as CSV; time series include generation 0 |
| `--divergence <N>` | Measure damage spreading from two copies of the configured initial state, for N positive steps |
| `--sweep-elementary` | Run all 256 elementary rules and print one metrics row each |
| `--replicates <N>` | Repeat a random run over 1–10,000 seeds and report mean and population standard deviation |
| `--list-modes` | List modes and exit |

Examples:

```bash
# Reproducible High Life run with PNG + GIF + RLE output
cargo run --release -- --rule B36/S23 --randomize --seed 42 \
    --steps 300 --png highlife.png --gif highlife.gif --rle highlife.rle

# Conway glider gun captured as a GIF
cargo run --release -- --pattern "Glider Gun" --steps 120 --gif gun.gif

# Machine-readable summary for scripts
cargo run --release -- --steps 100 --stats
```

---

## Usage

### Mouse

| Action | Input |
|--------|-------|
| Paint | Left-click / drag (respects brush size and paint state) |
| Erase | Right-click / drag |
| Zoom | Mouse wheel (centred on the cursor) |
| Pan | Middle-click drag, or arrow keys |
| Select a region | Select tool, then left-drag |
| Clear selection | Right-click (Select tool) |

### Keyboard

`Ctrl` below is `Cmd` on macOS.

| Action | Shortcut |
|--------|----------|
| Play / pause | `Space` |
| Step one generation | `S` |
| Reset to selected pattern | `R` |
| Randomize (current seed) | `N` |
| Toggle grid lines | `G` |
| Cycle color theme | `T` |
| Zoom to fit | `F` or `Ctrl+0` |
| Toggle isometric 3-D | `V` |
| Zoom in / out | `+` / `-` |
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Y` or `Ctrl+Shift+Z` |
| Copy / cut / paste selection | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |
| Clear selected cells | `Delete` |
| Deselect | `Escape` |
| Close a dialog | `Escape` |
| Save / open state | `Ctrl+S` / `Ctrl+O` |
| Export PNG | `Ctrl+E` |
| Pattern editor | `Ctrl+P` |
| Shortcut reference | `H` or `F1` |

### Themes

Choose a theme from **View → Theme** or the sidebar **View** section, or press
`T` to cycle. The eight themes are Dark, Light, Nord, Solarized Dark, Gruvbox,
Dracula, High Contrast and Paper. The choice is saved with the rest of the
configuration.

### Custom rules

Choose **Custom Rules** and type B/S notation, for example:

- `B36/S23` — High Life
- `B2/S` — Seeds
- `B3/S23` — Conway's Game of Life

### Saving and exporting

- **State files** (`.tessera`) are versioned JSON containing the mode, rules,
  boundary conditions, generation, size, cell buffer, and automaton state that
  lives outside the grid (Langton's Ant position and heading). Only canonical
  cell states are written — render-only markers never reach a file. Legacy
  `.lifegrid` files use default evolution settings with a warning; invalid cell
  states or auxiliary data are rejected rather than silently repaired.
- **RLE** uses standard Golly/LifeWiki syntax and carries the active rule in the
  header (`B36/S23`, `B2/S34H`, `WireWorld`, …). Imported documents are bounded:
  a single run may not exceed 1,000,000 cells and a document is capped at
  8,388,608, so a compact hostile file cannot exhaust memory. Truncation is
  reported in the status bar.
- **State files** are validated before loading: a size outside 1–4096 per side,
  a cell count that disagrees with the declared size, or a file written by a
  newer build is reported as an error rather than allocated.
- **GIF** recording keeps the newest frames within a 256 MB budget so long runs
  cannot exhaust memory.
- **Image export** is capped at 64 megapixels. A 1000×1000 grid at 64 px per cell
  would need about 12 GB of pixels, so the export is refused with a message
  instead of aborting the process.
- **Configuration** is stored at:
  - Linux: `$XDG_CONFIG_HOME/tessera-grid/` or `~/.config/tessera-grid/`
  - macOS: `~/Library/Application Support/tessera-grid/`
  - Windows: `%APPDATA%\tessera-grid\`
  - Override the directory with `TESSERA_CONFIG_DIR` (`LIFEGRID_CONFIG_DIR` is
    still honoured). An existing LifeGrid config is imported automatically.
  - The active theme, panel width, view mode, and brush size are saved here.

### Migrating from LifeGrid

Tessera Grid is the renamed LifeGrid project:

- The command-line binary is now `tessera` (crate `tessera-grid`).
- On first launch, `~/.config/lifegrid/lifegrid_config.json` is imported into
  `~/.config/tessera-grid/config.json`; the old file is left untouched.
- Valid legacy `.lifegrid` grids can still be opened with default evolution
  settings; new saves use `.tessera`.
- `LIFEGRID_CONFIG_DIR` continues to work alongside `TESSERA_CONFIG_DIR`.

---

## Computation and Analysis

Four built-in rules support universal computation in their idealized settings,
with suitable encodings and enough space. Finite grids are bounded
approximations, not demonstrations of arbitrary computation:

| System | Why it is universal | How to explore it |
|--------|--------------------|-------------------|
| Elementary 1D, rule 110 | Cook (2004) proved rule 110 simulates a universal Turing machine | `--rule W110`, or pick Elementary 1D and set rule 110 |
| Conway's Game of Life | Logic gates, counters and a universal machine have been built from gliders | Draw gates or import RLE from LifeWiki |
| Wireworld | Designed as an electronic-logic CA; gate libraries exist | The four states map directly onto wires, heads and tails |
| Langton's Ant | Supports universal computation with suitable encoded configurations | Explore trajectories; the familiar empty-plane highway is not a universal-machine encoding |

Rule 110 is a compact rule to explore: one binary row, three neighbours and an
8-bit rule number. [Cook's proof](https://www.complex-systems.com/abstracts/v15_i01_a01/)
establishes universality; a single-cell cone does not reproduce that encoding.
`tessera --rule W110 --width 160 --height 80 --steps 79 --analyze` prints
measurements of the newest row, while the GUI draws the space-time history.

### Measuring what you see

`--analyze` and the **Statistics** card measure active cells. In Elementary mode
this means the newest row, excluding diagram history; its bounds use y = 0.

- **Entropy** (`bits/cell`) — Shannon state-frequency entropy. A balanced
  checkerboard and balanced random binary grid both score one bit: this is not
  spatial complexity or a chaos classifier. A completely full binary grid scores
  zero, just like an empty one.
- **Bounds** — the bounding box of nonzero active cells.
- **Cycle** — exact equality of evolution states, including ant position and
  heading, is checked after hash matches. Generation zero is observed. Retention
  is bounded to 250,000 entries and 32 MiB of state payloads, plus entry overhead;
  "not observed" does not mean aperiodic. Finite deterministic systems eventually
  repeat, but that repeat may lie outside the retained window or run duration.
  The first retained occurrence is an upper bound on the transient when earlier
  states were discarded, not a claim about an unobserved part of the run.
- **Damage spreading** — `--divergence N` copies the configured initial state
  before `--steps`, changes one active cell, and prints every elapsed step
  including zero: distance, normalized damage fraction and source metadata.
  `ln(final distance) / N` is a descriptive finite-time log separation, not a
  rigorous Lyapunov exponent or proof of chaos. Extinction makes it undefined;
  one dying perturbation does not establish stability of the rule.

```bash
# Time series a scientist can plot
tessera --rule W110 --width 200 --height 100 --steps 99 \
    --analysis-csv rule110.csv --analyze

# Damage spreading for one initial state and one perturbation
tessera --pattern "Random Soup" --steps 0 --divergence 100 --seed 7

# All 256 elementary rules: measured outcomes, not inferred Wolfram classes
tessera --sweep-elementary --width 128 --height 96 --steps 95 \
    --analysis-csv elementary-sweep.csv
```

```bash
# Is the difference between two rules bigger than run-to-run noise?
tessera --replicates 20 --steps 200 --seed 1 --analysis-csv replicates.csv
```

The sweep uses a single-cell seed and reports fixed points, detected periods or
no observed repeat. Outcomes depend on width, boundary and observation duration.
Rules 30, 90 and 110 are useful comparisons, but state-frequency entropy alone
cannot assign a Wolfram class. All finite-grid runs eventually repeat.

Single runs are anecdotes. `--replicates N` runs the same configuration over `N`
seeds and reports the mean and standard deviation of population and entropy, plus
how many cycles were observed and with which periods. The reported standard
deviation divides by N; it is a description of these runs, not a confidence
interval or significance test. Replicates run one at a time to bound memory.

Time series, sweeps and replicates share a CSV schema: measurements, cycle
status, mode, rule, boundary, dimensions, initialization, seed, application
version and `metric_scope=active_cells`. Unknown periods are blank, not zero.
Time-series files contain N+1 samples for N steps. Random soups always report
their seed, including automatically generated seeds. Damage CSV uses a separate
shared GUI/CLI schema with the initial generation, snapshot hash, perturbation
coordinates and evolution settings; save the source state to replay GUI experiments.

Sweeps and replicates require positive `--steps`; divergence requires a positive
count. Incompatible flags are rejected instead of ignored: sweeps accept only
geometry, boundary, steps and CSV output; replicates additionally accept mode,
rule, seed and randomize, but not a pattern, divergence or media exports.
Langton's Ant has no randomized initializer, so randomization/replicates are
rejected for that mode. Invalid dimensions and export settings are errors rather
than silently clamped. Closed stdout pipes are harmless; other output failures
produce a nonzero exit status.

The GUI's damage experiment operates on independent copies, so editing or
playing the working simulation cannot contaminate its results. Stop or close the
Analysis window to cancel; partial observations remain available. Sweeps also
advance in short slices and can be stopped without blocking on all 256 rules.

### Wireworld devices

Two patterns are verified by simulation tests:

- **Clock** — a diamond ring of conductors with one electron. Exactly one head
  and one tail circulate forever, one cell per generation, returning to the
  starting state after `4 × radius` steps. Square corners would put a conductor
  diagonally next to the head and spawn a second signal, which is why the ring is
  a diamond.
- **Junction** — one stem wire meeting two diagonal branches. A pulse on the stem
  splits into both branches; a pulse on either branch reaches the stem. It needs
  at least 12×7 cells; smaller grids leave the pattern empty.

This junction is bidirectional. The requested diode/AND/OR library is not yet
implemented; future circuits need verified truth tables and pulse-timing tests.

---

## Supported Automata

| Mode | Rule | Boundary control |
|------|------|------------------|
| Conway's Game of Life | `B3/S23` | wrap / fixed / reflect |
| High Life | `B36/S23` | wrap / fixed / reflect |
| Hexagonal Life | `B2/S34H` (odd-r hex) | ignored (always wraps) |
| Elementary 1D | `W0`–`W255` (Wolfram code) | wrap / fixed / reflect |
| Immigration Game | three-colour Life; newborns take the majority colour | wrap / fixed / reflect |
| Rainbow Game | six-colour Life; newborns blend the average colour | wrap / fixed / reflect |
| Langton's Ant | `LangtonsAnt` | ignored (always wraps) |
| Wireworld | `WireWorld` | wrap / fixed / reflect |
| Brian's Brain | `Brian'sBrain` | wrap / fixed / reflect |
| Generations | `B3/S23/C8` | wrap / fixed / reflect |
| Custom Rules | any `B…/S…` | wrap / fixed / reflect |

---

## Project Structure

```text
TesseraGrid/              # repository name
├── tessera-grid-rs/       # crate: tessera-grid, binary: tessera
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # library root: engine + UI, used by tests/benches
│   │   ├── main.rs          # thin entry point: CLI dispatch + GUI launch
│   │   ├── cli/             # headless command line
│   │   │   ├── mod.rs       # shared setup and CLI tests
│   │   │   ├── args.rs      # clap argument definitions
│   │   │   └── run.rs       # headless run
│   │   ├── app/             # egui layer (no engine logic)
│   │   │   ├── mod.rs       # shared state, enums, constructor
│   │   │   ├── shell.rs     # eframe::App: frame loop, layout, exit
│   │   │   ├── actions.rs   # step, reset, randomize, undo
│   │   │   ├── input.rs     # keyboard shortcuts
│   │   │   ├── io.rs        # PNG/GIF/RLE/CSV/state file I/O
│   │   │   ├── selection.rs # copy, cut, paste, clear
│   │   │   ├── config.rs    # settings persistence
│   │   │   ├── menu_bar.rs  # File/Edit/Run/View/Help menus
│   │   │   ├── toolbar.rs   # quick-action toolbar
│   │   │   ├── statusbar.rs # status text and FPS counters
│   │   │   ├── timeline.rs  # generation scrubber bar
│   │   │   ├── theme.rs     # applies ThemeColors to egui
│   │   │   ├── canvas/      # flat and isometric rendering
│   │   │   │   ├── mod.rs   # view dispatch
│   │   │   │   ├── flat.rs  # 2-D grid, brush, selection
│   │   │   │   ├── iso.rs   # isometric 3-D view
│   │   │   │   ├── paint.rs # Bresenham brush strokes
│   │   │   │   └── viewport.rs # zoom-to-fit helpers
│   │   │   ├── sidebar/     # control cards
│   │   │   │   ├── mod.rs   # card styling and composition
│   │   │   │   ├── mode_info.rs
│   │   │   │   ├── automaton.rs
│   │   │   │   ├── view.rs
│   │   │   │   ├── statistics.rs
│   │   │   │   ├── export.rs
│   │   │   │   └── selection.rs
│   │   │   └── dialogs/     # floating dialogs
│   │   │       ├── mod.rs
│   │   │       ├── help.rs
│   │   │       ├── gallery.rs
│   │   │       ├── editor.rs   # pattern editor
│   │   │       ├── run_n.rs
│   │   │       ├── resize.rs
│   │   │       └── about.rs
│   │   ├── theme/           # color themes
│   │   │   ├── mod.rs       # Theme enum, names, accents
│   │   │   └── palettes.rs  # per-theme palettes
│   │   ├── sim.rs           # headless simulation controller
│   │   ├── rle.rs           # RLE parser/encoder
│   │   ├── patterns.rs      # built-in Conway patterns
│   │   ├── export.rs        # PNG and GIF rendering
│   │   ├── automata/        # 10 automata behind one trait, Mode enum + tests
│   │   └── core/            # grid, neighbour counting, boundary, undo, config
│   ├── deny.toml            # cargo-deny policy (advisories, licenses)
│   └── tests/
│       ├── engine.rs        # in-process engine tests
│       └── headless.rs      # end-to-end CLI tests
├── .github/workflows/       # CI, security audit and release pipelines
├── docs/
│   ├── announcing-tessera-grid.md  # 6.x release post
│   └── screenshots/         # images referenced by the docs
├── run-simulation.sh        # build + launch helper (forwards CLI flags)
├── rust-toolchain.toml      # pinned Rust channel/components
├── CHANGELOG.md
├── CONTRIBUTING.md
└── CODE_OF_CONDUCT.md
```

---

## Development

```bash
cd tessera-grid-rs
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
cargo deny check          # advisories, licenses, banned crates
cargo llvm-cov --all-targets --lcov --output-path lcov.info   # optional coverage
```

Unit tests live next to the code (`src/automata/tests.rs`, `src/sim.rs`,
`src/rle.rs`, `src/core/grid.rs`, `src/core/undo.rs`, `src/core/config.rs`,
`src/theme/mod.rs`, `src/app/selection.rs`, `src/app/io.rs`).
`tests/engine.rs` drives the library in process, and `tests/headless.rs` runs the
real binary end to end.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow.

---

## Roadmap

- [x] Multi-threaded grid stepping (Rayon)
- [x] In-app pattern editor
- [x] Timeline scrubber over recorded history
- [ ] GPU (`wgpu` compute) stepping backend
- [ ] Runtime plugin loading for custom automata
- [ ] RLE multistate authoring in the pattern editor
- [ ] MP4/WebM export

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

MIT — see [LICENSE](LICENSE).
