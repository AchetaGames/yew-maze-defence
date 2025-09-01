//! Core data models for Maze Defence.
//! This module defines the initial types aligning with the GDD.
//! TODOs are included to guide future implementation.

use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::JsValue;
use yew::Reducible; // added for logging // re-add for debug logging

#[allow(dead_code)]
const DEBUG_LOG: bool = false;
#[allow(dead_code)]
fn dlog(msg: &str) {
    if DEBUG_LOG {
        web_sys::console::log_1(&JsValue::from_str(msg));
    }
}

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
pub enum ArrowDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirRole {
    Entrance,
    Exit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    /// Free/empty traversable floor.
    Empty,
    /// Rock that can be mined; may contain gold and/or a boost.
    Rock {
        has_gold: bool,
        boost: Option<BoostKind>,
    },
    /// Player-placed wall that blocks pathing.
    Wall,
    /// Enemy spawn.
    Start,
    /// Direction indicators near the start (entrance/exit) with an arrow.
    Direction {
        dir: ArrowDir,
        role: DirRole,
    },
    /// Indestructible tile (e.g., around Start to force entrance/exit).
    Indestructible,
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
    pub x: f64,
    pub y: f64,
    pub speed_tps: f64,
    pub hp: u32,
    pub spawned_at: u64,
    pub path_index: usize, // kept for minimal UI/debug compatibility (next node index)
    pub dir_dx: f64,
    pub dir_dy: f64,
    pub radius_scale: f64,
    pub loop_dist: f64, // continuous distance along loop polyline [0, loop_total_length)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunState {
    pub grid_size: GridSize,
    pub tiles: Vec<Tile>,
    pub currencies: Currencies,
    pub stats: RunStats,
    pub life: u32,
    pub mining_speed: f64,
    pub started: bool,
    pub is_paused: bool,
    pub path: Vec<Position>,
    pub path_loop: Vec<Position>, // cyclic path including start/entrance/exit
    pub loop_cum_lengths: Vec<f64>, // cumulative lengths per node (same length as path_loop)
    pub loop_total_length: f64,
    pub enemies: Vec<Enemy>,
    pub last_enemy_spawn_time_secs: u64,
    pub version: u64,
    pub game_over: bool,
    pub last_mined_idx: Option<usize>,
    pub sim_time: f64,
    pub towers: Vec<Tower>,
    pub tower_base_range: f64,
    pub tower_base_damage: u32,
    pub tower_cost: u64,
    pub projectiles: Vec<Projectile>,
    pub run_id: u64, // NEW: increments each reset to allow camera re-center
    // === Derived / upgrade influenced fields ===
    pub life_max: u32,
    pub life_regen_per_sec: f64,
    pub life_regen_accum: f64,
    pub tower_fire_rate_global: f64,
    pub crit_chance: f64,
    pub crit_damage_mult: f64,
    pub gold_bounty_per_kill: u64,
    pub gold_bounty_mul: f64,
    pub damage_ramp_per_sec: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TowerKind {
    Basic,
    // Future variants influenced by boost tiles (e.g., Slow, Damage, Range, FireRate)
    Slow,
    Damage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tower {
    pub x: u32,
    pub y: u32,
    pub kind: TowerKind,
    pub range: f64,
    pub damage: u32,
    pub fire_rate: f64,          // shots per second
    pub cooldown_remaining: f64, // seconds until next shot
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub remaining: f64, // seconds until impact
    pub damage: u32,
}

impl Tower {
    pub fn new(x: u32, y: u32, kind: TowerKind, base_range: f64, base_damage: u32) -> Self {
        // Basic defaults; could vary by kind
        let (range_mul, dmg_mul, fire_rate) = match kind {
            TowerKind::Basic => (1.0, 1.0, 1.0),
            TowerKind::Slow => (1.1, 0.5, 0.75),
            TowerKind::Damage => (0.9, 1.5, 0.8),
        };
        Self {
            x,
            y,
            kind,
            range: base_range * range_mul,
            damage: (base_damage as f64 * dmg_mul).round() as u32,
            fire_rate,
            cooldown_remaining: 0.0,
        }
    }
}

impl RunState {
    fn create_run_base(
        grid_size: GridSize,
        gold_chance: f64,
        boost_kinds: &[BoostKind],
        boost_freq_weight: f64,
    ) -> Self {
        // 1. Fill with rocks (with gold/boost distributions)
        let mut tiles: Vec<Tile> =
            Vec::with_capacity((grid_size.width * grid_size.height) as usize);
        for _y in 0..grid_size.height {
            for _x in 0..grid_size.width {
                let r = js_sys::Math::random();
                let has_gold = r < gold_chance;
                let boost = if boost_kinds.is_empty() {
                    None
                } else {
                    let spawn_chance = 0.05 * boost_freq_weight; // base 5% scaled
                    if js_sys::Math::random() < spawn_chance.min(0.90) {
                        // cap to avoid flooding
                        let idx =
                            (js_sys::Math::random() * boost_kinds.len() as f64).floor() as usize;
                        Some(boost_kinds[idx])
                    } else {
                        None
                    }
                };
                tiles.push(Tile {
                    kind: TileKind::Rock { has_gold, boost },
                    hardness: 3,
                });
            }
        }
        // 2. Carve start + entrance/exit + initial corridor
        fn set_special(tiles: &mut [Tile], gs: GridSize, x: i32, y: i32, kind: TileKind) {
            if x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height {
                let idx = (y as u32 * gs.width + x as u32) as usize;
                tiles[idx].kind = kind;
                tiles[idx].hardness = 255;
            }
        }
        fn set_empty_if_rock(tiles: &mut [Tile], gs: GridSize, x: i32, y: i32) {
            if x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height {
                let idx = (y as u32 * gs.width + x as u32) as usize;
                if let TileKind::Rock { .. } = tiles[idx].kind {
                    tiles[idx].kind = TileKind::Empty;
                    tiles[idx].hardness = 1;
                }
            }
        }
        let min_margin: i32 = 3;
        let cx0 = (grid_size.width as i32) / 2;
        let cy0 = (grid_size.height as i32) / 2;
        let half_w = (grid_size.width as i32) / 4;
        let half_h = (grid_size.height as i32) / 4;
        let rand_range = |max_abs: i32| {
            if max_abs <= 0 {
                0
            } else {
                ((js_sys::Math::random() * ((max_abs * 2 + 1) as f64)).floor() as i32) - max_abs
            }
        };
        let mut sx = cx0 + rand_range(half_w);
        let mut sy = cy0 + rand_range(half_h);
        let min_x = min_margin;
        let min_y = min_margin;
        let max_x = grid_size.width as i32 - 1 - min_margin;
        let max_y = grid_size.height as i32 - 1 - min_margin;
        if sx < min_x {
            sx = min_x;
        }
        if sx > max_x {
            sx = max_x;
        }
        if sy < min_y {
            sy = min_y;
        }
        if sy > max_y {
            sy = max_y;
        }
        let orient = (js_sys::Math::random() * 4.0).floor() as i32;
        let (dx1, dy1, ent_dir) = match orient {
            0 => (1, 0, ArrowDir::Right),
            1 => (0, 1, ArrowDir::Down),
            2 => (-1, 0, ArrowDir::Left),
            _ => (0, -1, ArrowDir::Up),
        };
        set_special(&mut tiles, grid_size, sx, sy, TileKind::Start);
        set_special(
            &mut tiles,
            grid_size,
            sx + dx1,
            sy + dy1,
            TileKind::Direction {
                dir: ent_dir,
                role: DirRole::Entrance,
            },
        );
        set_special(
            &mut tiles,
            grid_size,
            sx - dx1,
            sy - dy1,
            TileKind::Direction {
                dir: ent_dir,
                role: DirRole::Exit,
            },
        );
        match ent_dir {
            ArrowDir::Left | ArrowDir::Right => {
                set_special(&mut tiles, grid_size, sx, sy - 1, TileKind::Indestructible);
                set_special(&mut tiles, grid_size, sx, sy + 1, TileKind::Indestructible);
            }
            _ => {
                set_special(&mut tiles, grid_size, sx - 1, sy, TileKind::Indestructible);
                set_special(&mut tiles, grid_size, sx + 1, sy, TileKind::Indestructible);
            }
        }
        // corridor carving
        set_empty_if_rock(&mut tiles, grid_size, sx + 2 * dx1, sy + 2 * dy1);
        let sign = if js_sys::Math::random() < 0.5 { 1 } else { -1 };
        let px = -dy1 * sign;
        let py = dx1 * sign;
        for k in 1..=3 {
            set_empty_if_rock(
                &mut tiles,
                grid_size,
                sx + 2 * dx1 + k * px,
                sy + 2 * dy1 + k * py,
            );
        }
        for k in 1..=4 {
            set_empty_if_rock(
                &mut tiles,
                grid_size,
                sx + 2 * dx1 + 3 * px - k * dx1,
                sy + 2 * dy1 + 3 * py - k * dy1,
            );
        }
        for k in 1..=3 {
            set_empty_if_rock(
                &mut tiles,
                grid_size,
                sx - 2 * dx1 + (3 - k) * px,
                sy - 2 * dy1 + (3 - k) * py,
            );
        }
        set_empty_if_rock(&mut tiles, grid_size, sx - 2 * dx1, sy - 2 * dy1);
        let mut rs = RunState {
            grid_size,
            tiles,
            currencies: Currencies {
                gold: 5,
                ..Default::default()
            },
            stats: RunStats::default(),
            life: 20,
            mining_speed: 6.0,
            started: false,
            is_paused: false,
            path: Vec::new(),
            path_loop: Vec::new(),
            loop_cum_lengths: Vec::new(),
            loop_total_length: 0.0,
            enemies: Vec::new(),
            last_enemy_spawn_time_secs: 0,
            version: 0,
            game_over: false,
            last_mined_idx: None,
            sim_time: 0.0,
            towers: Vec::new(),
            tower_base_range: 3.5,
            tower_base_damage: 2,
            tower_cost: 2,
            projectiles: Vec::new(),
            run_id: 0,
            life_max: 20,
            life_regen_per_sec: 0.0,
            life_regen_accum: 0.0,
            tower_fire_rate_global: 1.0,
            crit_chance: 0.0,
            crit_damage_mult: 1.0,
            gold_bounty_per_kill: 0,
            gold_bounty_mul: 1.0,
            damage_ramp_per_sec: 0.0,
        };
        rs.path = compute_path(&rs);
        rs.path_loop = build_loop_path(&rs);
        update_loop_geometry(&mut rs);
        rs
    }
    pub fn new_basic(grid_size: GridSize) -> Self {
        Self::create_run_base(grid_size, 0.12, &[], 1.0)
    }
    pub fn new_with_upgrades(base: GridSize, ups: &UpgradeState) -> Self {
        use crate::model::UpgradeId::*;
        let expand_lvl = ups.level(GridExpand) as u32;
        let grid_size = GridSize {
            width: base.width + expand_lvl * 2,
            height: base.height + expand_lvl * 2,
        };
        let gold_chance = (0.12 + 0.03 * ups.level(GoldSpawn) as f64).min(0.95);
        let mut boost_kinds: Vec<BoostKind> = Vec::new();
        if ups.level(BoostTilesUnlock) > 0 {
            boost_kinds.push(BoostKind::Slow);
            boost_kinds.push(BoostKind::Damage);
            if ups.level(BoostTileDiversity) > 0 {
                boost_kinds.push(BoostKind::Range);
                boost_kinds.push(BoostKind::FireRate);
            }
        }
        let freq_weight = 1.0 + 0.20 * ups.level(BoostTileFrequency) as f64;
        Self::create_run_base(grid_size, gold_chance, &boost_kinds, freq_weight)
    }
}

// ---------------- Pathfinding helpers -----------------
// Replace BFS-based compute_path with A* (Manhattan heuristic)
fn a_star_path(rs: &RunState, start: (i32, i32), goal: (i32, i32)) -> Vec<Position> {
    let (sx, sy) = start;
    let (gx, gy) = goal;
    let gs = rs.grid_size;
    let in_bounds =
        |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    if !in_bounds(sx, sy) || !in_bounds(gx, gy) {
        return Vec::new();
    }
    // Only allow traversal through Empty tiles; exclude Start and Direction tiles to avoid ping-pong through them
    let is_walkable = |idx: usize| matches!(rs.tiles[idx].kind, TileKind::Empty);
    let start_idx = (sy as u32 * gs.width + sx as u32) as usize;
    let goal_idx = (gy as u32 * gs.width + gx as u32) as usize;
    if !is_walkable(start_idx) || !is_walkable(goal_idx) {
        return Vec::new();
    }
    use std::cmp::Ordering;
    use std::collections::{BinaryHeap, HashMap};
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        f: u32,
        idx: usize,
    }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other.f.cmp(&self.f).then_with(|| self.idx.cmp(&other.idx))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }
    let mut open = BinaryHeap::new();
    let mut g: HashMap<usize, u32> = HashMap::new();
    let mut parent: Vec<Option<usize>> = vec![None; (gs.width * gs.height) as usize];
    let h = |x: i32, y: i32| ((x - gx).abs() + (y - gy).abs()) as u32; // Manhattan
    g.insert(start_idx, 0);
    open.push(Node {
        f: h(sx, sy),
        idx: start_idx,
    });
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    while let Some(Node { idx, .. }) = open.pop() {
        if idx == goal_idx {
            break;
        }
        let gx0 = (idx as u32 % gs.width) as i32;
        let gy0 = (idx as u32 / gs.width) as i32;
        let g_here = *g.get(&idx).unwrap();
        for (dx, dy) in dirs {
            let nx = gx0 + dx;
            let ny = gy0 + dy;
            if !in_bounds(nx, ny) {
                continue;
            }
            let nidx = (ny as u32 * gs.width + nx as u32) as usize;
            if !is_walkable(nidx) {
                continue;
            }
            let tentative = g_here + 1;
            if tentative < *g.get(&nidx).unwrap_or(&u32::MAX) {
                g.insert(nidx, tentative);
                parent[nidx] = Some(idx);
                let f = tentative + h(nx, ny);
                open.push(Node { f, idx: nidx });
            }
        }
    }
    if parent[goal_idx].is_none() && start_idx != goal_idx {
        return Vec::new();
    }
    // reconstruct
    let mut rev = Vec::new();
    let mut cur = Some(goal_idx);
    while let Some(ci) = cur {
        rev.push(ci);
        if ci == start_idx {
            break;
        }
        cur = parent[ci];
    }
    rev.reverse();
    rev.into_iter()
        .map(|i| {
            let x = (i as u32 % gs.width) as u32;
            let y = (i as u32 / gs.width) as u32;
            Position { x, y }
        })
        .collect()
}

