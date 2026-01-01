use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::neighborhood::{BoundaryCondition, Neighborhood};
use crate::rule::{Rule, RuleSpec};

#[derive(Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub width: u32,
    pub height: u32,
    pub rule: Rule,
    pub neighborhood: Neighborhood,
    pub boundary: BoundaryCondition,
    pub seed: u64,
    pub initial_density: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            width: 128,
            height: 128,
            rule: crate::rule::catalog::game_of_life(),
            neighborhood: Neighborhood::moore(),
            boundary: BoundaryCondition::Toroidal,
            seed: 0,
            initial_density: 0.3,
        }
    }
}

impl SimulationConfig {
    pub fn to_shareable_string(&self) -> String {
        use base64::Engine;
        let json = serde_json::to_string(self).unwrap_or_default();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json)
    }

    pub fn from_shareable_string(s: &str) -> Option<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .ok()?;
        let json = String::from_utf8(bytes).ok()?;
        serde_json::from_str(&json).ok()
    }
}

pub struct Simulation {
    pub config: SimulationConfig,
    cells: Vec<u8>,
    generation: u64,
    history: VecDeque<Vec<u8>>,
    max_history: usize,
    rng: StdRng,
    stats: SimulationStats,
}

#[derive(Clone, Default)]
pub struct SimulationStats {
    pub population: u64,
    pub density: f64,
    pub entropy: f64,
    pub population_history: VecDeque<u64>,
    pub entropy_history: VecDeque<f64>,
    max_history: usize,
}

impl SimulationStats {
    fn new(max_history: usize) -> Self {
        Self {
            max_history,
            ..Default::default()
        }
    }

    fn record(&mut self, population: u64, total_cells: u64, state_counts: &[u64]) {
        self.population = population;
        self.density = population as f64 / total_cells as f64;
        self.entropy = Self::calculate_entropy(state_counts, total_cells);

        self.population_history.push_back(population);
        self.entropy_history.push_back(self.entropy);

        while self.population_history.len() > self.max_history {
            self.population_history.pop_front();
        }
        while self.entropy_history.len() > self.max_history {
            self.entropy_history.pop_front();
        }
    }

