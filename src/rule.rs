//! Rule system for 2D cellular automata
//!
//! Supports multiple rule formats:
//! - Life-like (B/S notation): e.g., B3/S23 for Conway's Game of Life
//! - Wolfram-style numbering for totalistic rules
//! - General binary rules (2^512 for Moore neighborhood)
//! - Isotropic rules (rotation/reflection invariant)

use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum number of neighbors for any supported neighborhood
pub const MAX_NEIGHBORS: usize = 24; // Extended Moore range 2

/// A cellular automaton rule
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name
    pub name: String,
    /// The rule specification
    pub spec: RuleSpec,
    /// Optional description/notes
    pub description: Option<String>,
    /// Source/attribution
    pub source: Option<String>,
    /// Discovered properties
    pub properties: RuleProperties,
}

/// Rule specification - defines how next state is computed
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuleSpec {
    /// Life-like rules using Birth/Survival notation
    /// Only depends on neighbor count, not configuration
    LifeLike(LifeLikeRule),

    /// Totalistic rules - generalized B/S for different neighborhoods
    Totalistic(TotalisticRule),

    /// Outer totalistic - like Life-like but allows more states
    OuterTotalistic(OuterTotalisticRule),

    /// General binary rule - arbitrary function of neighborhood configuration
    /// For Moore neighborhood: 2^512 possible rules
    General(GeneralRule),

    /// Isotropic rules - invariant under rotation/reflection
    /// Smaller rule space than general, larger than totalistic
    Isotropic(IsotropicRule),

    /// Generations rules - cells have multiple states (alive -> aging -> dead)
    Generations(GenerationsRule),
}

/// Life-like rule (B/S notation)
/// Standard for 2-state, Moore neighborhood, totalistic rules
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LifeLikeRule {
    /// Number of neighbors that cause birth (dead -> alive)
    pub birth: Vec<u8>,
    /// Number of neighbors that allow survival (alive -> alive)
    pub survival: Vec<u8>,
}

impl LifeLikeRule {
    pub fn new(birth: &[u8], survival: &[u8]) -> Self {
        let mut b: Vec<u8> = birth.to_vec();
        let mut s: Vec<u8> = survival.to_vec();
        b.sort_unstable();
        s.sort_unstable();
        b.dedup();
        s.dedup();
        Self {
            birth: b,
            survival: s,
        }
    }

    /// Parse from B/S notation string (e.g., "B3/S23" or "23/3")
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_uppercase();

        // Try B.../S... format
        if s.contains('B') || s.contains('S') {
            let mut birth = Vec::new();
            let mut survival = Vec::new();
            let mut current = &mut birth;

            for c in s.chars() {
                match c {
                    'B' => current = &mut birth,
                    'S' => current = &mut survival,
                    '/' | ' ' => {}
                    '0'..='8' => current.push(c.to_digit(10)? as u8),
                    _ => return None,
                }
            }

            return Some(Self::new(&birth, &survival));
        }

        // Try S/B format (older notation like "23/3")
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            let survival: Vec<u8> = parts[0]
                .chars()
                .filter_map(|c| c.to_digit(10).map(|d| d as u8))
                .collect();
            let birth: Vec<u8> = parts[1]
                .chars()
                .filter_map(|c| c.to_digit(10).map(|d| d as u8))
                .collect();
            return Some(Self::new(&birth, &survival));
        }

        None
    }

    /// Convert to canonical B/S string
    pub fn to_bs_string(&self) -> String {
        let b: String = self.birth.iter().map(|n| n.to_string()).collect();
        let s: String = self.survival.iter().map(|n| n.to_string()).collect();
        format!("B{}/S{}", b, s)
    }

    /// Convert to numeric representation for compact storage
    /// Bit i is set if i neighbors causes birth (bits 0-8)
    /// Bit i+9 is set if i neighbors allows survival (bits 9-17)
    pub fn to_bits(&self) -> u32 {
        let mut bits = 0u32;
        for &b in &self.birth {
            if b <= 8 {
                bits |= 1 << (b + 9);
            }
        }
        for &s in &self.survival {
            if s <= 8 {
                bits |= 1 << s;
            }
        }
        bits
    }

    /// Create from numeric representation
    pub fn from_bits(bits: u32) -> Self {
        let birth: Vec<u8> = (0..=8).filter(|&i| (bits >> (i + 9)) & 1 == 1).collect();
        let survival: Vec<u8> = (0..=8).filter(|&i| (bits >> i) & 1 == 1).collect();
        Self { birth, survival }
    }

    /// Calculate Li-Packard lambda parameter
    /// λ = (number of transitions to alive state) / (total transitions)
    /// For totalistic rules: λ = (|B| + |S|) / 18
    pub fn lambda(&self) -> f64 {
        (self.birth.len() + self.survival.len()) as f64 / 18.0
    }

    /// Apply rule to get next state
    pub fn apply(&self, alive: bool, neighbor_count: u8) -> bool {
        if alive {
            self.survival.contains(&neighbor_count)
        } else {
            self.birth.contains(&neighbor_count)
        }
    }
}

