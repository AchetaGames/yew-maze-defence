//! Core data models for Maze Defence.
//! This module defines the initial types aligning with the GDD.
//! TODOs are included to guide future implementation.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::rc::Rc;
use yew::Reducible;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowDir { Up, Down, Left, Right }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirRole { Entrance, Exit }

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
    /// Direction indicators near the start (entrance/exit) with an arrow.
    Direction { dir: ArrowDir, role: DirRole },
    /// Indestructible tile (e.g., around Start to force entrance/exit).
    Indestructible,
    /// Target/exit that completes a loop (reserved for future use).
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
pub struct Enemy {
    /// World position in tile units (center-based), updated by simulation.
    pub x: f64,
    pub y: f64,
    /// Movement speed in tiles per second, fixed at spawn.
    pub speed_tps: f64,
    /// Hit points, fixed at spawn.
    pub hp: u32,
    /// The run time at which this enemy spawned.
    pub spawned_at: u64,
    /// Index of the next waypoint in path the enemy is moving towards.
    pub path_index: usize,
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
    /// Has the run's timer started (set on first action)?
    pub started: bool,
    /// Is the game currently paused (timer stops, interactions disabled except navigation)?
    pub is_paused: bool,
    /// Cached path from entrance to exit as grid tile positions.
    pub path: Vec<Position>,
    /// Active enemies in the world.
    pub enemies: Vec<Enemy>,
    /// Last second at which an enemy was spawned (for cadence control).
    pub last_enemy_spawn_time_secs: u64,
}

impl RunState {
    pub fn new_basic(grid_size: GridSize) -> Self {
        // Initialize grid with Rock tiles, deterministic distributions for gold and boosts.
        let mut tiles = Vec::with_capacity((grid_size.width * grid_size.height) as usize);
        for _y in 0..grid_size.height {
            for _x in 0..grid_size.width {
                // Randomize gold presence (~12% chance). Boosts are disabled initially (None).
                let r = js_sys::Math::random();
                let has_gold = r < 0.12;
                let boost = None;
                tiles.push(Tile {
                    kind: TileKind::Rock { has_gold, boost },
                    hardness: 3,
                });
            }
        }

        // Randomized Start near the center with randomized orientation (Entrance/Exit),
        // plus a short Empty path (ring) connecting Entrance to Exit.
        fn set_special(tiles: &mut Vec<Tile>, grid_size: GridSize, x: i32, y: i32, kind: TileKind) {
            if x >= 0 && y >= 0 && (x as u32) < grid_size.width && (y as u32) < grid_size.height {
                let idx = (y as u32 * grid_size.width + x as u32) as usize;
                tiles[idx].kind = kind;
                tiles[idx].hardness = 255; // unmineable for Start/Direction/Indestructible
            }
        }
        fn set_empty_if_rock(tiles: &mut Vec<Tile>, grid_size: GridSize, x: i32, y: i32) {
            if x >= 0 && y >= 0 && (x as u32) < grid_size.width && (y as u32) < grid_size.height {
                let idx = (y as u32 * grid_size.width + x as u32) as usize;
                if let TileKind::Rock { .. } = tiles[idx].kind {
                    tiles[idx].kind = TileKind::Empty;
                    tiles[idx].hardness = 1;
                }
            }
        }
        // Choose Start within central band, with safe margin so the ring fits.
        let min_margin: i32 = 3;
        let cx0 = (grid_size.width as i32) / 2;
        let cy0 = (grid_size.height as i32) / 2;
        let half_w = (grid_size.width as i32) / 4; // ±width/4
        let half_h = (grid_size.height as i32) / 4; // ±height/4
        let rand_range = |max_abs: i32| -> i32 {
            if max_abs <= 0 { 0 } else { ((js_sys::Math::random() * ((max_abs * 2 + 1) as f64)).floor() as i32) - max_abs }
        };
        let mut sx = cx0 + rand_range(half_w);
        let mut sy = cy0 + rand_range(half_h);
        // Clamp within margins
        let min_x = min_margin;
        let min_y = min_margin;
        let max_x = grid_size.width as i32 - 1 - min_margin;
        let max_y = grid_size.height as i32 - 1 - min_margin;
        if sx < min_x { sx = min_x; }
        if sx > max_x { sx = max_x; }
        if sy < min_y { sy = min_y; }
        if sy > max_y { sy = max_y; }
        // Orientation: 0=Right/Left, 1=Down/Up, 2=Left/Right, 3=Up/Down
        let orient = (js_sys::Math::random() * 4.0).floor() as i32;
        let (dx1, dy1, ent_dir) = match orient {
            0 => (1, 0, ArrowDir::Right),
            1 => (0, 1, ArrowDir::Down),
            2 => (-1, 0, ArrowDir::Left),
            _ => (0, -1, ArrowDir::Up),
        };
        // Place Start
        set_special(&mut tiles, grid_size, sx, sy, TileKind::Start);
        // Entrance: one tile in (dx1,dy1), arrow pointing away from start => ent_dir
        set_special(&mut tiles, grid_size, sx + dx1, sy + dy1, TileKind::Direction { dir: ent_dir, role: DirRole::Entrance });
        // Exit: one tile opposite, arrow pointing towards start => same ent_dir from that tile toward start
        set_special(&mut tiles, grid_size, sx - dx1, sy - dy1, TileKind::Direction { dir: ent_dir, role: DirRole::Exit });
        // Indestructibles on orthogonal sides
        match ent_dir {
            ArrowDir::Left | ArrowDir::Right => {
                set_special(&mut tiles, grid_size, sx, sy - 1, TileKind::Indestructible);
                set_special(&mut tiles, grid_size, sx, sy + 1, TileKind::Indestructible);
            }
            ArrowDir::Up | ArrowDir::Down => {
                set_special(&mut tiles, grid_size, sx - 1, sy, TileKind::Indestructible);
                set_special(&mut tiles, grid_size, sx + 1, sy, TileKind::Indestructible);
            }
        }
        // Carve a single-sided path from Entrance to Exit (no full ring)
        // Step 1: go forward from Entrance one tile
        set_empty_if_rock(&mut tiles, grid_size, sx + 2*dx1, sy + 2*dy1);
        // Step 2: turn perpendicular (left or right randomly) for a short corridor
        let sign: i32 = if js_sys::Math::random() < 0.5 { 1 } else { -1 };
        let px = -dy1 * sign;
        let py = dx1 * sign;
        for k in 1..=3 {
            set_empty_if_rock(&mut tiles, grid_size, sx + 2*dx1 + k*px, sy + 2*dy1 + k*py);
        }
        // Step 3: go back past Start towards Exit side
        for k in 1..=4 {
            set_empty_if_rock(&mut tiles, grid_size, sx + 2*dx1 + 3*px - k*dx1, sy + 2*dy1 + 3*py - k*dy1);
        }
        // Step 4: return perpendicular towards the Exit connector
        for k in 1..=3 {
            set_empty_if_rock(&mut tiles, grid_size, sx - 2*dx1 + (3 - k)*px, sy - 2*dy1 + (3 - k)*py);
        }
        // Final: ensure a direct neighbor to the Exit tile is open
        set_empty_if_rock(&mut tiles, grid_size, sx - 2*dx1, sy - 2*dy1);

        Self {
            grid_size,
            tiles,
            currencies: Currencies::default(),
            stats: RunStats::default(),
            life: 20,
            mining_speed: 1.0,
            started: false,
            is_paused: false,
            path: Vec::new(),
            enemies: Vec::new(),
            last_enemy_spawn_time_secs: 0,
        }
    }
}

