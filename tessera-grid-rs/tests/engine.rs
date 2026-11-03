//! In-process engine checks.
//!
//! These use the library directly (rather than spawning the `tessera` binary)
//! so they can assert on the derived state that the UI relies on.

use tessera_grid::core::BoundaryMode;
use tessera_grid::sim::{DamageExperiment, RuleSpec, Simulation, MAX_GRID_SIDE};

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
        // The marker is a render overlay: the canonical grid stays binary.
        let display = sim.display_cells();
        let markers = display.iter().filter(|&&state| state == 2).count();
        assert_eq!(markers, 1, "exactly one cell should carry the ant marker");
        assert!(
            sim.grid().cells.iter().all(|&state| state <= 1),
            "the simulation grid must never store the marker"
        );
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

#[test]
fn population_never_counts_the_ant_marker() {
    let mut sim = ant(16, 16);
    assert_eq!(sim.population(), 0, "an empty grid has no population");
    sim.step();
    assert_eq!(sim.population(), 1, "one flipped cell, not the marker too");
    assert_eq!(sim.population(), sim.grid().population() as u64);
}

#[test]
fn out_of_range_cells_are_clamped_before_stepping() {
    let mut sim = ant(8, 8);
    // Simulates a hand-written state file or a paste from a richer mode.
    sim.set_cells(&[2u8; 64]);
    sim.step();
    assert!(
        sim.grid().cells.iter().all(|&state| state <= 1),
        "clamping must keep the grid binary"
    );

    let mut wire = Simulation::new(
        "Wireworld",
        6,
        6,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        "Empty",
    );
    wire.set_cells(&[9u8; 36]);
    wire.step();
    assert!(wire.grid().cells.iter().all(|&state| state <= 3));
}

#[test]
fn saved_ant_state_round_trips_with_position_and_heading() {
    let mut sim = ant(24, 24);
    sim.run_steps(9, || false);
    let cells = sim.state_cells();
    let aux = sim.aux_state();
    assert_eq!(aux.len(), 17, "position and heading are captured");
    let generation = sim.generation;

    sim.run_steps(9, || false);
    let later = sim.state_cells();

    sim.restore_state(&cells, &aux, generation)
        .expect("valid saved state");
    assert_eq!(sim.generation, 9);
    assert_eq!(sim.state_cells(), cells, "cells restore exactly");
    assert_eq!(
        sim.population(),
        cells.iter().filter(|&&c| c > 0).count() as u64
    );

    // Replaying from the restored state must retrace the same history; this
    // fails if the ant's position and heading were not restored.
    sim.run_steps(9, || false);
    assert_eq!(sim.state_cells(), later);
}

#[test]
fn timeline_recording_is_optional() {
    let mut sim = conway(20, 20);
    sim.record_timeline = false;
    sim.randomize(4);
    sim.run_steps(5, || false);
    assert_eq!(sim.timeline_len(), 0, "nothing is recorded");

    sim.record_timeline = true;
    sim.step();
    assert_eq!(
        sim.timeline_len(),
        2,
        "enabling recording seeds the pre-step state"
    );
}

#[test]
fn render_frames_carry_the_marker_but_not_the_grid() {
    let mut sim = ant(12, 12);
    sim.step();
    let frame = sim.take_render_frame();
    let markers = frame.cells.iter().filter(|&&state| state == 2).count();
    assert_eq!(markers, 1, "the canvas still draws the ant");
    assert!(sim.grid().cells.iter().all(|&state| state <= 1));
    sim.return_render_frame(frame);

    // An unchanged grid reuses the buffer contents instead of copying again.
    let frame = sim.take_render_frame();
    assert_eq!(frame.cells.iter().filter(|&&state| state == 2).count(), 1);
    sim.return_render_frame(frame);
}

#[test]
fn elementary_rules_fill_a_space_time_diagram() {
    let mut sim = Simulation::new(
        "Elementary 1D",
        64,
        32,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]).with_elementary(110),
        "Single Cell",
    );
    assert_eq!(sim.rule_string(), "W110");
    assert_eq!(sim.automaton_name(), "Elementary 1D");

    sim.run_steps(31, || false);
    let frontier = &sim.grid().cells[31 * 64..32 * 64];
    let live = frontier.iter().filter(|&&cell| cell > 0).count() as u64;
    assert!(live > 0, "rule 110 keeps producing cells");
    assert_eq!(sim.population(), live, "only the newest row is population");
    assert!((sim.density() - live as f32 / 64.0).abs() < f32::EPSILON);

    // Once the diagram is full it keeps scrolling.
    let before = sim.grid().cells.clone();
    sim.step();
    assert_eq!(&sim.grid().cells[..31 * 64], &before[64..]);
    assert!(sim.grid().cells.iter().all(|&cell| cell <= 1));
}