impl fmt::Display for LifeLikeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_bs_string())
    }
}

/// Totalistic rule for arbitrary neighborhood sizes
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TotalisticRule {
    /// Maximum possible neighbors (determines rule space size)
    pub max_neighbors: u8,
    /// Neighbor counts that cause birth
    pub birth: Vec<u8>,
    /// Neighbor counts that allow survival
    pub survival: Vec<u8>,
}

impl TotalisticRule {
    pub fn apply(&self, alive: bool, neighbor_count: u8) -> bool {
        if alive {
            self.survival.contains(&neighbor_count)
        } else {
            self.birth.contains(&neighbor_count)
        }
    }

    pub fn lambda(&self) -> f64 {
        let total = 2 * (self.max_neighbors as usize + 1);
        (self.birth.len() + self.survival.len()) as f64 / total as f64
    }
}

/// Outer totalistic rule with multiple states
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OuterTotalisticRule {
    pub num_states: u8,
    pub max_neighbors: u8,
    /// Transition table: state -> neighbor_count -> next_state
    pub transitions: Vec<Vec<u8>>,
}

/// General binary rule - complete lookup table
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneralRule {
    /// Number of cells in neighborhood (including center)
    pub neighborhood_size: u8,
    /// Lookup table: configuration (as bits) -> next state
    /// Size is 2^neighborhood_size bits = 2^(neighborhood_size-3) bytes
    pub table: Vec<u8>,
}

impl GeneralRule {
    pub fn new(neighborhood_size: u8) -> Self {
        let table_size = 1usize << (neighborhood_size.saturating_sub(3).max(0));
        Self {
            neighborhood_size,
            table: vec![0; table_size.max(1)],
        }
    }

    /// Get next state for a neighborhood configuration
    /// Config is a bitmask where bit i = state of neighbor i
    pub fn apply(&self, config: u32) -> bool {
        let byte_idx = (config / 8) as usize;
        let bit_idx = config % 8;
        if byte_idx < self.table.len() {
            (self.table[byte_idx] >> bit_idx) & 1 == 1
        } else {
            false
        }
    }

    /// Set the output for a configuration
    pub fn set(&mut self, config: u32, alive: bool) {
        let byte_idx = (config / 8) as usize;
        let bit_idx = config % 8;
        if byte_idx < self.table.len() {
            if alive {
                self.table[byte_idx] |= 1 << bit_idx;
            } else {
                self.table[byte_idx] &= !(1 << bit_idx);
            }
        }
    }
}

/// Isotropic rule - invariant under symmetry operations
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IsotropicRule {
    /// Equivalence classes mapping: canonical config -> output
    /// Much smaller than general rule due to symmetry
    pub equivalence_outputs: Vec<bool>,
    /// Mapping from configuration to equivalence class index
    #[serde(skip)]
    pub config_to_class: Vec<u16>,
}

/// Generations rule - multiple states with aging
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GenerationsRule {
    /// Number of states (including dead=0 and alive=1)
    pub num_states: u8,
    /// Birth conditions (dead + enough neighbors -> state 1)
    pub birth: Vec<u8>,
    /// Survival conditions (state 1 + enough neighbors -> stay state 1)
    pub survival: Vec<u8>,
    // States 2..num_states-1 automatically decay to next state
    // State num_states-1 decays to 0 (dead)
}

impl GenerationsRule {
    pub fn apply(&self, state: u8, neighbor_count: u8) -> u8 {
        match state {
            0 => {
                if self.birth.contains(&neighbor_count) {
                    1
                } else {
                    0
                }
            }
            1 => {
                if self.survival.contains(&neighbor_count) {
                    1
                } else if self.num_states > 2 {
                    2
                } else {
                    0
                }
            }
            s => {
                // Aging states decay
                if s + 1 >= self.num_states {
                    0
                } else {
                    s + 1
                }
            }
        }
    }
}

/// Discovered/computed properties of a rule
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuleProperties {
    /// Wolfram classification (1-4)
    pub wolfram_class: Option<u8>,
    /// Li-Packard lambda parameter
    pub lambda: Option<f64>,
    /// Whether rule supports gliders
    pub has_gliders: Option<bool>,
    /// Whether rule is symmetric (same as its dual)
    pub is_symmetric: Option<bool>,
    /// Whether rule is known to be Turing complete
    pub turing_complete: Option<bool>,
    /// Typical behavior description
    pub behavior: Option<String>,
}