    fn calculate_entropy(state_counts: &[u64], total: u64) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let mut entropy = 0.0;
        for &count in state_counts {
            if count > 0 {
                let p = count as f64 / total as f64;
                entropy -= p * p.log2();
            }
        }
        entropy
    }
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        let size = (config.width * config.height) as usize;
        let mut rng = StdRng::seed_from_u64(config.seed);

        let cells: Vec<u8> = (0..size)
            .map(|_| {
                if rng.gen::<f32>() < config.initial_density {
                    1
                } else {
                    0
                }
            })
            .collect();

        let mut sim = Self {
            config,
            cells,
            generation: 0,
            history: VecDeque::new(),
            max_history: 1000,
            rng,
            stats: SimulationStats::new(1000),
        };
        sim.update_stats();
        sim
    }

    pub fn from_cells(config: SimulationConfig, cells: Vec<u8>) -> Self {
        let mut sim = Self {
            config,
            cells,
            generation: 0,
            history: VecDeque::new(),
            max_history: 1000,
            rng: StdRng::seed_from_u64(0),
            stats: SimulationStats::new(1000),
        };
        sim.update_stats();
        sim
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn stats(&self) -> &SimulationStats {
        &self.stats
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }

    pub fn step(&mut self) {
        self.history.push_back(self.cells.clone());
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }

        let mut next = vec![0u8; self.cells.len()];
        let w = self.config.width as i32;
        let h = self.config.height as i32;

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let current = self.cells[idx];
                let neighbor_count = self.count_neighbors(x, y);

                next[idx] = self.apply_rule(current, neighbor_count);
            }
        }

        self.cells = next;
        self.generation += 1;
        self.update_stats();
    }

    pub fn step_back(&mut self) -> bool {
        if let Some(prev) = self.history.pop_back() {
            self.cells = prev;
            self.generation = self.generation.saturating_sub(1);
            self.update_stats();
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.rng = StdRng::seed_from_u64(self.config.seed);
        let size = (self.config.width * self.config.height) as usize;
        self.cells = (0..size)
            .map(|_| {
                if self.rng.gen::<f32>() < self.config.initial_density {
                    1
                } else {
                    0
                }
            })
            .collect();
        self.generation = 0;
        self.history.clear();
        self.stats = SimulationStats::new(self.max_history);
        self.update_stats();
    }

    pub fn clear(&mut self) {
        self.cells.fill(0);
        self.generation = 0;
        self.history.clear();
        self.update_stats();
    }

    pub fn randomize(&mut self, seed: Option<u64>) {
        let seed = seed.unwrap_or_else(|| rand::thread_rng().gen());
        self.config.seed = seed;
        self.reset();
    }

    pub fn set_cell(&mut self, x: u32, y: u32, state: u8) {
        if x < self.config.width && y < self.config.height {
            let idx = (y * self.config.width + x) as usize;
            self.cells[idx] = state;
        }
    }

    pub fn get_cell(&self, x: u32, y: u32) -> u8 {
        if x < self.config.width && y < self.config.height {
            self.cells[(y * self.config.width + x) as usize]
        } else {
            0
        }
    }

    pub fn set_rule(&mut self, rule: Rule) {
        self.config.rule = rule;
    }

    pub fn set_neighborhood(&mut self, neighborhood: Neighborhood) {
        self.config.neighborhood = neighborhood;
    }

    fn count_neighbors(&self, x: i32, y: i32) -> u8 {
        let w = self.config.width as i32;
        let h = self.config.height as i32;
        let mut count = 0u8;

        for &(dx, dy) in &self.config.neighborhood.offsets {
            let nx = x + dx;
            let ny = y + dy;

            let (nx, ny) = match self.config.boundary {
                BoundaryCondition::Toroidal => (((nx % w) + w) % w, ((ny % h) + h) % h),
                BoundaryCondition::Fixed(val) => {
                    if nx < 0 || nx >= w || ny < 0 || ny >= h {
                        count += val;
                        continue;
                    }
                    (nx, ny)
                }
                BoundaryCondition::Reflective => {
                    let nx = if nx < 0 {
                        -nx
                    } else if nx >= w {
                        2 * w - nx - 2
                    } else {
                        nx
                    };
                    let ny = if ny < 0 {
                        -ny
                    } else if ny >= h {
                        2 * h - ny - 2
                    } else {
                        ny
                    };
                    (nx.clamp(0, w - 1), ny.clamp(0, h - 1))
                }
            };

            count += self.cells[(ny * w + nx) as usize];
        }

        count
    }

    fn apply_rule(&self, current: u8, neighbor_count: u8) -> u8 {
        match &self.config.rule.spec {
            RuleSpec::LifeLike(rule) => {
                if rule.apply(current != 0, neighbor_count) {
                    1
                } else {
                    0
                }
            }
            RuleSpec::Totalistic(rule) => {
                if rule.apply(current != 0, neighbor_count) {
                    1
                } else {
                    0
                }
            }
            RuleSpec::Generations(rule) => rule.apply(current, neighbor_count),
            _ => current,
        }
    }

    fn update_stats(&mut self) {
        let total = self.cells.len() as u64;
        let mut state_counts = [0u64; 256];
        let mut population = 0u64;

        for &cell in &self.cells {
            state_counts[cell as usize] += 1;
            if cell > 0 {
                population += 1;
            }
        }

        self.stats.record(population, total, &state_counts);
    }

    pub fn place_pattern(&mut self, pattern: &Pattern, x: u32, y: u32) {
        for &(px, py, state) in &pattern.cells {
            let nx = x.wrapping_add(px as u32);
            let ny = y.wrapping_add(py as u32);
            self.set_cell(nx, ny, state);
        }
    }

    pub fn export_state(&self) -> SimulationState {
        SimulationState {
            config: self.config.clone(),
            cells: self.cells.clone(),
            generation: self.generation,
        }
    }

    pub fn import_state(&mut self, state: SimulationState) {
        self.config = state.config;
        self.cells = state.cells;
        self.generation = state.generation;
        self.history.clear();
        self.update_stats();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub config: SimulationConfig,
    pub cells: Vec<u8>,
    pub generation: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    pub cells: Vec<(i32, i32, u8)>,
    pub width: u32,
    pub height: u32,
    pub rule: Option<String>,
    pub description: Option<String>,
}

impl Pattern {
    pub fn new(name: impl Into<String>, cells: Vec<(i32, i32, u8)>) -> Self {
        let width = cells.iter().map(|(x, _, _)| *x).max().unwrap_or(0) as u32 + 1;
        let height = cells.iter().map(|(_, y, _)| *y).max().unwrap_or(0) as u32 + 1;
        Self {
            name: name.into(),
            cells,
            width,
            height,
            rule: None,
            description: None,
        }
    }

    pub fn from_rle(rle: &str) -> Option<Self> {
        let mut name = String::new();
        let mut rule = None;
        let mut cells = Vec::new();
        let mut x = 0i32;
        let mut y = 0i32;
        let mut run_count = 0u32;

        for line in rle.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                if line.starts_with("#N ") {
                    name = line[3..].to_string();
                }
                continue;
            }
            if line.starts_with('x') {
                for part in line.split(',') {
                    let part = part.trim();
                    if part.starts_with("rule") {
                        if let Some(r) = part.split('=').nth(1) {
                            rule = Some(r.trim().to_string());
                        }
                    }
                }
                continue;
            }

            for ch in line.chars() {
                match ch {
                    '0'..='9' => {
                        run_count = run_count * 10 + ch.to_digit(10).unwrap();
                    }
                    'b' => {
                        let count = if run_count == 0 { 1 } else { run_count };
                        x += count as i32;
                        run_count = 0;
                    }
                    'o' => {
                        let count = if run_count == 0 { 1 } else { run_count };
                        for _ in 0..count {
                            cells.push((x, y, 1));
                            x += 1;
                        }
                        run_count = 0;
                    }
                    '$' => {
                        let count = if run_count == 0 { 1 } else { run_count };
                        y += count as i32;
                        x = 0;
                        run_count = 0;
                    }
                    '!' => break,
                    _ => {}
                }
            }
        }

        if cells.is_empty() {
            return None;
        }

        let mut pattern = Pattern::new(if name.is_empty() { "Imported" } else { &name }, cells);
        pattern.rule = rule;
        Some(pattern)
    }

    pub fn to_rle(&self) -> String {
        let mut result = String::new();

        if !self.name.is_empty() {
            result.push_str(&format!("#N {}\n", self.name));
        }

        result.push_str(&format!("x = {}, y = {}", self.width, self.height));
        if let Some(ref rule) = self.rule {
            result.push_str(&format!(", rule = {}", rule));
        }
        result.push('\n');

        let mut grid = vec![vec![0u8; self.width as usize]; self.height as usize];
        for &(x, y, state) in &self.cells {
            if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
                grid[y as usize][x as usize] = state;
            }
        }

        for (y, row) in grid.iter().enumerate() {
            let mut run_start = 0;
            while run_start < row.len() {
                let state = row[run_start];
                let mut run_end = run_start + 1;
                while run_end < row.len() && row[run_end] == state {
                    run_end += 1;
                }
                let run_len = run_end - run_start;

                let ch = if state == 0 { 'b' } else { 'o' };
                if run_len > 1 {
                    result.push_str(&format!("{}{}", run_len, ch));
                } else {
                    result.push(ch);
                }

                run_start = run_end;
            }
            if y < grid.len() - 1 {
                result.push('$');
            }
        }
        result.push('!');
        result
    }
}

