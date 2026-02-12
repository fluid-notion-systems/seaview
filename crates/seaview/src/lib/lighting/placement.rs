//! Light placement algorithms for distributing lights across the scene
//!
//! This module implements various algorithms for positioning lights in a 2D plane
//! to provide even coverage of the scene.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Algorithm for placing lights in the scene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementAlgorithm {
    /// Uniform grid layout (rows x cols closest to sqrt(N))
    UniformGrid,

    /// Hexagonal packing with offset rows for better coverage
    HexagonalPacking,

    /// Poisson disk sampling for random but evenly spaced distribution
    PoissonDisk,

    /// Single horizontal row
    SingleRow,

    /// Single vertical column
    SingleColumn,
}

impl PlacementAlgorithm {
    /// Get all available placement algorithms
    pub fn all() -> &'static [PlacementAlgorithm] {
        &[
            PlacementAlgorithm::UniformGrid,
            PlacementAlgorithm::HexagonalPacking,
            PlacementAlgorithm::PoissonDisk,
            PlacementAlgorithm::SingleRow,
            PlacementAlgorithm::SingleColumn,
        ]
    }

    /// Get a human-readable name for this algorithm
    pub fn name(&self) -> &'static str {
        match self {
            PlacementAlgorithm::UniformGrid => "Uniform Grid",
            PlacementAlgorithm::HexagonalPacking => "Hexagonal Packing",
            PlacementAlgorithm::PoissonDisk => "Poisson Disk",
            PlacementAlgorithm::SingleRow => "Single Row",
            PlacementAlgorithm::SingleColumn => "Single Column",
        }
    }

    /// Calculate positions for N lights within the given bounds
    ///
    /// # Arguments
    /// * `num_lights` - Number of lights to place
    /// * `bounds_min` - Minimum X and Z coordinates of the scene
    /// * `bounds_max` - Maximum X and Z coordinates of the scene
    ///
    /// # Returns
    /// Vector of (x, z) positions for each light
    pub fn calculate_positions(
        &self,
        num_lights: usize,
        bounds_min: Vec2,
        bounds_max: Vec2,
    ) -> Vec<Vec2> {
        if num_lights == 0 {
            return Vec::new();
        }

        match self {
            PlacementAlgorithm::UniformGrid => {
                calculate_uniform_grid(num_lights, bounds_min, bounds_max)
            }
            PlacementAlgorithm::HexagonalPacking => {
                calculate_hexagonal_packing(num_lights, bounds_min, bounds_max)
            }
            PlacementAlgorithm::PoissonDisk => {
                calculate_poisson_disk(num_lights, bounds_min, bounds_max)
            }
            PlacementAlgorithm::SingleRow => {
                calculate_single_row(num_lights, bounds_min, bounds_max)
            }
            PlacementAlgorithm::SingleColumn => {
                calculate_single_column(num_lights, bounds_min, bounds_max)
            }
        }
    }
}

/// Calculate uniform grid positions
fn calculate_uniform_grid(num_lights: usize, bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let size = bounds_max - bounds_min;

    // Calculate grid dimensions (rows x cols closest to sqrt(N))
    let cols = (num_lights as f32).sqrt().ceil() as usize;
    let rows = (num_lights + cols - 1) / cols; // Ceiling division

    let mut positions = Vec::with_capacity(num_lights);

    for i in 0..num_lights {
        let row = i / cols;
        let col = i % cols;

        // Center the grid and add small offsets to avoid edges
        let x = bounds_min.x + size.x * (col as f32 + 0.5) / cols as f32;
        let z = bounds_min.y + size.y * (row as f32 + 0.5) / rows as f32;

        positions.push(Vec2::new(x, z));
    }

    positions
}