impl Rule {
    pub fn life_like(name: impl Into<String>, birth: &[u8], survival: &[u8]) -> Self {
        let rule = LifeLikeRule::new(birth, survival);
        let lambda = rule.lambda();
        Self {
            name: name.into(),
            spec: RuleSpec::LifeLike(rule),
            description: None,
            source: None,
            properties: RuleProperties {
                lambda: Some(lambda),
                ..Default::default()
            },
        }
    }

    /// Get canonical identifier string for this rule
    pub fn canonical_id(&self) -> String {
        match &self.spec {
            RuleSpec::LifeLike(r) => r.to_bs_string(),
            RuleSpec::Totalistic(r) => {
                let b: String = r.birth.iter().map(|n| n.to_string()).collect();
                let s: String = r.survival.iter().map(|n| n.to_string()).collect();
                format!("T{}:B{}/S{}", r.max_neighbors, b, s)
            }
            RuleSpec::Generations(r) => {
                let b: String = r.birth.iter().map(|n| n.to_string()).collect();
                let s: String = r.survival.iter().map(|n| n.to_string()).collect();
                format!("G{}:B{}/S{}", r.num_states, b, s)
            }
            RuleSpec::General(r) => {
                // Hash the table for identification
                let hash = simple_hash(&r.table);
                format!("GEN{}:{:016x}", r.neighborhood_size, hash)
            }
            RuleSpec::Isotropic(r) => {
                let hash = simple_hash_bools(&r.equivalence_outputs);
                format!("ISO:{:016x}", hash)
            }
            RuleSpec::OuterTotalistic(r) => {
                format!("OT{}x{}", r.num_states, r.max_neighbors)
            }
        }
    }
}