// ---------------- Pathfinding helpers -----------------
fn dir_to_delta(dir: ArrowDir) -> (i32, i32) {
    match dir { ArrowDir::Up => (0,-1), ArrowDir::Down => (0,1), ArrowDir::Left => (-1,0), ArrowDir::Right => (1,0) }
}

fn find_entrance_exit(rs: &RunState) -> Option<((i32,i32,ArrowDir),(i32,i32,ArrowDir))> {
    let gs = rs.grid_size;
    let mut ent: Option<(i32,i32,ArrowDir)> = None;
    let mut exit: Option<(i32,i32,ArrowDir)> = None;
    for y in 0..gs.height { for x in 0..gs.width { let idx = (y * gs.width + x) as usize; if let TileKind::Direction { dir, role } = rs.tiles[idx].kind { match role { DirRole::Entrance => ent = Some((x as i32, y as i32, dir)), DirRole::Exit => exit = Some((x as i32, y as i32, dir)), } } } }
    match (ent, exit) { (Some(e), Some(x)) => Some((e,x)), _ => None }
}

pub fn compute_path(rs: &RunState) -> Vec<Position> {
    let gs = rs.grid_size;
    let Some(((ex, ey, edir), (xx, xy, xdir))) = find_entrance_exit(rs) else { return Vec::new(); };
    let (edx, edy) = dir_to_delta(edir);
    let (xdx, xdy) = dir_to_delta(xdir);
    let sx = ex + edx; // start from cell beyond entrance arrow
    let sy = ey + edy;
    let tx = xx - xdx; // end at cell before exit arrow (neighbor Empty ensured)
    let ty = xy - xdy;
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    if !in_bounds(sx, sy) || !in_bounds(tx, ty) { return Vec::new(); }
    let sidx = (sy as u32 * gs.width + sx as u32) as usize;
    let tidx = (ty as u32 * gs.width + tx as u32) as usize;
    let is_empty = |idx: usize| matches!(rs.tiles[idx].kind, TileKind::Empty);
    if !is_empty(sidx) || !is_empty(tidx) { return Vec::new(); }
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut visited = vec![false; (gs.width * gs.height) as usize];
    let mut parent: Vec<Option<usize>> = vec![None; (gs.width * gs.height) as usize];
    visited[sidx] = true; q.push_back(sidx);
    let dirs = [(1,0),(-1,0),(0,1),(0,-1)];
    while let Some(idx) = q.pop_front() {
        if idx == tidx { break; }
        let x = (idx as u32 % gs.width) as i32; let y = (idx as u32 / gs.width) as i32;
        for (dx,dy) in dirs { let nx = x + dx; let ny = y + dy; if !in_bounds(nx, ny) { continue; } let nidx = (ny as u32 * gs.width + nx as u32) as usize; if visited[nidx] { continue; } if !is_empty(nidx) { continue; } visited[nidx] = true; parent[nidx] = Some(idx); q.push_back(nidx); }
    }
    if !visited[tidx] { return Vec::new(); }
    let mut path_rev: Vec<usize> = Vec::new();
    let mut cur = Some(tidx);
    while let Some(ci) = cur { path_rev.push(ci); cur = parent[ci]; }
    path_rev.reverse();
    path_rev.into_iter().map(|i| { let x = (i as u32 % gs.width) as u32; let y = (i as u32 / gs.width) as u32; Position { x, y } }).collect()
}

