# Announcing Tessera Grid

*LifeGrid is now Tessera Grid — a Rust-native cellular-automata workbench with
themes, a timeline you can scrub, a pattern editor and a headless CLI.*

![Tessera Grid main window](screenshots/main-window.png)

## The rename

The project started as a Python/Tkinter experiment called **LifeGrid** and grew
into a Rust/egui desktop application. The old name described only Conway's Game
of Life, while the simulator now hosts ten different automata — Life-like rules,
Hexagonal Life, Immigration and Rainbow games, Langton's Ant, Wireworld, Brian's
Brain and Generations.

**Tessera Grid** keeps the idea of tiles (*tessera* is Latin for a mosaic tile)
without tying the app to a single rule set. The rename covers the crate
(`tessera-grid`), the binary (`tessera`), the window title and the repository.

Nothing is lost in the move:

- Settings are imported from `~/.config/lifegrid/lifegrid_config.json` on first
  launch and written to `~/.config/tessera-grid/config.json` afterwards.
- Saved grids use the `.tessera` extension, and legacy `.lifegrid` files still
  load.
- `LIFEGRID_CONFIG_DIR` is still honoured alongside `TESSERA_CONFIG_DIR`.

## What's new in 6.x

### Eight colour themes

Dark, Light, Nord, Solarized Dark, Gruvbox, Dracula, High Contrast and Paper.
Each theme carries its own canvas palette, grid lines, panel and card colours,
so light themes stay readable and high contrast stays usable. Cycle with `T`,
or pick one from the View menu or the sidebar.

![Themes](screenshots/themes.png)

### Timeline scrubbing

Every generation is recorded into a bounded history (512 steps or 64 MB by
default). The scrubber under the canvas steps back through the recent past,
pauses the simulation while you inspect a frame, and **Return to live** jumps
back to the present. Stepping from a scrubbed position forks the timeline, so
experiments never silently overwrite what you were looking at.

### Pattern editor

`Ctrl+P` opens a small drafting grid. Draw with the mouse, then rotate, flip or
invert the draft and either stamp it into the centre of the live grid or copy it
out as RLE. It is a quick way to test a reaction without hand-painting cells on
a 100×100 canvas.

### Multi-threaded stepping

Neighbour counting and the per-cell update are both spread across cores with
Rayon, and the per-step grid copy is gone. On a 4-core machine, a 600×600 Conway
run is roughly twice as fast as the same workload pinned to a single thread.
A GPU (`wgpu` compute) backend remains on the roadmap.

### Headless first

The engine has no UI dependency, so every mode is scriptable:

```bash
tessera --headless --steps 100 --png out.png
tessera --rule B36/S23 --randomize --seed 42 --gif out.gif --stats
tessera --list-modes
```

## Getting it

Grab a binary from the [releases page](https://github.com/James-HoneyBadger/TesseraGrid/releases):

| Platform | Asset |
|----------|-------|
| Linux x86_64 | `tessera-linux-x86_64` |
| macOS (Apple silicon) | `tessera-macos-arm64` |
| Windows x86_64 | `tessera-windows-x86_64.exe` |

Or build from source:

```bash
git clone https://github.com/James-HoneyBadger/TesseraGrid
cd TesseraGrid/tessera-grid-rs
cargo build --release --locked
```

## Under the hood

The egui layer is split into focused modules — `canvas/` for flat and isometric
rendering, `sidebar/` with one module per card, `dialogs/` per dialog, plus
`actions`, `input`, `io` and `shell` — while `src/sim.rs` stays a headless
controller that the CLI and the GUI share. CI runs formatting, Clippy, tests on
Linux/macOS/Windows, a coverage job and `cargo deny` for advisories and licenses.

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the module map and
[CHANGELOG.md](../CHANGELOG.md) for the full history.