fn simple_hash(data: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn simple_hash_bools(data: &[bool]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Well-known Life-like rules
pub mod catalog {
    use super::*;

    pub fn game_of_life() -> Rule {
        Rule {
            name: "Conway's Game of Life".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3], &[2, 3])),
            description: Some(
                "The classic cellular automaton discovered by John Conway in 1970. \
                               Exhibits complex behavior including gliders, oscillators, and \
                               computational universality."
                    .into(),
            ),
            source: Some("Conway, 1970".into()),
            properties: RuleProperties {
                wolfram_class: Some(4),
                lambda: Some(5.0 / 18.0),
                has_gliders: Some(true),
                is_symmetric: Some(false),
                turing_complete: Some(true),
                behavior: Some("Complex/Class IV".into()),
            },
        }
    }

    pub fn highlife() -> Rule {
        Rule {
            name: "HighLife".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 6], &[2, 3])),
            description: Some(
                "Similar to Life but with an additional birth condition at 6. \
                               Features a small replicator pattern."
                    .into(),
            ),
            source: Some("Nathan Thompson, 1994".into()),
            properties: RuleProperties {
                wolfram_class: Some(4),
                lambda: Some(6.0 / 18.0),
                has_gliders: Some(true),
                turing_complete: Some(true),
                ..Default::default()
            },
        }
    }

    pub fn day_and_night() -> Rule {
        Rule {
            name: "Day & Night".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 6, 7, 8], &[3, 4, 6, 7, 8])),
            description: Some(
                "Symmetric rule where patterns work the same with colors inverted. \
                               Named because 'day' and 'night' (inverted) behave identically."
                    .into(),
            ),
            source: Some("Nathan Thompson".into()),
            properties: RuleProperties {
                wolfram_class: Some(4),
                is_symmetric: Some(true),
                has_gliders: Some(true),
                ..Default::default()
            },
        }
    }

    pub fn seeds() -> Rule {
        Rule {
            name: "Seeds".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[2], &[])),
            description: Some(
                "Explosive rule where cells immediately die but spawn neighbors. \
                               Creates chaotic, expanding patterns."
                    .into(),
            ),
            source: Some("Brian Silverman".into()),
            properties: RuleProperties {
                wolfram_class: Some(3),
                has_gliders: Some(false),
                behavior: Some("Explosive/Class III".into()),
                ..Default::default()
            },
        }
    }

    pub fn life_without_death() -> Rule {
        Rule {
            name: "Life without Death".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3], &[0, 1, 2, 3, 4, 5, 6, 7, 8])),
            description: Some(
                "Like Life, but cells never die. Creates growing ladder-like structures.".into(),
            ),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                has_gliders: Some(true),
                behavior: Some("Expanding/Complex".into()),
                ..Default::default()
            },
        }
    }

    pub fn diamoeba() -> Rule {
        Rule {
            name: "Diamoeba".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 5, 6, 7, 8], &[5, 6, 7, 8])),
            description: Some(
                "Creates large diamond-shaped amoeba-like patterns that grow and shrink.".into(),
            ),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn two_x_two() -> Rule {
        Rule {
            name: "2x2".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 6], &[1, 2, 5])),
            description: Some("Characterized by 2x2 blocks that remain stable.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn morley() -> Rule {
        Rule {
            name: "Morley".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 6, 8], &[2, 4, 5])),
            description: Some("Also known as Move. Has natural gliders.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                has_gliders: Some(true),
                ..Default::default()
            },
        }
    }

    pub fn anneal() -> Rule {
        Rule {
            name: "Anneal".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[4, 6, 7, 8], &[3, 5, 6, 7, 8])),
            description: Some(
                "Tends to form large stable regions. Similar to majority vote.".into(),
            ),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(2),
                behavior: Some("Settling/Class II".into()),
                ..Default::default()
            },
        }
    }

    pub fn maze() -> Rule {
        Rule {
            name: "Maze".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3], &[1, 2, 3, 4, 5])),
            description: Some("Grows maze-like corridors from random initial conditions.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                behavior: Some("Maze-forming".into()),
                ..Default::default()
            },
        }
    }

    pub fn mazectric() -> Rule {
        Rule {
            name: "Mazectric".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3], &[1, 2, 3, 4])),
            description: Some("Like Maze but with longer, straighter corridors.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn coral() -> Rule {
        Rule {
            name: "Coral".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3], &[4, 5, 6, 7, 8])),
            description: Some(
                "Grows slowly like coral, creating stable branching structures.".into(),
            ),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                behavior: Some("Slow growth".into()),
                ..Default::default()
            },
        }
    }

    pub fn replicator() -> Rule {
        Rule {
            name: "Replicator".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[1, 3, 5, 7], &[1, 3, 5, 7])),
            description: Some(
                "Every pattern is a replicator. Creates complex fractal-like growth.".into(),
            ),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(3),
                behavior: Some("Self-replicating".into()),
                ..Default::default()
            },
        }
    }

    pub fn gnarl() -> Rule {
        Rule {
            name: "Gnarl".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[1], &[1])),
            description: Some("Explosive growth creating gnarled tree-like patterns.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(3),
                behavior: Some("Explosive tree growth".into()),
                ..Default::default()
            },
        }
    }

    pub fn coagulations() -> Rule {
        Rule {
            name: "Coagulations".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 7, 8], &[2, 3, 5, 6, 7, 8])),
            description: Some("Forms stable, coagulated blobs.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn assimilation() -> Rule {
        Rule {
            name: "Assimilation".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 4, 5], &[4, 5, 6, 7])),
            description: Some("Diamonds slowly assimilate nearby live cells.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn stains() -> Rule {
        Rule {
            name: "Stains".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[3, 6, 7, 8], &[2, 3, 5, 6, 7, 8])),
            description: Some("Creates ink-stain-like stable patterns.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(4),
                ..Default::default()
            },
        }
    }

    pub fn walled_cities() -> Rule {
        Rule {
            name: "Walled Cities".into(),
            spec: RuleSpec::LifeLike(LifeLikeRule::new(&[4, 5, 6, 7, 8], &[2, 3, 4, 5])),
            description: Some("Stable regions surrounded by walls.".into()),
            source: None,
            properties: RuleProperties {
                wolfram_class: Some(2),
                ..Default::default()
            },
        }
    }

    /// Get all known rules
    pub fn all() -> Vec<Rule> {
        vec![
            game_of_life(),
            highlife(),
            day_and_night(),
            seeds(),
            life_without_death(),
            diamoeba(),
            two_x_two(),
            morley(),
            anneal(),
            maze(),
            mazectric(),
            coral(),
            replicator(),
            gnarl(),
            coagulations(),
            assimilation(),
            stains(),
            walled_cities(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_parsing() {
        let rule = LifeLikeRule::parse("B3/S23").unwrap();
        assert_eq!(rule.birth, vec![3]);
        assert_eq!(rule.survival, vec![2, 3]);
        assert_eq!(rule.to_bs_string(), "B3/S23");
    }

    #[test]
    fn test_old_notation() {
        let rule = LifeLikeRule::parse("23/3").unwrap();
        assert_eq!(rule.birth, vec![3]);
        assert_eq!(rule.survival, vec![2, 3]);
    }

    #[test]
    fn test_bits_roundtrip() {
        let rule = LifeLikeRule::new(&[3, 6], &[2, 3]);
        let bits = rule.to_bits();
        let restored = LifeLikeRule::from_bits(bits);
        assert_eq!(rule.birth, restored.birth);
        assert_eq!(rule.survival, restored.survival);
    }

    #[test]
    fn test_lambda() {
        let life = LifeLikeRule::new(&[3], &[2, 3]);
        let lambda = life.lambda();
        assert!((lambda - 3.0 / 18.0).abs() < 0.001);
    }
}