#[test]
fn the_elementary_rule_can_change_without_clearing_the_diagram() {
    let mut sim = Simulation::new(
        "Elementary 1D",
        32,
        8,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]).with_elementary(110),
        "Single Cell",
    );
    sim.run_steps(3, || false);
    let cells = sim.grid().cells.clone();
    assert!(sim.set_rules(&[30], &[]), "elementary rules are settable");
    assert_eq!(sim.rule_string(), "W30");
    assert_eq!(
        sim.grid().cells,
        cells,
        "changing the rule keeps the diagram"
    );

    sim.step();
    assert_ne!(sim.grid().cells, cells, "the new rule takes effect");
}

#[test]
fn metrics_report_entropy_bounds_and_cycles() {
    let mut sim = conway(20, 20);
    sim.load_pattern("Blinker");
    sim.detect_cycles = true;
    assert_eq!(sim.cycle(), None, "nothing observed yet");

    sim.run_steps(6, || false);
    let cycle = sim.cycle().expect("a blinker cycles");
    assert_eq!(cycle.period, 2, "blinker period");

    let metrics = sim.metrics();
    assert_eq!(metrics.generation, 6);
    assert_eq!(metrics.population, sim.population());
    assert!(
        metrics.entropy > 0.0 && metrics.entropy < 1.0,
        "entropy = {}",
        metrics.entropy
    );
    let (x0, y0, x1, y1) = metrics.bbox.expect("blinker has bounds");
    assert_eq!(
        (x1 - x0 + 1, y1 - y0 + 1),
        (3, 1),
        "a blinker is three by one"
    );
}

#[test]
fn cycle_detection_stays_off_until_enabled() {
    let mut sim = conway(20, 20);
    sim.load_pattern("Blinker");
    sim.run_steps(6, || false);
    assert_eq!(sim.cycle(), None);
}

#[test]
fn editing_the_grid_invalidates_the_detected_cycle() {
    let mut sim = conway(20, 20);
    sim.load_pattern("Blinker");
    sim.detect_cycles = true;
    sim.run_steps(6, || false);
    assert!(sim.cycle().is_some(), "a blinker settles into a cycle");

    sim.paint(0, 0, 1);
    assert_eq!(sim.cycle(), None, "painting starts a new observation");
    assert_eq!(sim.metrics().cycle, None);
}

#[test]
fn undoing_invalidates_the_detected_cycle() {
    let mut sim = conway(20, 20);
    sim.load_pattern("Blinker");
    sim.detect_cycles = true;
    sim.push_undo();
    sim.run_steps(8, || false);
    assert!(sim.cycle().is_some());

    assert!(sim.undo());
    assert_eq!(sim.cycle(), None, "a jump in history resets detection");
}

fn elementary(rule: u8, height: usize, pattern: &str) -> Simulation {
    Simulation::new(
        "Elementary 1D",
        8,
        height,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]).with_elementary(rule),
        pattern,
    )
}

fn assert_active_population(sim: &Simulation) {
    let expected = sim.active_cells().iter().filter(|&&cell| cell != 0).count() as u64;
    assert_eq!(sim.population(), expected);
    assert!(sim.density() <= 1.0);
    assert_eq!(sim.metrics().population, expected);
}

#[test]
fn elementary_history_edits_do_not_affect_active_population_or_underflow() {
    let mut sim = elementary(0, 8, "Single Cell");
    sim.step();
    assert_eq!(sim.population(), 0);
    sim.paint(0, 4, 0);
    assert_active_population(&sim);
    sim.paint(0, 4, 1);
    assert_active_population(&sim);
    sim.edit_cells(|cells| cells[1] = 1);
    assert_active_population(&sim);
}

#[test]
fn elementary_restore_undo_scrub_and_resize_preserve_the_active_frontier() {
    let mut sim = elementary(204, 8, "Single Cell");
    sim.run_steps(3, || false);
    let cells = sim.state_cells();
    let aux = sim.aux_state();
    sim.restore_state(&cells, &aux, 3).unwrap();
    assert_eq!(sim.population(), 1);
    assert_active_population(&sim);
    sim.push_undo();
    sim.paint(3, 4, 0);
    sim.undo();
    assert_active_population(&sim);
    sim.step();
    assert!(sim.scrub(0));
    assert_active_population(&sim);
    sim.resize(
        8,
        2,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]).with_elementary(204),
        true,
    );
    assert_eq!(sim.population(), 1, "cropping retains the most recent row");
    sim.step();
    assert_eq!(sim.population(), 1);
    assert_active_population(&sim);
}

