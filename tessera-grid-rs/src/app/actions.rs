use super::*;

impl TesseraGridApp {
    pub(super) fn step_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.speed.max(1) as f64)
    }

    pub(super) fn do_step(&mut self) {
        self.sim.step();
    }

    pub(super) fn randomize(&mut self) {
        let seed = self.seed_input.trim().parse::<u64>().unwrap_or_else(|_| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1)
        });
        self.push_undo();
        self.last_seed = seed;
        self.seed_input = seed.to_string();
        self.sim.randomize(seed);
        self.running = false;
        self.set_status(format!("Randomized (seed {seed})"));
    }

    pub(super) fn do_reset(&mut self) {
        self.push_undo();
        self.sim.reset();
        self.running = false;
    }

    pub(super) fn rebuild_automaton(&mut self) {
        let (w, h) = (self.sim.width(), self.sim.height());
        self.sim.rebuild(
            &self.selected_mode,
            w,
            h,
            RuleSpec::new(
                self.selected_boundary,
                &self.custom_birth,
                &self.custom_survival,
            ),
            &self.selected_pattern,
        );
        if !self
            .sim
            .available_patterns()
            .contains(&self.selected_pattern.as_str())
        {
            self.selected_pattern = self
                .sim
                .available_patterns()
                .first()
                .copied()
                .unwrap_or("")
                .to_owned();
            let pattern = self.selected_pattern.clone();
            self.sim.load_pattern(&pattern);
        }
        self.running = false;
        self.viewport_offset = Vec2::ZERO;
        self.selection = None;
        self.selection_anchor = None;
        self.paint_state = self.paint_state.min(self.sim.max_state()).max(1);
    }

    pub(super) fn push_undo(&mut self) {
        self.sim.push_undo();
    }
}