pub mod patterns {
    use super::Pattern;

    pub fn glider() -> Pattern {
        Pattern::new(
            "Glider",
            vec![(1, 0, 1), (2, 1, 1), (0, 2, 1), (1, 2, 1), (2, 2, 1)],
        )
    }

    pub fn blinker() -> Pattern {
        Pattern::new("Blinker", vec![(0, 0, 1), (1, 0, 1), (2, 0, 1)])
    }

    pub fn block() -> Pattern {
        Pattern::new("Block", vec![(0, 0, 1), (1, 0, 1), (0, 1, 1), (1, 1, 1)])
    }

    pub fn beehive() -> Pattern {
        Pattern::new(
            "Beehive",
            vec![
                (1, 0, 1),
                (2, 0, 1),
                (0, 1, 1),
                (3, 1, 1),
                (1, 2, 1),
                (2, 2, 1),
            ],
        )
    }

    pub fn toad() -> Pattern {
        Pattern::new(
            "Toad",
            vec![
                (1, 0, 1),
                (2, 0, 1),
                (3, 0, 1),
                (0, 1, 1),
                (1, 1, 1),
                (2, 1, 1),
            ],
        )
    }

    pub fn beacon() -> Pattern {
        Pattern::new(
            "Beacon",
            vec![
                (0, 0, 1),
                (1, 0, 1),
                (0, 1, 1),
                (3, 2, 1),
                (2, 3, 1),
                (3, 3, 1),
            ],
        )
    }