#[test]
fn elementary_metrics_and_cycles_do_not_depend_on_diagram_height() {
    for height in [1, 2, 8] {
        let mut sim = elementary(1, height, "Empty");
        sim.detect_cycles = true;
        sim.run_steps(3, || false);
        assert_eq!(sim.population(), 8);
        assert_eq!(sim.metrics().entropy, 0.0, "uniform active row");
        assert_eq!(sim.metrics().bbox, Some((0, 0, 7, 0)));
        let cycle = sim.cycle().unwrap();
        assert_eq!(cycle.period, 2);
        assert_eq!(cycle.transient, 0);
    }
}

#[test]
fn cycles_include_initial_generation_and_all_ant_state() {
    let mut blinker = conway(20, 20);
    blinker.load_pattern("Blinker");
    blinker.detect_cycles = true;
    blinker.run_steps(2, || false);
    assert_eq!(blinker.cycle().unwrap().transient, 0);
    assert_eq!(blinker.cycle().unwrap().period, 2);
    let mut sim = ant(1, 2);
    sim.detect_cycles = true;
    sim.run_steps(5, || false);
    assert_eq!(sim.cycle().unwrap().period, 4);
    assert_eq!(sim.cycle().unwrap().transient, 0);
}

#[test]
fn changing_rules_or_boundaries_invalidates_cycles_and_the_metrics_revision() {
    let mut sim = Simulation::new(
        "Custom Rules",
        8,
        8,
        RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
        "Empty",
    );
    sim.detect_cycles = true;
    sim.step();
    assert_eq!(sim.cycle().unwrap().period, 1);
    let revision = sim.revision();
    assert!(sim.set_rules(&[0], &[]));
    assert_eq!(sim.cycle(), None);
    assert!(sim.revision() > revision);
    sim.step();
    assert_eq!(sim.population(), 64);
    sim.step();
    assert_eq!(sim.cycle().unwrap().period, 2);
    let revision = sim.revision();
    sim.set_boundary(BoundaryMode::Fixed);
    assert!(sim.revision() > revision);
    assert_eq!(sim.cycle(), None);
}

#[test]
fn invalid_restore_is_rejected_without_partial_mutation() {
    let mut sim = ant(8, 8);
    sim.run_steps(4, || false);
    let (cells, aux, generation) = (sim.state_cells(), sim.aux_state(), sim.generation);
    assert!(sim.restore_state(&cells[..2], &aux, 0).is_err());
    assert!(sim.restore_state(&cells, &[0], 0).is_err());
    assert!(sim.restore_state(&[2; 64], &aux, 0).is_err());
    assert_eq!(sim.state_cells(), cells);
    assert_eq!(sim.aux_state(), aux);
    assert_eq!(sim.generation, generation);
}

#[test]
fn damage_experiments_use_exact_state_and_not_display_history() {
    for height in [1, 8] {
        let mut sim = elementary(204, height, "Single Cell");
        sim.run_steps(3, || false);
        let (cells, aux, generation) = (sim.state_cells(), sim.aux_state(), sim.generation);
        let mut experiment = DamageExperiment::new(&sim).unwrap();
        assert_eq!(experiment.control().state_cells(), cells);
        assert_eq!(experiment.control().aux_state(), aux);
        assert_eq!(experiment.control().generation, generation);
        assert_eq!(experiment.sample().distance, 1);
        assert_eq!(experiment.sample().fraction, 1.0 / 8.0);
        for _ in 0..4 {
            assert_eq!(
                experiment.step().distance,
                1,
                "history is not accumulated damage"
            );
        }

        assert_eq!(sim.state_cells(), cells, "source is unchanged");
        assert_eq!(experiment.log_separation_rate(), Some(0.0));
    }
}

#[test]
fn forks_preserve_future_evolution_for_every_mode() {
    for mode in tessera_grid::automata::ALL_MODES {
        let mut original = Simulation::new(
            mode,
            11,
            9,
            RuleSpec::new(BoundaryMode::Wrap, &[0, 3], &[1, 2, 3]).with_elementary(30),
            "Empty",
        );
        original.randomize(42);
        original.run_steps(3, || false);
        let mut fork = original.fork().unwrap();
        assert_eq!(
            fork.evolution_settings(),
            original.evolution_settings(),
            "{mode}"
        );
        for _ in 0..10 {
            original.step();
            fork.step();
            assert_eq!(fork.state_cells(), original.state_cells(), "{mode}");
            assert_eq!(fork.aux_state(), original.aux_state(), "{mode}");
            assert_eq!(fork.population(), original.population(), "{mode}");
        }
    }
}