/// Calculate hexagonal packing positions
fn calculate_hexagonal_packing(num_lights: usize, bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let size = bounds_max - bounds_min;

    // Hexagonal packing has better coverage with offset rows
    let cols = (num_lights as f32).sqrt().ceil() as usize;
    let rows = (num_lights + cols - 1) / cols;

    let mut positions = Vec::with_capacity(num_lights);

    for i in 0..num_lights {
        let row = i / cols;
        let col = i % cols;

        // Offset every other row by half a column width
        let x_offset = if row % 2 == 1 { 0.5 / cols as f32 } else { 0.0 };

        let x = bounds_min.x + size.x * (col as f32 + 0.5 + x_offset * cols as f32) / cols as f32;
        let z = bounds_min.y + size.y * (row as f32 + 0.5) / rows as f32;

        positions.push(Vec2::new(x, z));
    }

    positions
}

/// Calculate Poisson disk sampling positions
/// This creates a random but evenly-spaced distribution
fn calculate_poisson_disk(num_lights: usize, bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let size = bounds_max - bounds_min;

    // For simplicity, use a dart-throwing approach with minimum distance
    let area = size.x * size.y;
    let min_distance = (area / num_lights as f32).sqrt() * 0.8;

    let mut positions: Vec<Vec2> = Vec::with_capacity(num_lights);
    let mut rng_state = 12345u32; // Simple LCG for deterministic results

    let max_attempts = num_lights * 30;
    let mut attempts = 0;

    while positions.len() < num_lights && attempts < max_attempts {
        attempts += 1;

        // Simple linear congruential generator
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let rand_x = (rng_state >> 16) as f32 / 65535.0;

        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let rand_z = (rng_state >> 16) as f32 / 65535.0;

        let candidate = Vec2::new(
            bounds_min.x + size.x * rand_x,
            bounds_min.y + size.y * rand_z,
        );

        // Check if this position is far enough from all others
        let mut valid = true;
        for pos in &positions {
            if pos.distance(candidate) < min_distance {
                valid = false;
                break;
            }
        }

        if valid {
            positions.push(candidate);
        }
    }

    // If we couldn't place all lights, fall back to grid for remaining
    if positions.len() < num_lights {
        let remaining = num_lights - positions.len();
        let grid_positions = calculate_uniform_grid(remaining, bounds_min, bounds_max);
        positions.extend(grid_positions);
    }

    positions
}

/// Calculate single row positions
fn calculate_single_row(num_lights: usize, bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let size = bounds_max - bounds_min;
    let center_z = (bounds_min.y + bounds_max.y) / 2.0;

    let mut positions = Vec::with_capacity(num_lights);

    for i in 0..num_lights {
        let x = bounds_min.x + size.x * (i as f32 + 0.5) / num_lights as f32;
        positions.push(Vec2::new(x, center_z));
    }

    positions
}

/// Calculate single column positions
fn calculate_single_column(num_lights: usize, bounds_min: Vec2, bounds_max: Vec2) -> Vec<Vec2> {
    let size = bounds_max - bounds_min;
    let center_x = (bounds_min.x + bounds_max.x) / 2.0;

    let mut positions = Vec::with_capacity(num_lights);

    for i in 0..num_lights {
        let z = bounds_min.y + size.y * (i as f32 + 0.5) / num_lights as f32;
        positions.push(Vec2::new(center_x, z));
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_grid() {
        let positions = calculate_uniform_grid(9, Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        assert_eq!(positions.len(), 9);

        // Should form a 3x3 grid
        // Check that we have positions distributed across the space
        assert!(positions.iter().any(|p| p.x < 40.0 && p.y < 40.0));
        assert!(positions.iter().any(|p| p.x > 60.0 && p.y > 60.0));
    }

    #[test]
    fn test_single_row() {
        let positions = calculate_single_row(5, Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        assert_eq!(positions.len(), 5);

        // All should have same Z coordinate (center)
        let z_values: Vec<f32> = positions.iter().map(|p| p.y).collect();
        assert!(z_values.iter().all(|&z| (z - 50.0).abs() < 0.1));
    }

    #[test]
    fn test_single_column() {
        let positions = calculate_single_column(5, Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        assert_eq!(positions.len(), 5);

        // All should have same X coordinate (center)
        let x_values: Vec<f32> = positions.iter().map(|p| p.x).collect();
        assert!(x_values.iter().all(|&x| (x - 50.0).abs() < 0.1));
    }
}