    pub fn pulsar() -> Pattern {
        let mut cells = Vec::new();
        let pattern = [
            (2, 0),
            (3, 0),
            (4, 0),
            (8, 0),
            (9, 0),
            (10, 0),
            (0, 2),
            (5, 2),
            (7, 2),
            (12, 2),
            (0, 3),
            (5, 3),
            (7, 3),
            (12, 3),
            (0, 4),
            (5, 4),
            (7, 4),
            (12, 4),
            (2, 5),
            (3, 5),
            (4, 5),
            (8, 5),
            (9, 5),
            (10, 5),
        ];
        for &(x, y) in &pattern {
            cells.push((x, y, 1));
            cells.push((x, 12 - y, 1));
        }
        Pattern::new("Pulsar", cells)
    }

    pub fn lwss() -> Pattern {
        Pattern::new(
            "LWSS",
            vec![
                (1, 0, 1),
                (4, 0, 1),
                (0, 1, 1),
                (0, 2, 1),
                (4, 2, 1),
                (0, 3, 1),
                (1, 3, 1),
                (2, 3, 1),
                (3, 3, 1),
            ],
        )
    }

    pub fn r_pentomino() -> Pattern {
        Pattern::new(
            "R-pentomino",
            vec![(1, 0, 1), (2, 0, 1), (0, 1, 1), (1, 1, 1), (1, 2, 1)],
        )
    }

    pub fn acorn() -> Pattern {
        Pattern::new(
            "Acorn",
            vec![
                (1, 0, 1),
                (3, 1, 1),
                (0, 2, 1),
                (1, 2, 1),
                (4, 2, 1),
                (5, 2, 1),
                (6, 2, 1),
            ],
        )
    }

    pub fn gosper_glider_gun() -> Pattern {
        Pattern::new(
            "Gosper Glider Gun",
            vec![
                (24, 0, 1),
                (22, 1, 1),
                (24, 1, 1),
                (12, 2, 1),
                (13, 2, 1),
                (20, 2, 1),
                (21, 2, 1),
                (34, 2, 1),
                (35, 2, 1),
                (11, 3, 1),
                (15, 3, 1),
                (20, 3, 1),
                (21, 3, 1),
                (34, 3, 1),
                (35, 3, 1),
                (0, 4, 1),
                (1, 4, 1),
                (10, 4, 1),
                (16, 4, 1),
                (20, 4, 1),
                (21, 4, 1),
                (0, 5, 1),
                (1, 5, 1),
                (10, 5, 1),
                (14, 5, 1),
                (16, 5, 1),
                (17, 5, 1),
                (22, 5, 1),
                (24, 5, 1),
                (10, 6, 1),
                (16, 6, 1),
                (24, 6, 1),
                (11, 7, 1),
                (15, 7, 1),
                (12, 8, 1),
                (13, 8, 1),
            ],
        )
    }

    pub fn all() -> Vec<Pattern> {
        vec![
            glider(),
            blinker(),
            block(),
            beehive(),
            toad(),
            beacon(),
            pulsar(),
            lwss(),
            r_pentomino(),
            acorn(),
            gosper_glider_gun(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reproducibility() {
        let config = SimulationConfig {
            seed: 12345,
            ..Default::default()
        };

        let sim1 = Simulation::new(config.clone());
        let sim2 = Simulation::new(config);

        assert_eq!(sim1.cells(), sim2.cells());
    }

    #[test]
    fn test_step_back() {
        let config = SimulationConfig::default();
        let mut sim = Simulation::new(config);
        let initial = sim.cells().to_vec();

        sim.step();
        assert!(sim.step_back());
        assert_eq!(sim.cells(), &initial[..]);
    }

    #[test]
    fn test_rle_roundtrip() {
        let pattern = patterns::glider();
        let rle = pattern.to_rle();
        let parsed = Pattern::from_rle(&rle).unwrap();
        assert_eq!(pattern.cells.len(), parsed.cells.len());
    }
}
