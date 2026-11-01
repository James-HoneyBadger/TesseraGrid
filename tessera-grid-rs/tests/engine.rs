//! In-process engine checks.
//!
//! These use the library directly (rather than spawning the `tessera` binary)
//! so they can assert on the derived state that the UI relies on.

use tessera_grid::core::BoundaryMode;
use tessera_grid::sim::{RuleSpec, Simulation, MAX_GRID_SIDE};

fn conway(width: usize, height: usize) -> Simulation {
    Simulation::new(
        "Conway's Game of Life",
        width,
        height,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        "Empty",
    )
}

fn ant(width: usize, height: usize) -> Simulation {
    Simulation::new(
        "Langton's Ant",
        width,
        height,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        "Empty",
    )
}

#[test]
fn population_tracks_every_mutation_path() {
    let mut sim = conway(30, 30);
    assert_eq!(sim.population(), 0);

    sim.paint(1, 1, 1);
    sim.paint(1, 2, 1);
    assert_eq!(sim.population(), 2);

    // Painting the same state again must not double-count.
    sim.paint(1, 1, 1);
    assert_eq!(sim.population(), 2);

    sim.paint(1, 1, 0);
    assert_eq!(sim.population(), 1);

    // Out-of-bounds painting is ignored.
    sim.paint(1000, 1000, 1);
    assert_eq!(sim.population(), 1);

    sim.set_cells(&vec![1u8; 30 * 30]);
    assert_eq!(sim.population(), 900);

    sim.edit_cells(|cells| cells.fill(0));
    assert_eq!(sim.population(), 0);

    sim.load_pattern("Glider");
    assert_eq!(sim.population(), 5);

    sim.run_steps(3, || false);
    assert_eq!(
        sim.population(),
        sim.grid().population() as u64,
        "stepping must keep the cached count exact"
    );

    sim.undo();
    assert_eq!(
        sim.population(),
        sim.grid().population() as u64,
        "undo must refresh the cached count"
    );

    sim.resize(
        40,
        40,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        true,
    );
    assert_eq!(sim.population(), sim.grid().population() as u64);
}

#[test]
fn undo_restores_automaton_state_that_lives_outside_the_grid() {
    let mut sim = ant(24, 24);
    sim.run_steps(6, || false);
    sim.push_undo();
    sim.run_steps(6, || false);
    let after_later_steps = sim.grid().cells.clone();

    assert!(sim.undo(), "undo should succeed");
    assert_eq!(sim.generation, 6);
    assert_ne!(sim.grid().cells, after_later_steps);

    // Replaying from the restored state must retrace the same history. This
    // fails if undo restores cells but leaves the ant where it turned next.
    sim.run_steps(6, || false);
    assert_eq!(
        sim.grid().cells,
        after_later_steps,
        "undo must restore the ant heading as well as the cells"
    );
}

#[test]
fn the_ant_marker_is_always_a_single_cell() {
    let mut sim = ant(16, 16);
    for _ in 0..50 {
        sim.step();
        let markers = sim.grid().cells.iter().filter(|&&state| state == 2).count();
        assert_eq!(markers, 1, "exactly one cell should carry the ant marker");
    }
}

#[test]
fn grid_requests_from_files_are_clamped() {
    let big = conway(usize::MAX, usize::MAX);
    assert_eq!(big.width(), MAX_GRID_SIDE);
    assert_eq!(big.height(), MAX_GRID_SIDE);

    let tiny = conway(0, 0);
    assert_eq!((tiny.width(), tiny.height()), (1, 1));

    let mut resized = conway(10, 10);
    resized.resize(
        usize::MAX,
        10,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        false,
    );
    assert_eq!(resized.width(), MAX_GRID_SIDE);
}

#[test]
fn scrubbing_reaches_every_buffered_generation() {
    let mut sim = conway(20, 20);
    sim.randomize(11);
    sim.run_steps(5, || false);
    assert_eq!(sim.timeline_len(), 6);

    for index in 0..6 {
        assert!(sim.scrub(index));
        assert_eq!(sim.timeline_generation(index), Some(sim.generation));
        assert_eq!(sim.population(), sim.grid().population() as u64);
    }

    // The newest entry is the live state.
    assert!(sim.scrub(5));
    assert_eq!(sim.timeline_pos(), None);
}

#[test]
fn playing_from_a_scrubbed_generation_forks_the_timeline() {
    let mut sim = conway(20, 20);
    sim.randomize(5);
    sim.run_steps(6, || false);
    assert_eq!(sim.timeline_len(), 7);

    assert!(sim.scrub(2));
    sim.step();

    assert_eq!(sim.generation, 3);
    assert_eq!(
        sim.timeline_len(),
        4,
        "history after the scrub point must be dropped"
    );
    assert_eq!(sim.timeline_pos(), None);
}

#[test]
fn render_frames_track_the_grid_and_reuse_their_buffers() {
    let mut sim = conway(12, 12);
    sim.paint(0, 0, 1);

    let frame = sim.take_render_frame();
    assert_eq!((frame.width, frame.height), (12, 12));
    assert_eq!(frame.cells[0], 1);
    assert_eq!(frame.ages.len(), 12 * 12);
    let buffer = frame.cells.as_ptr();
    sim.return_render_frame(frame);

    // Nothing changed, so the cached buffers are handed back untouched.
    let frame = sim.take_render_frame();
    assert_eq!(frame.cells.as_ptr(), buffer);
    assert_eq!(frame.cells[0], 1);
    sim.return_render_frame(frame);

    // A real change must be reflected.
    sim.paint(1, 1, 1);
    let frame = sim.take_render_frame();
    assert_eq!(frame.cells[12 + 1], 1);
    sim.return_render_frame(frame);
}
