use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Neighborhood {
    pub name: String,
    pub offsets: Vec<(i32, i32)>,
    pub range: u8,
}

impl Neighborhood {
    pub fn moore() -> Self {
        Self {
            name: "Moore".into(),
            offsets: vec![
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ],
            range: 1,
        }
    }

    pub fn von_neumann() -> Self {
        Self {
            name: "von Neumann".into(),
            offsets: vec![(0, -1), (-1, 0), (1, 0), (0, 1)],
            range: 1,
        }
    }

    pub fn moore_extended(range: u8) -> Self {
        let r = range as i32;
        let mut offsets = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx != 0 || dy != 0 {
                    offsets.push((dx, dy));
                }
            }
        }
        Self {
            name: format!("Moore (r={})", range),
            offsets,
            range,
        }
    }

    pub fn von_neumann_extended(range: u8) -> Self {
        let r = range as i32;
        let mut offsets = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let dist = dx.abs() + dy.abs();
                if dist > 0 && dist <= r {
                    offsets.push((dx, dy));
                }
            }
        }
        Self {
            name: format!("von Neumann (r={})", range),
            offsets,
            range,
        }
    }

    pub fn hexagonal() -> Self {
        Self {
            name: "Hexagonal".into(),
            offsets: vec![(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1)],
            range: 1,
        }
    }

    pub fn custom(name: impl Into<String>, offsets: Vec<(i32, i32)>) -> Self {
        let range = offsets
            .iter()
            .map(|(dx, dy)| dx.abs().max(dy.abs()) as u8)
            .max()
            .unwrap_or(1);
        Self {
            name: name.into(),
            offsets,
            range,
        }
    }

    pub fn size(&self) -> usize {
        self.offsets.len()
    }

    pub fn max_neighbor_count(&self) -> u8 {
        self.offsets.len() as u8
    }

    pub fn to_kernel_mask(&self, kernel_size: usize) -> Vec<bool> {
        let half = (kernel_size / 2) as i32;
        let mut mask = vec![false; kernel_size * kernel_size];
        for &(dx, dy) in &self.offsets {
            let x = (half + dx) as usize;
            let y = (half + dy) as usize;
            if x < kernel_size && y < kernel_size {
                mask[y * kernel_size + x] = true;
            }
        }
        mask
    }

    pub fn all_standard() -> Vec<Self> {
        vec![
            Self::moore(),
            Self::von_neumann(),
            Self::moore_extended(2),
            Self::von_neumann_extended(2),
            Self::hexagonal(),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryCondition {
    Toroidal,
    Fixed(u8),
    Reflective,
}

impl Default for BoundaryCondition {
    fn default() -> Self {
        Self::Toroidal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moore_size() {
        assert_eq!(Neighborhood::moore().size(), 8);
    }

    #[test]
    fn test_von_neumann_size() {
        assert_eq!(Neighborhood::von_neumann().size(), 4);
    }

    #[test]
    fn test_extended_moore() {
        let n = Neighborhood::moore_extended(2);
        assert_eq!(n.size(), 24); // 5x5 - 1 center
    }
}
