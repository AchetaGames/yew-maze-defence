//! Core data models for Maze Defence.
//! This module defines the initial types aligning with the GDD.
//! TODOs are included to guide future implementation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoostKind {
    Range,
    Damage,
    FireRate,
    Slow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    /// Free/empty traversable floor.
    Empty,
    /// Rock that can be mined; may contain gold and/or a boost.
    Rock { has_gold: bool, boost: Option<BoostKind> },
    /// Player-placed wall that blocks pathing.
    Wall,
    /// Enemy spawn.
    Start,
    /// Target/exit that completes a loop.
    End,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    /// How hard the tile is to mine (higher = takes longer). Wall may be immutable in MVP.
    pub hardness: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Currencies {
    pub gold: u64,
    pub research: u64,
    /// How many blocks mined (used as credits for placements within a run initially).
    pub tile_credits: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStats {
    pub time_survived_secs: u64,
    pub loops_completed: u32,
    pub blocks_mined: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunState {
    pub grid_size: GridSize,
    /// Row-major tiles; length = width * height.
    pub tiles: Vec<Tile>,
    pub currencies: Currencies,
    pub stats: RunStats,
    /// Player life for the current run.
    pub life: u32,
    /// Mining speed multiplier (tiles per second relative to hardness units).
    pub mining_speed: f64,
    // TODO: path: Vec<Position>
    // TODO: towers, enemies
}

impl RunState {
    pub fn new_basic(grid_size: GridSize) -> Self {
        // Initialize grid with Rock tiles, deterministic gold pattern.
        let mut tiles = Vec::with_capacity((grid_size.width * grid_size.height) as usize);
        for y in 0..grid_size.height {
            for x in 0..grid_size.width {
                let idx_val = (x as u64) * 31 + (y as u64) * 17;
                let has_gold = (idx_val % 7) == 0; // simple deterministic distribution
                tiles.push(Tile {
                    kind: TileKind::Rock { has_gold, boost: None },
                    hardness: 3,
                });
            }
        }
        Self {
            grid_size,
            tiles,
            currencies: Currencies::default(),
            stats: RunStats::default(),
            life: 20,
            mining_speed: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UpgradeState {
    pub mining_speed_level: u8,
    pub starting_gold_bonus: u32,
    /// For MVP, towers refund fully. Later make configurable.
    pub tower_refund_rate_percent: u8,
    // TODO: additional permanent upgrades
}

// TODO: Implement BFS/A* pathfinding utilities ensuring Start->End remains reachable when placing walls.
// TODO: Add persistence helpers (e.g., serialize/deserialize UpgradeState to localStorage via gloo-storage).
