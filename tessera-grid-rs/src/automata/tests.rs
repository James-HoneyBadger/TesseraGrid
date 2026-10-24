use super::*;
use crate::core::BoundaryMode;

fn paint(auto: &mut dyn Automaton, cells: &[(usize, usize, u8)]) {
    for &(x, y, state) in cells {
        auto.paint(y, x, state);
    }
}

#[test]
fn conway_blinker_has_period_two() {
    let mut life = ConwayLife::new(5, 5);
    paint(&mut life, &[(1, 2, 1), (2, 2, 1), (3, 2, 1)]);
    let horizontal = life.get_grid().cells.clone();

    life.step();
    let vertical = life.get_grid().cells.clone();
    assert_ne!(horizontal, vertical);

    life.step();
    assert_eq!(life.get_grid().cells, horizontal);
}

#[test]
fn conway_block_is_still() {
    let mut life = ConwayLife::new(5, 5);
    paint(&mut life, &[(1, 1, 1), (2, 1, 1), (1, 2, 1), (2, 2, 1)]);
    let before = life.get_grid().cells.clone();
    life.step();
    assert_eq!(life.get_grid().cells, before);
}

#[test]
fn conway_birth_needs_three_neighbours() {
    let mut life = ConwayLife::new(5, 5);
    paint(&mut life, &[(1, 1, 1), (2, 1, 1), (1, 2, 1)]);
    life.step();
    assert_eq!(life.get_grid().get(2, 2), 1);
}

#[test]
fn highlife_births_on_six_neighbours() {
    let mut life = HighLife::new(5, 5);
    paint(
        &mut life,
        &[
            (1, 1, 1),
            (2, 1, 1),
            (3, 1, 1),
            (1, 2, 1),
            (1, 3, 1),
            (2, 3, 1),
        ],
    );
    life.step();
    assert_eq!(life.get_grid().get(2, 2), 1, "B6 should birth the centre");
}

#[test]
fn wireworld_head_advances_through_conductor() {
    let mut wire = Wireworld::new(5, 5);
    paint(&mut wire, &[(2, 2, 1), (3, 2, 3)]);
    wire.step();
    assert_eq!(wire.get_grid().get(2, 2), 2, "head becomes tail");
    assert_eq!(wire.get_grid().get(2, 3), 1, "conductor becomes head");
}

#[test]
fn brians_brain_cycles_states() {
    let mut brain = BriansBrain::new(5, 5);
    paint(&mut brain, &[(1, 2, 1), (3, 2, 1)]);
    brain.step();
    assert_eq!(brain.get_grid().get(2, 2), 1, "off + 2 firing => firing");
    assert_eq!(brain.get_grid().get(2, 1), 2, "firing => refractory");
}

#[test]
fn generations_decays_unsupported_cells() {
    let mut gen = Generations::new(5, 5);
    gen.paint(2, 2, 1);
    gen.step();
    assert_eq!(gen.get_grid().get(2, 2), 2, "lonely cell starts decaying");
}

#[test]
fn langtons_ant_moves_and_flips() {
    let mut ant = LangtonsAnt::new(9, 9);
    ant.step();
    // Starting cell flips to alive and the ant turns right before moving east.
    assert_eq!(ant.get_grid().get(4, 4), 1);
    assert_eq!((ant.ant_x, ant.ant_y), (5, 4));
}

#[test]
fn boundary_modes_resolve_edges() {
    assert_eq!(BoundaryMode::Wrap.resolve(-1, -1, 10, 10), Some((9, 9)));
    assert_eq!(BoundaryMode::Wrap.resolve(10, 10, 10, 10), Some((0, 0)));
    assert_eq!(BoundaryMode::Fixed.resolve(-1, 0, 10, 10), None);
    assert_eq!(BoundaryMode::Fixed.resolve(10, 0, 10, 10), None);
    assert_eq!(BoundaryMode::Reflect.resolve(-1, -1, 10, 10), Some((0, 0)));
    assert_eq!(
        BoundaryMode::Reflect.resolve(10, 10, 10, 10),
        Some((9, 9)),
        "reflection clamps just outside the edge"
    );
}

#[test]
fn all_conway_patterns_load() {
    let mut life = ConwayLife::new(80, 80);
    for name in crate::patterns::CONWAY_PATTERN_NAMES {
        life.load_pattern(name);
        let population = life.get_grid().population();
        match *name {
            "Empty" => assert_eq!(population, 0, "{name} should be empty"),
            "Random Soup" => assert!(population > 0, "{name} should fill cells"),
            _ => assert!(population > 0, "{name} should contain live cells"),
        }
    }
}

#[test]
fn factory_covers_every_mode() {
    for mode in ALL_MODES {
        let auto = make_automaton(mode, 20, 20);
        assert_eq!(auto.get_grid().width, 20, "{mode} width");
        assert_eq!(auto.get_grid().height, 20, "{mode} height");
        assert!(auto.max_state() >= 1, "{mode} paintable states");
        assert!(!auto.rule_string().is_empty(), "{mode} rule string");
    }
}

#[test]
fn every_mode_supports_empty_and_random_soup() {
    for mode in ALL_MODES {
        let mut auto = make_automaton(mode, 30, 30);
        assert!(
            auto.available_patterns().contains(&"Empty"),
            "{mode} should offer Empty"
        );
        auto.load_pattern("Empty");
        // Langton's Ant renders the ant itself as a marker cell.
        let expected = if *mode == "Langton's Ant" { 1 } else { 0 };
        assert_eq!(
            auto.get_grid().population(),
            expected,
            "{mode} Empty clears"
        );

        auto.randomize(42);
        let first = auto.get_grid().cells.clone();
        auto.randomize(42);
        assert_eq!(
            auto.get_grid().cells,
            first,
            "{mode} randomize must be deterministic"
        );
    }
}