fn find_entrance_exit(rs: &RunState) -> Option<((i32, i32, ArrowDir), (i32, i32, ArrowDir))> {
    let gs = rs.grid_size;
    let mut ent: Option<(i32, i32, ArrowDir)> = None;
    let mut exit: Option<(i32, i32, ArrowDir)> = None;
    for y in 0..gs.height {
        for x in 0..gs.width {
            let idx = (y * gs.width + x) as usize;
            if let TileKind::Direction { dir, role } = rs.tiles[idx].kind {
                match role {
                    DirRole::Entrance => ent = Some((x as i32, y as i32, dir)),
                    DirRole::Exit => exit = Some((x as i32, y as i32, dir)),
                }
            }
        }
    }
    match (ent, exit) {
        (Some(e), Some(x)) => Some((e, x)),
        _ => None,
    }
}

// Global preview path (entrance walkway start -> exit walkway end)
pub fn compute_path(rs: &RunState) -> Vec<Position> {
    let Some(((ex, ey, _edir), (xx, xy, _xdir))) = find_entrance_exit(rs) else {
        return Vec::new();
    };
    let gs = rs.grid_size;
    let in_bounds =
        |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    let is_empty = |x: i32, y: i32| {
        if !in_bounds(x, y) {
            return false;
        }
        let idx = (y as u32 * gs.width + x as u32) as usize;
        matches!(rs.tiles[idx].kind, TileKind::Empty)
    };
    // Collect candidate start neighbors (empties adjacent to entrance direction tile)
    let mut starts: Vec<(i32, i32)> = Vec::new();
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = ex + dx;
        let ny = ey + dy;
        if is_empty(nx, ny) {
            starts.push((nx, ny));
        }
    }
    // Collect candidate goal neighbors (empties adjacent to exit direction tile)
    let mut goals: Vec<(i32, i32)> = Vec::new();
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = xx + dx;
        let ny = xy + dy;
        if is_empty(nx, ny) {
            goals.push((nx, ny));
        }
    }
    if starts.is_empty() || goals.is_empty() {
        return Vec::new();
    }
    // Evaluate all start->goal pairs; pick shortest non-empty path
    let mut best: Option<Vec<Position>> = None;
    let mut best_len = usize::MAX;
    for (sx, sy) in &starts {
        for (gx, gy) in &goals {
            if sx == gx && sy == gy {
                continue;
            }
            let path = a_star_path(rs, (*sx, *sy), (*gx, *gy));
            if path.len() > 1 && path.len() < best_len {
                best_len = path.len();
                best = Some(path);
            }
        }
    }
    best.unwrap_or_default()
}