// ---------------- Reducer & Actions -----------------
#[derive(Clone, Debug)]
pub enum RunAction {
    TogglePause,
    SetPaused(bool),
    StartRun,
    TickSecond, // called once per elapsed real second
    MiningComplete { idx: usize },
    SimTick { dt: f64 }, // ~16ms; advances enemies & spawns
}

impl Reducible for RunState {
    type Action = RunAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        use RunAction::*;
        let mut new = (*self).clone();
        match action {
            TogglePause => { new.is_paused = !new.is_paused; }
            SetPaused(p) => { new.is_paused = p; }
            StartRun => { if !new.started { new.started = true; } }
            TickSecond => {
                if new.started && !new.is_paused { new.stats.time_survived_secs = new.stats.time_survived_secs.saturating_add(1); }
            }
            MiningComplete { idx } => {
                if idx < new.tiles.len() {
                    if let TileKind::Rock { has_gold, .. } = new.tiles[idx].kind.clone() {
                        new.tiles[idx].kind = TileKind::Empty;
                        if new.stats.blocks_mined < u32::MAX { new.stats.blocks_mined += 1; }
                        new.currencies.tile_credits = new.currencies.tile_credits.saturating_add(1);
                        if has_gold { new.currencies.gold = new.currencies.gold.saturating_add(1); }
                        // Recompute path after terrain change
                        new.path = compute_path(&new);
                    }
                }
            }
            SimTick { dt } => {
                if !(new.started && !new.is_paused) { return self; }
                if new.path.is_empty() { new.path = compute_path(&new); }
                // Spawn cadence: every 2 seconds if path len>=2
                let t = new.stats.time_survived_secs;
                if new.path.len() >= 2 && t.saturating_sub(new.last_enemy_spawn_time_secs) >= 2 {
                    let first = new.path[0];
                    let speed = 1.0 + 0.002 * (t as f64);
                    let hp = 5 + (t / 10) as u32;
                    let e = Enemy { x: first.x as f64 + 0.5, y: first.y as f64 + 0.5, speed_tps: speed, hp, spawned_at: t, path_index: 1 };
                    new.enemies.push(e);
                    new.last_enemy_spawn_time_secs = t;
                }
                // Advance enemies
                if !new.path.is_empty() && dt > 0.0 {
                    let path_clone = new.path.clone();
                    let mut survivors: Vec<Enemy> = Vec::with_capacity(new.enemies.len());
                    for mut e in new.enemies.drain(..) {
                        let mut remaining = dt * e.speed_tps; // tiles to travel this frame
                        while remaining > 0.0 && e.path_index < path_clone.len() {
                            let wp = path_clone[e.path_index];
                            let tx = wp.x as f64 + 0.5; let ty = wp.y as f64 + 0.5;
                            let dx = tx - e.x; let dy = ty - e.y; let dist = (dx*dx + dy*dy).sqrt();
                            if dist < 1e-6 { e.path_index += 1; continue; }
                            if remaining >= dist { e.x = tx; e.y = ty; remaining -= dist; e.path_index += 1; } else { let ratio = remaining / dist; e.x += dx * ratio; e.y += dy * ratio; remaining = 0.0; }
                        }
                        if e.path_index >= path_clone.len() { if new.life > 0 { new.life -= 1; } if new.stats.loops_completed < u32::MAX { new.stats.loops_completed += 1; } } else { survivors.push(e); }
                    }
                    new.enemies = survivors;
                }
            }
        }
        Rc::new(new)
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