fn build_loop_path(rs: &RunState) -> Vec<Position> {
    // Build cyclic ordered nodes: Start -> EntranceDir -> path (entrance->exit cells) -> ExitDir
    let mut start_pos = None;
    let mut ent_dir_tile = None;
    let mut exit_dir_tile = None;
    for y in 0..rs.grid_size.height {
        for x in 0..rs.grid_size.width {
            let idx = (y * rs.grid_size.width + x) as usize;
            match rs.tiles[idx].kind {
                TileKind::Start => start_pos = Some(Position { x, y }),
                TileKind::Direction { dir: _, role } => match role {
                    DirRole::Entrance => ent_dir_tile = Some(Position { x, y }),
                    DirRole::Exit => exit_dir_tile = Some(Position { x, y }),
                },
                _ => {}
            }
        }
    }
    let (Some(start), Some(ent), Some(exit)) = (start_pos, ent_dir_tile, exit_dir_tile) else {
        return Vec::new();
    };
    let mut loop_nodes = Vec::new();
    loop_nodes.push(start);
    if loop_nodes.last().unwrap() != &ent {
        loop_nodes.push(ent);
    }
    // path should only contain empty tiles between entrance and exit (already enforced by a_star_path)
    for p in &rs.path {
        if p != &start && p != &ent && p != &exit {
            loop_nodes.push(*p);
        }
    }
    if loop_nodes.last().unwrap() != &exit {
        loop_nodes.push(exit);
    }
    // Clean: remove immediate duplicates
    let mut cleaned: Vec<Position> = Vec::with_capacity(loop_nodes.len());
    for node in loop_nodes.into_iter() {
        if cleaned.last() == Some(&node) {
            continue;
        }
        cleaned.push(node);
    }
    // Remove immediate reversals A,B,A -> drop middle B (or second A?). We want monotonic forward progression; remove the middle node causing reversal pattern B.
    let mut no_reversal: Vec<Position> = Vec::with_capacity(cleaned.len());
    for n in cleaned.into_iter() {
        if no_reversal.len() >= 2 {
            let a = no_reversal[no_reversal.len() - 2];
            let _b = no_reversal[no_reversal.len() - 1];
            if a.x == n.x && a.y == n.y {
                // pattern A,B,A -> drop B
                no_reversal.pop(); // remove B
            }
        }
        no_reversal.push(n);
    }
    no_reversal
}

fn update_loop_geometry(rs: &mut RunState) {
    rs.loop_cum_lengths.clear();
    rs.loop_total_length = 0.0;
    if rs.path_loop.len() < 2 {
        return;
    }
    rs.loop_cum_lengths.reserve(rs.path_loop.len());
    rs.loop_cum_lengths.push(0.0);
    let mut acc = 0.0;
    for i in 1..rs.path_loop.len() {
        let a = rs.path_loop[i - 1];
        let b = rs.path_loop[i];
        let dx = b.x as f64 - a.x as f64;
        let dy = b.y as f64 - a.y as f64;
        let seg = (dx * dx + dy * dy).sqrt();
        acc += seg;
        rs.loop_cum_lengths.push(acc);
    }
    // add implicit closing segment from last back to first
    let first = rs.path_loop[0];
    let last = *rs.path_loop.last().unwrap();
    let dx = first.x as f64 - last.x as f64;
    let dy = first.y as f64 - last.y as f64;
    let closing = (dx * dx + dy * dy).sqrt();
    rs.loop_total_length = acc + closing;
}

// === Upgrade Tree System ===
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeId {
    MiningSpeed,
    TowerDamage,
    TowerRange,
    FireRate,
    CritChance,
    CritDamage,
    StartingGold,
    Health,
    GoldGain,
    GoldSpawn,
    BoostTilesUnlock,
    BoostTileFrequency,
    BoostTileDiversity,
    LifeRegen,
    TowerDamage2,
    DamageRamp,
    GridExpand,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UnlockCondition {
    Always,
    AnyLevel(UpgradeId),
    Maxed(UpgradeId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpgradeDef {
    pub id: UpgradeId,
    pub name: &'static str,
    pub desc: &'static str,
    pub max_level: u8,
    pub base_cost: u64,
    pub cost_growth: f64,
    pub unlock: UnlockCondition,
}

pub static UPGRADE_DEFS: &[UpgradeDef] = &[
    // Stage 1 (always visible basics)
    UpgradeDef {
        id: UpgradeId::MiningSpeed,
        name: "Mining Speed",
        desc: "+15% base mining speed / lvl",
        max_level: 5,
        base_cost: 10,
        cost_growth: 1.6,
        unlock: UnlockCondition::Always,
    },
    UpgradeDef {
        id: UpgradeId::TowerDamage,
        name: "Tower Damage I",
        desc: "+12% base tower damage / lvl",
        max_level: 5,
        base_cost: 12,
        cost_growth: 1.6,
        unlock: UnlockCondition::Always,
    },
    UpgradeDef {
        id: UpgradeId::StartingGold,
        name: "Starting Gold",
        desc: "+3 starting gold / lvl (new run)",
        max_level: 4,
        base_cost: 10,
        cost_growth: 1.5,
        unlock: UnlockCondition::Always,
    },
    UpgradeDef {
        id: UpgradeId::Health,
        name: "Max Life",
        desc: "+5 max life / lvl (new run)",
        max_level: 5,
        base_cost: 14,
        cost_growth: 1.55,
        unlock: UnlockCondition::Always,
    },
    // Stage 2 (need any prior progress)
    UpgradeDef {
        id: UpgradeId::TowerRange,
        name: "Tower Range",
        desc: "+6% tower range / lvl",
        max_level: 5,
        base_cost: 14,
        cost_growth: 1.55,
        unlock: UnlockCondition::AnyLevel(UpgradeId::TowerDamage),
    },
    UpgradeDef {
        id: UpgradeId::FireRate,
        name: "Fire Rate",
        desc: "+8% fire rate / lvl",
        max_level: 5,
        base_cost: 16,
        cost_growth: 1.55,
        unlock: UnlockCondition::AnyLevel(UpgradeId::TowerDamage),
    },
    UpgradeDef {
        id: UpgradeId::GoldGain,
        name: "Gold Bounty",
        desc: "+4% gold per kill / lvl (base 1)",
        max_level: 5,
        base_cost: 20,
        cost_growth: 1.6,
        unlock: UnlockCondition::AnyLevel(UpgradeId::MiningSpeed),
    },
    UpgradeDef {
        id: UpgradeId::GoldSpawn,
        name: "Gold Spawn",
        desc: "+3% chance a rock has gold / lvl (future runs)",
        max_level: 5,
        base_cost: 22,
        cost_growth: 1.6,
        unlock: UnlockCondition::AnyLevel(UpgradeId::MiningSpeed),
    },
    // Stage 3 (branch unlocks)
    UpgradeDef {
        id: UpgradeId::LifeRegen,
        name: "Life Regeneration",
        desc: "+0.5 life/sec / lvl (in-run)",
        max_level: 4,
        base_cost: 28,
        cost_growth: 1.6,
        unlock: UnlockCondition::AnyLevel(UpgradeId::Health),
    },
    UpgradeDef {
        id: UpgradeId::BoostTilesUnlock,
        name: "Unlock Boost Tiles",
        desc: "Enable spawning of Slow & Damage boost tiles (future runs)",
        max_level: 1,
        base_cost: 30,
        cost_growth: 2.0,
        unlock: UnlockCondition::AnyLevel(UpgradeId::MiningSpeed),
    },
    UpgradeDef {
        id: UpgradeId::TowerDamage2,
        name: "Tower Damage II",
        desc: "+10% additional tower damage / lvl",
        max_level: 5,
        base_cost: 30,
        cost_growth: 1.7,
        unlock: UnlockCondition::Maxed(UpgradeId::TowerDamage),
    },
    UpgradeDef {
        id: UpgradeId::DamageRamp,
        name: "Damage Ramp",
        desc: "+3% damage per enemy second alive / lvl",
        max_level: 4,
        base_cost: 32,
        cost_growth: 1.65,
        unlock: UnlockCondition::AnyLevel(UpgradeId::FireRate),
    },
    UpgradeDef {
        id: UpgradeId::BoostTileFrequency,
        name: "Boost Frequency",
        desc: "+20% boost tile weight / lvl",
        max_level: 4,
        base_cost: 35,
        cost_growth: 1.65,
        unlock: UnlockCondition::AnyLevel(UpgradeId::BoostTilesUnlock),
    },
    // Stage 4 (late game)
    UpgradeDef {
        id: UpgradeId::CritChance,
        name: "Crit Chance",
        desc: "+2% crit chance / lvl",
        max_level: 5,
        base_cost: 25,
        cost_growth: 1.65,
        unlock: UnlockCondition::AnyLevel(UpgradeId::FireRate),
    },
    UpgradeDef {
        id: UpgradeId::CritDamage,
        name: "Crit Damage",
        desc: "+20% crit damage / lvl",
        max_level: 5,
        base_cost: 40,
        cost_growth: 1.7,
        unlock: UnlockCondition::Maxed(UpgradeId::CritChance),
    },
    UpgradeDef {
        id: UpgradeId::BoostTileDiversity,
        name: "Boost Diversity",
        desc: "Adds new boost variants (future runs)",
        max_level: 1,
        base_cost: 55,
        cost_growth: 2.0,
        unlock: UnlockCondition::Maxed(UpgradeId::BoostTileFrequency),
    },
    UpgradeDef {
        id: UpgradeId::GridExpand,
        name: "Grid Expansion",
        desc: "+2 grid size (w & h) / lvl (future runs)",
        max_level: 3,
        base_cost: 60,
        cost_growth: 1.8,
        unlock: UnlockCondition::Maxed(UpgradeId::MiningSpeed),
    },
];

impl UpgradeId {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpgradeState {
    pub levels: Vec<u8>,
    pub tower_refund_rate_percent: u8,
}
impl Default for UpgradeState {
    fn default() -> Self {
        Self {
            levels: vec![0; UPGRADE_DEFS.len()],
            tower_refund_rate_percent: 100,
        }
    }
}
impl UpgradeState {
    pub fn level(&self, id: UpgradeId) -> u8 {
        *self.levels.get(id.index()).unwrap_or(&0)
    }
    pub fn max_level(&self, id: UpgradeId) -> u8 {
        UPGRADE_DEFS[id.index()].max_level
    }
    pub fn is_unlocked(&self, id: UpgradeId) -> bool {
        use UnlockCondition::*;
        match UPGRADE_DEFS[id.index()].unlock {
            Always => true,
            AnyLevel(dep) => self.level(dep) > 0,
            Maxed(dep) => self.level(dep) >= self.max_level(dep),
        }
    }
    pub fn next_cost(&self, id: UpgradeId) -> Option<u64> {
        let def = &UPGRADE_DEFS[id.index()];
        let lvl = self.level(id);
        if lvl >= def.max_level {
            None
        } else {
            Some((def.base_cost as f64 * def.cost_growth.powi(lvl as i32)).round() as u64)
        }
    }
    pub fn can_purchase(&self, id: UpgradeId) -> bool {
        self.is_unlocked(id) && self.level(id) < self.max_level(id)
    }
    pub fn purchase(&mut self, id: UpgradeId) {
        let cur = self.level(id);
        if cur < self.max_level(id) {
            self.levels[id.index()] = cur + 1;
        }
    }
}

pub fn apply_upgrades_to_run(run: &mut RunState, ups: &UpgradeState) {
    use UpgradeId::*;
    let l = |id: UpgradeId| ups.level(id) as f64;
    // Core modifiers
    run.mining_speed = 6.0 * (1.0 + 0.15 * l(MiningSpeed));
    let dmg1 = 0.12 * l(TowerDamage);
    let dmg2 = 0.10 * l(TowerDamage2);
    run.tower_base_damage = (2.0 * (1.0 + dmg1 + dmg2)) as u32;
    if run.tower_base_damage < 1 {
        run.tower_base_damage = 1;
    }
    run.tower_base_range = 3.5 * (1.0 + 0.06 * l(TowerRange));
    run.tower_fire_rate_global = 1.0 + 0.08 * l(FireRate);
    run.crit_chance = 0.02 * l(CritChance); // capped later
    run.crit_damage_mult = 1.0 + 0.20 * l(CritDamage);
    run.damage_ramp_per_sec = 0.03 * l(DamageRamp);
    // Fresh-run dependent values (only if run not started yet)
    if run.stats.time_survived_secs == 0 && !run.started {
        let base_life = 20 + 5 * ups.level(Health) as u32;
        run.life_max = base_life;
        if run.life > run.life_max {
            run.life = run.life_max;
        } else {
            run.life = run.life_max;
        }
        run.currencies.gold = run
            .currencies
            .gold
            .saturating_add(3 * ups.level(StartingGold) as u64);
    }
    run.life_max = 20 + 5 * ups.level(Health) as u32; // keep updated in-run
    if run.life > run.life_max {
        run.life = run.life_max;
    }
    run.life_regen_per_sec = 0.5 * l(LifeRegen);
    run.gold_bounty_mul = 1.0 + 0.04 * l(GoldGain);
    let base_bounty = if ups.level(GoldGain) > 0 { 1 } else { 0 }; // base 1 gold per kill once unlocked any level
    run.gold_bounty_per_kill = (base_bounty as f64 * run.gold_bounty_mul).round() as u64;
    // Recalculate existing towers
    for tw in &mut run.towers {
        let (range_mul_kind, dmg_mul_kind, fire_rate_kind) = match tw.kind {
            TowerKind::Basic => (1.0, 1.0, 1.0),
            TowerKind::Slow => (1.1, 0.5, 0.75),
            TowerKind::Damage => (0.9, 1.5, 0.8),
        };
        tw.range = run.tower_base_range * range_mul_kind;
        tw.damage = ((run.tower_base_damage as f64) * dmg_mul_kind).round() as u32;
        tw.fire_rate = fire_rate_kind * run.tower_fire_rate_global;
    }
}

// ---------------- Reducer & Actions -----------------
#[derive(Clone, Debug)]
pub enum RunAction {
    TogglePause,
    StartRun,
    #[allow(dead_code)]
    TickSecond,
    MiningComplete {
        idx: usize,
    },
    SimTick {
        dt: f64,
    },
    #[allow(dead_code)]
    ResetRun,
    ResetRunWithUpgrades {
        ups: UpgradeState,
    },
    PlaceWall {
        x: u32,
        y: u32,
    },
    PlaceTower {
        x: u32,
        y: u32,
    },
    RemoveTower {
        x: u32,
        y: u32,
    },
    SpendResearch {
        amount: u64,
    },
    ApplyUpgrades {
        ups: UpgradeState,
    },
    SetResearch {
        amount: u64,
    },
}

impl Reducible for RunState {
    type Action = RunAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        use RunAction::*;
        // Reset is special: return brand new state
        if let ResetRunWithUpgrades { ups } = &action {
            let prev_research = self.currencies.research;
            let prev_run_id = self.run_id;
            let mut fresh = RunState::new_with_upgrades(self.grid_size, ups);
            fresh.currencies.research = prev_research;
            fresh.run_id = prev_run_id + 1;
            return Rc::new(fresh);
        }
        if let ResetRun = action {
            let prev_research = self.currencies.research;
            let prev_run_id = self.run_id;
            let mut fresh = RunState::new_basic(self.grid_size);
            fresh.currencies.research = prev_research;
            fresh.run_id = prev_run_id + 1;
            return Rc::new(fresh);
        }
        let mut new = (*self).clone();
        match action {
            ResetRun => unreachable!(),
            RunAction::ResetRunWithUpgrades { .. } => unreachable!(),
            TogglePause => {
                if !new.game_over {
                    new.is_paused = !new.is_paused;
                }
            }
            StartRun => {
                if !new.started && !new.game_over {
                    new.started = true;
                }
            }
            TickSecond => {
                if new.started && !new.is_paused && !new.game_over {
                    new.stats.time_survived_secs = new.stats.time_survived_secs.saturating_add(1); // life regen
                    if new.life < new.life_max && new.life_regen_per_sec > 0.0 {
                        new.life_regen_accum += new.life_regen_per_sec;
                        if new.life_regen_accum >= 1.0 {
                            let gain = new.life_regen_accum.floor() as u32;
                            new.life_regen_accum -= gain as f64;
                            new.life = (new.life + gain).min(new.life_max);
                        }
                    }
                }
            }
            SpendResearch { amount } => {
                if new.currencies.research >= amount {
                    new.currencies.research -= amount;
                }
            }
            ApplyUpgrades { ups } => {
                apply_upgrades_to_run(&mut new, &ups);
            }
            SetResearch { amount } => {
                new.currencies.research = amount;
            }
            MiningComplete { idx } => {
                if new.game_over {
                } else if idx < new.tiles.len() {
                    new.last_mined_idx = Some(idx);
                    dlog(&format!(
                        "MiningComplete reducer idx={} kind_before={:?}",
                        idx, new.tiles[idx].kind
                    ));
                    match new.tiles[idx].kind.clone() {
                        TileKind::Rock { has_gold, .. } => {
                            new.tiles[idx].kind = TileKind::Empty;
                            new.tiles[idx].hardness = 1; // mined tiles become soft empty
                            if new.stats.blocks_mined < u32::MAX {
                                new.stats.blocks_mined += 1;
                            }
                            new.currencies.tile_credits =
                                new.currencies.tile_credits.saturating_add(1);
                            if has_gold {
                                new.currencies.gold = new.currencies.gold.saturating_add(1);
                            }
                            // proceed with updates
                            new.path = compute_path(&new);
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
                            let nodes = new.path_loop.clone();
                            let cum = new.loop_cum_lengths.clone();
                            let total = new.loop_total_length;
                            for e in &mut new.enemies {
                                let (d, dx, dy, next_i) = project_distance_on_loop_snapshot(
                                    &nodes, &cum, total, e.x, e.y,
                                );
                                e.loop_dist = d;
                                e.dir_dx = dx;
                                e.dir_dy = dy;
                                e.path_index = next_i;
                            }
                            if !new.path.is_empty() {
                                let last_index = new.path.len().saturating_sub(1);
                                for e in &mut new.enemies {
                                    if e.path_index > last_index {
                                        e.path_index = last_index;
                                    }
                                }
                            } else {
                                for e in &mut new.enemies {
                                    e.path_index = 0;
                                }
                            }
                        }
                        TileKind::Wall => {
                            // allow removing walls by mining too
                            new.tiles[idx].kind = TileKind::Empty;
                            new.tiles[idx].hardness = 1; // mined tiles become soft empty
                            if new.stats.blocks_mined < u32::MAX {
                                new.stats.blocks_mined += 1;
                            }
                            new.currencies.tile_credits =
                                new.currencies.tile_credits.saturating_add(1);
                            // no gold from walls
                            new.path = compute_path(&new);
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
                            let nodes = new.path_loop.clone();
                            let cum = new.loop_cum_lengths.clone();
                            let total = new.loop_total_length;
                            for e in &mut new.enemies {
                                let (d, dx, dy, next_i) = project_distance_on_loop_snapshot(
                                    &nodes, &cum, total, e.x, e.y,
                                );
                                e.loop_dist = d;
                                e.dir_dx = dx;
                                e.dir_dy = dy;
                                e.path_index = next_i;
                            }
                            if !new.path.is_empty() {
                                let last_index = new.path.len().saturating_sub(1);
                                for e in &mut new.enemies {
                                    if e.path_index > last_index {
                                        e.path_index = last_index;
                                    }
                                }
                            } else {
                                for e in &mut new.enemies {
                                    e.path_index = 0;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            SimTick { dt } => {
                if !(new.started && !new.is_paused && !new.game_over) {
                    return self;
                }
                new.sim_time += dt;
                // TOWER FIRING -> spawn projectiles (damage resolved on impact)
                if !new.towers.is_empty() && !new.enemies.is_empty() {
                    for tw in &mut new.towers {
                        if tw.cooldown_remaining > 0.0 {
                            tw.cooldown_remaining -= dt;
                        }
                        if tw.cooldown_remaining > 0.0 {
                            continue;
                        }
                        // acquire first enemy in range
                        let cx = tw.x as f64 + 0.5;
                        let cy = tw.y as f64 + 0.5;
                        let mut target_index: Option<usize> = None;
                        let mut target_pos: Option<(f64, f64)> = None;
                        for (ei, e) in new.enemies.iter().enumerate() {
                            let dx = e.x - cx;
                            let dy = e.y - cy;
                            if dx * dx + dy * dy <= tw.range * tw.range {
                                target_index = Some(ei);
                                target_pos = Some((e.x, e.y));
                                break;
                            }
                        }
                        if let (Some((tx, ty)), Some(ei)) = (target_pos, target_index) {
                            let dx = tx - cx;
                            let dy = ty - cy;
                            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
                            let speed = 8.0; // tiles per second
                            let travel_time = dist / speed;
                            let mut dmg = tw.damage as f64;
                            // damage ramp based on enemy age
                            let enemy_age = if let Some(e) = new.enemies.get(ei) {
                                (new.stats.time_survived_secs.saturating_sub(e.spawned_at)) as f64
                            } else {
                                0.0
                            };
                            if new.damage_ramp_per_sec > 0.0 {
                                dmg *= 1.0 + new.damage_ramp_per_sec * enemy_age;
                            }
                            // crit
                            if new.crit_chance > 0.0 && js_sys::Math::random() < new.crit_chance {
                                dmg *= new.crit_damage_mult;
                            }
                            if dmg < 1.0 {
                                dmg = 1.0;
                            }
                            new.projectiles.push(Projectile {
                                x: cx,
                                y: cy,
                                vx: dx / dist * speed,
                                vy: dy / dist * speed,
                                remaining: travel_time,
                                damage: dmg.round() as u32,
                            });
                            tw.cooldown_remaining =
                                1.0 / (tw.fire_rate * new.tower_fire_rate_global.max(0.01));
                        }
                    }
                }
                // UPDATE PROJECTILES & APPLY DAMAGE ON IMPACT
                if !new.projectiles.is_empty() {
                    let mut kills = 0u64; // count kills for research
                    let mut i = 0;
                    while i < new.projectiles.len() {
                        let mut remove = false;
                        {
                            let p = &mut new.projectiles[i];
                            p.x += p.vx * dt;
                            p.y += p.vy * dt;
                            p.remaining -= dt;
                            if p.remaining <= 0.0 {
                                // impact position
                                let ix = p.x;
                                let iy = p.y;
                                // find nearest enemy within hit radius
                                let mut hit_index: Option<usize> = None;
                                let mut best_d2 = 0.3_f64 * 0.3_f64;
                                for (ei, e) in new.enemies.iter().enumerate() {
                                    let dx = e.x - ix;
                                    let dy = e.y - iy;
                                    let d2 = dx * dx + dy * dy;
                                    if d2 <= best_d2 {
                                        best_d2 = d2;
                                        hit_index = Some(ei);
                                    }
                                }
                                if let Some(ei) = hit_index {
                                    if let Some(e) = new.enemies.get_mut(ei) {
                                        if p.damage >= e.hp {
                                            e.hp = 0;
                                        } else {
                                            e.hp -= p.damage;
                                        }
                                    }
                                }
                                remove = true; // projectile consumed
                            }
                        }
                        if remove {
                            new.projectiles.remove(i);
                        } else {
                            i += 1;
                        }
                    }
                    // cull dead enemies after projectile impacts
                    if !new.enemies.is_empty() {
                        new.enemies.retain(|e| {
                            if e.hp == 0 {
                                kills = kills.saturating_add(1);
                                false
                            } else {
                                true
                            }
                        });
                        if kills > 0 {
                            new.currencies.research = new.currencies.research.saturating_add(kills); // research per kill
                            if new.gold_bounty_per_kill > 0 {
                                new.currencies.gold = new
                                    .currencies
                                    .gold
                                    .saturating_add(kills * new.gold_bounty_per_kill);
                            }
                        }
                    }
                }
                // Maintain path & enemies movement (unchanged below)
                if new.path.is_empty() {
                    new.path = compute_path(&new);
                }
                let old_nodes = new.path_loop.clone();
                let recomputed = build_loop_path(&new);
                if recomputed != old_nodes {
                    new.path_loop = recomputed;
                    update_loop_geometry(&mut new);
                    let nodes = new.path_loop.clone();
                    let cum = new.loop_cum_lengths.clone();
                    let total = new.loop_total_length;
                    for e in &mut new.enemies {
                        let (d, dx, dy, next_i) =
                            project_distance_on_loop_snapshot(&nodes, &cum, total, e.x, e.y);
                        e.loop_dist = d;
                        e.dir_dx = dx;
                        e.dir_dy = dy;
                        e.path_index = next_i;
                    }
                }
                let total_len = new.loop_total_length;
                if total_len > 1e-6 && dt > 0.0 {
                    let nodes = new.path_loop.clone();
                    let cum = new.loop_cum_lengths.clone();
                    let n = nodes.len();
                    for e in &mut new.enemies {
                        let prev_dist = e.loop_dist;
                        e.loop_dist = (e.loop_dist + e.speed_tps * dt) % total_len;
                        if e.loop_dist < prev_dist {
                            // completed a loop
                            if new.life > 0 {
                                new.life = new.life.saturating_sub(1);
                                if new.stats.loops_completed < u32::MAX {
                                    new.stats.loops_completed += 1;
                                }
                            }
                            if new.life == 0 {
                                break;
                            }
                        }
                        if n < 2 {
                            continue;
                        }
                        let seg_index = if e.loop_dist < cum[n - 1] {
                            match cum
                                .binary_search_by(|probe| probe.partial_cmp(&e.loop_dist).unwrap())
                            {
                                Ok(pos) => {
                                    if pos == 0 {
                                        0
                                    } else {
                                        pos - 1
                                    }
                                }
                                Err(pos) => {
                                    if pos == 0 {
                                        0
                                    } else {
                                        pos - 1
                                    }
                                }
                            }
                        } else {
                            n - 1
                        };
                        let a = nodes[seg_index];
                        let b = nodes[(seg_index + 1) % n];
                        let ax = a.x as f64 + 0.5;
                        let ay = a.y as f64 + 0.5;
                        let bx = b.x as f64 + 0.5;
                        let by = b.y as f64 + 0.5;
                        let seg_dx = bx - ax;
                        let seg_dy = by - ay;
                        let seg_len = if seg_index == n - 1 {
                            let dx = bx - ax;
                            let dy = by - ay;
                            (dx * dx + dy * dy).sqrt()
                        } else {
                            ((b.x as f64 - a.x as f64).powi(2) + (b.y as f64 - a.y as f64).powi(2))
                                .sqrt()
                        }
                        .max(1e-9);
                        let base = if seg_index == n - 1 {
                            cum[n - 1]
                        } else {
                            cum[seg_index]
                        };
                        let local = e.loop_dist - base;
                        let t = (local / seg_len).clamp(0.0, 1.0);
                        e.x = ax + seg_dx * t;
                        e.y = ay + seg_dy * t;
                        e.dir_dx = seg_dx / seg_len;
                        e.dir_dy = seg_dy / seg_len;
                        e.path_index = (seg_index + 1) % n;
                    }
                }
                // Enemy spawning
                if new.life > 0 {
                    let t = new.stats.time_survived_secs;
                    let need_spawn = new.path_loop.len() >= 2
                        && (new.enemies.is_empty()
                            || t.saturating_sub(new.last_enemy_spawn_time_secs) >= 2);
                    if need_spawn {
                        if let (Some(a), Some(b)) = (new.path_loop.get(0), new.path_loop.get(1)) {
                            let mut dx = b.x as f64 - a.x as f64;
                            let mut dy = b.y as f64 - a.y as f64;
                            let mag = (dx * dx + dy * dy).sqrt();
                            if mag > 1e-6 {
                                dx /= mag;
                                dy /= mag;
                            }
                            let speed = 1.0 + 0.002 * (t as f64);
                            let hp = 5 + (t / 10) as u32;
                            let rscale = 0.85 + js_sys::Math::random() * 0.3;
                            new.enemies.push(Enemy {
                                x: a.x as f64 + 0.5,
                                y: a.y as f64 + 0.5,
                                speed_tps: speed,
                                hp,
                                spawned_at: t,
                                path_index: 1,
                                dir_dx: dx,
                                dir_dy: dy,
                                radius_scale: rscale,
                                loop_dist: 0.0,
                            });
                            new.last_enemy_spawn_time_secs = t;
                        }
                    }
                }
                if new.life == 0 && !new.game_over {
                    new.game_over = true;
                    new.is_paused = true;
                }
            }
            PlaceWall { x, y } => {
                if new.game_over { /* ignore */
                } else if x < new.grid_size.width && y < new.grid_size.height {
                    let idx = (y * new.grid_size.width + x) as usize;
                    if let TileKind::Empty = new.tiles[idx].kind {
                        new.tiles[idx].kind = TileKind::Rock {
                            has_gold: false,
                            boost: None,
                        };
                        new.tiles[idx].hardness = 3;
                        let test_path = compute_path(&new);
                        if test_path.is_empty() {
                            // revert to keep path
                            new.tiles[idx].kind = TileKind::Empty;
                            new.tiles[idx].hardness = 1;
                        } else {
                            new.path = test_path;
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
                            let nodes = new.path_loop.clone();
                            let cum = new.loop_cum_lengths.clone();
                            let total = new.loop_total_length;
                            for e in &mut new.enemies {
                                let (d, dx, dy, next_i) = project_distance_on_loop_snapshot(
                                    &nodes, &cum, total, e.x, e.y,
                                );
                                e.loop_dist = d;
                                e.dir_dx = dx;
                                e.dir_dy = dy;
                                e.path_index = next_i;
                            }
                        }
                    }
                }
            }
            PlaceTower { x, y } => {
                if new.game_over { /* ignore */
                } else if x < new.grid_size.width && y < new.grid_size.height {
                    let idx = (y * new.grid_size.width + x) as usize;
                    if !new.towers.iter().any(|t| t.x == x && t.y == y) {
                        if let TileKind::Rock { boost, .. } = new.tiles[idx].kind.clone() {
                            if new.currencies.gold >= new.tower_cost {
                                let kind = match boost {
                                    Some(BoostKind::Slow) => TowerKind::Slow,
                                    Some(BoostKind::Damage) => TowerKind::Damage,
                                    _ => TowerKind::Basic,
                                };
                                new.currencies.gold =
                                    new.currencies.gold.saturating_sub(new.tower_cost);
                                new.towers.push(Tower::new(
                                    x,
                                    y,
                                    kind,
                                    new.tower_base_range,
                                    new.tower_base_damage,
                                ));
                            }
                        }
                    }
                }
            }
            RemoveTower { x, y } => {
                if new.game_over { /* ignore */
                } else {
                    let before = new.towers.len();
                    new.towers.retain(|t| !(t.x == x && t.y == y));
                    if new.towers.len() < before {
                        new.currencies.gold = new.currencies.gold.saturating_add(new.tower_cost);
                    }
                }
            }
        }
        new.version = new.version.saturating_add(1);
        Rc::new(new)
    }
}

// Helper for reprojecting enemies after path changes
fn project_distance_on_loop_snapshot(
    nodes: &Vec<Position>,
    cum: &Vec<f64>,
    total: f64,
    x: f64,
    y: f64,
) -> (f64, f64, f64, usize) {
    let n = nodes.len();
    if n < 2 || total <= 1e-9 {
        return (0.0, 1.0, 0.0, 0);
    }
    let mut best_d2 = f64::MAX;
    let mut best = (0.0, 1.0, 0.0, 1usize);
    for i in 0..n {
        let a = nodes[i];
        let b = nodes[(i + 1) % n];
        let ax = a.x as f64 + 0.5;
        let ay = a.y as f64 + 0.5;
        let bx = b.x as f64 + 0.5;
        let by = b.y as f64 + 0.5;
        let dx = bx - ax;
        let dy = by - ay;
        let seg_len2 = dx * dx + dy * dy;
        if seg_len2 < 1e-12 {
            continue;
        }
        let t = ((x - ax) * dx + (y - ay) * dy) / seg_len2;
        let tc = t.clamp(0.0, 1.0);
        let px = ax + dx * tc;
        let py = ay + dy * tc;
        let d2 = (px - x) * (px - x) + (py - y) * (py - y);
        if d2 < best_d2 {
            best_d2 = d2;
            let seg_len = seg_len2.sqrt();
            let mut loop_dist = (if i == 0 { 0.0 } else { cum[i] }) + seg_len * tc;
            if i == n - 1 {
                let closing_start = total - seg_len;
                loop_dist = closing_start + seg_len * tc;
            }
            let mag = seg_len.max(1e-6);
            best = (loop_dist % total, dx / mag, dy / mag, (i + 1) % n);
        }
    }
    best
}
