//! Core data models for Maze Defence.
//! This module defines the initial types aligning with the GDD.
//! TODOs are included to guide future implementation.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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
    // visual variation
    pub wobble_phase: f64,
    pub wobble_amp: f64,
    pub radius_scale: f64,
    pub dir_dx: f64,
    pub dir_dy: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunState {
    pub grid_size: GridSize,
    /// Row-major tiles; length = width * height.
    pub tiles: Vec<Tile>,
    pub currencies: Currencies,
    pub stats: RunStats,
    pub life: u32,
    pub mining_speed: f64,
    pub started: bool,
    pub is_paused: bool,
    pub path: Vec<Position>,
    pub path_loop: Vec<Position>, // cyclic path including start/entrance/exit
    pub enemies: Vec<Enemy>,
    pub last_enemy_spawn_time_secs: u64,
    pub version: u64,
    pub game_over: bool,
    pub last_mined_idx: Option<usize>,
    pub sim_time: f64,
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
            if max_abs <= 0 {
                0
            } else {
                ((js_sys::Math::random() * ((max_abs * 2 + 1) as f64)).floor() as i32) - max_abs
            }
        };
        let mut sx = cx0 + rand_range(half_w);
        let mut sy = cy0 + rand_range(half_h);
        // Clamp within margins
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
        // Exit: one tile opposite, arrow pointing towards start => same ent_dir from that tile toward start
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
        set_empty_if_rock(&mut tiles, grid_size, sx + 2 * dx1, sy + 2 * dy1);
        // Step 2: turn perpendicular (left or right randomly) for a short corridor
        let sign: i32 = if js_sys::Math::random() < 0.5 { 1 } else { -1 };
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
        // Step 3: go back past Start towards Exit side
        for k in 1..=4 {
            set_empty_if_rock(
                &mut tiles,
                grid_size,
                sx + 2 * dx1 + 3 * px - k * dx1,
                sy + 2 * dy1 + 3 * py - k * dy1,
            );
        }
        // Step 4: return perpendicular towards the Exit connector
        for k in 1..=3 {
            set_empty_if_rock(
                &mut tiles,
                grid_size,
                sx - 2 * dx1 + (3 - k) * px,
                sy - 2 * dy1 + (3 - k) * py,
            );
        }
        // Final: ensure a direct neighbor to the Exit tile is open
        set_empty_if_rock(&mut tiles, grid_size, sx - 2 * dx1, sy - 2 * dy1);

        let mut rs = Self {
            grid_size,
            tiles,
            currencies: Currencies::default(),
            stats: RunStats::default(),
            life: 200,
            mining_speed: 1.0,
            started: false,
            is_paused: false,
            path: Vec::new(),
            path_loop: Vec::new(),
            enemies: Vec::new(),
            last_enemy_spawn_time_secs: 0,
            version: 0,
            game_over: false,
            last_mined_idx: None,
            sim_time: 0.0,
        };
        rs.path = compute_path(&rs);
        rs.path_loop = build_loop_path(&rs);
        rs
    }
}

// ---------------- Pathfinding helpers -----------------
fn dir_to_delta(dir: ArrowDir) -> (i32, i32) {
    match dir {
        ArrowDir::Up => (0, -1),
        ArrowDir::Down => (0, 1),
        ArrowDir::Left => (-1, 0),
        ArrowDir::Right => (1, 0),
    }
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

pub fn compute_path(rs: &RunState) -> Vec<Position> {
    let gs = rs.grid_size;
    let Some(((ex, ey, edir), (xx, xy, xdir))) = find_entrance_exit(rs) else {
        return Vec::new();
    };
    let (edx, edy) = dir_to_delta(edir);
    let (xdx, xdy) = dir_to_delta(xdir);
    let sx = ex + edx; // start from cell beyond entrance arrow
    let sy = ey + edy;
    let tx = xx - xdx; // end at cell before exit arrow (neighbor Empty ensured)
    let ty = xy - xdy;
    let in_bounds =
        |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    if !in_bounds(sx, sy) || !in_bounds(tx, ty) {
        return Vec::new();
    }
    let sidx = (sy as u32 * gs.width + sx as u32) as usize;
    let tidx = (ty as u32 * gs.width + tx as u32) as usize;
    let is_empty = |idx: usize| matches!(rs.tiles[idx].kind, TileKind::Empty);
    if !is_empty(sidx) || !is_empty(tidx) {
        return Vec::new();
    }
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut visited = vec![false; (gs.width * gs.height) as usize];
    let mut parent: Vec<Option<usize>> = vec![None; (gs.width * gs.height) as usize];
    visited[sidx] = true;
    q.push_back(sidx);
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    while let Some(idx) = q.pop_front() {
        if idx == tidx {
            break;
        }
        let x = (idx as u32 % gs.width) as i32;
        let y = (idx as u32 / gs.width) as i32;
        for (dx, dy) in dirs {
            let nx = x + dx;
            let ny = y + dy;
            if !in_bounds(nx, ny) {
                continue;
            }
            let nidx = (ny as u32 * gs.width + nx as u32) as usize;
            if visited[nidx] {
                continue;
            }
            if !is_empty(nidx) {
                continue;
            }
            visited[nidx] = true;
            parent[nidx] = Some(idx);
            q.push_back(nidx);
        }
    }
    if !visited[tidx] {
        return Vec::new();
    }
    let mut path_rev: Vec<usize> = Vec::new();
    let mut cur = Some(tidx);
    while let Some(ci) = cur {
        path_rev.push(ci);
        cur = parent[ci];
    }
    path_rev.reverse();
    path_rev
        .into_iter()
        .map(|i| {
            let x = (i as u32 % gs.width) as u32;
            let y = (i as u32 / gs.width) as u32;
            Position { x, y }
        })
        .collect()
}

fn build_loop_path(rs: &RunState) -> Vec<Position> {
    // New ordering: Start -> Entrance direction tile -> linear path nodes -> Exit direction tile (cyclic wrap back to Start)
    // rs.path still represents linear empty cells beyond entrance arrow through before exit arrow.
    let mut start_pos: Option<Position> = None;
    let mut entrance_pos: Option<Position> = None;
    let mut exit_pos: Option<Position> = None;
    for y in 0..rs.grid_size.height { for x in 0..rs.grid_size.width { let idx=(y*rs.grid_size.width + x) as usize; match rs.tiles[idx].kind { TileKind::Start => start_pos = Some(Position{x,y}), TileKind::Direction{dir:_, role} => { match role { DirRole::Entrance => entrance_pos = Some(Position{x,y}), DirRole::Exit => exit_pos = Some(Position{x,y}), } }, _=>{} } } }
    let (Some(start), Some(ent), Some(exit)) = (start_pos, entrance_pos, exit_pos) else { return rs.path.clone(); };
    let mut loop_vec = Vec::new();
    loop_vec.push(start);
    if ent.x!=start.x || ent.y!=start.y { loop_vec.push(ent); }
    // append linear path nodes (already excludes direction tiles)
    for p in &rs.path { loop_vec.push(*p); }
    if exit.x!=loop_vec.last().map(|p| (p.x,p.y)).unwrap_or((u32::MAX,u32::MAX)).0 || exit.y!=loop_vec.last().unwrap().y { loop_vec.push(exit); }
    loop_vec
}

// ---------------- Reducer & Actions -----------------
#[derive(Clone, Debug)]
pub enum RunAction {
    TogglePause,
    StartRun,
    TickSecond, // called once per elapsed real second
    MiningComplete { idx: usize },
    SimTick { dt: f64 }, // ~16ms; advances enemies & spawns
    ResetRun,            // new
}

impl Reducible for RunState {
    type Action = RunAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        use RunAction::*;
        // Reset is special: return brand new state
        if let ResetRun = action {
            return Rc::new(RunState::new_basic(self.grid_size));
        }
        let mut new = (*self).clone();
        match action {
            ResetRun => unreachable!(),
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
                    new.stats.time_survived_secs = new.stats.time_survived_secs.saturating_add(1);
                }
            }
            MiningComplete { idx } => {
                if new.game_over {
                } else if idx < new.tiles.len() {
                    if let TileKind::Rock { has_gold, .. } = new.tiles[idx].kind.clone() {
                        new.tiles[idx].kind = TileKind::Empty;
                        if new.stats.blocks_mined < u32::MAX {
                            new.stats.blocks_mined += 1;
                        }
                        new.currencies.tile_credits = new.currencies.tile_credits.saturating_add(1);
                        if has_gold {
                            new.currencies.gold = new.currencies.gold.saturating_add(1);
                        }
                        new.path = compute_path(&new);
                        new.path_loop = build_loop_path(&new);
                        adjust_enemies_after_path_change(&mut new);
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
                }
            }
            SimTick { dt } => {
                if !(new.started && !new.is_paused && !new.game_over) { return self; }
                new.sim_time += dt;
                if new.path.is_empty() { new.path = compute_path(&new); }
                if new.path_loop.len() < 2 { new.path_loop = build_loop_path(&new); }
                // Move enemies along cyclic path_loop
                if !new.path_loop.is_empty() && dt > 0.0 && !new.enemies.is_empty() {
                    let loop_nodes = new.path_loop.clone();
                    let len = loop_nodes.len();
                    for e in &mut new.enemies {
                        let old_x = e.x; let old_y = e.y; // store start pos for direction delta
                        if len==0 { break; }
                        if e.path_index >= len { e.path_index %= len; }
                        let mut remaining = dt * e.speed_tps;
                        while remaining > 0.0 && new.life > 0 {
                            let target = loop_nodes[e.path_index];
                            let tx = target.x as f64 + 0.5; let ty = target.y as f64 + 0.5;
                            let dx = tx - e.x; let dy = ty - e.y; let dist = (dx*dx + dy*dy).sqrt();
                            if dist < 1e-6 {
                                e.path_index += 1;
                                if e.path_index >= len { e.path_index = 0; if new.life>0 { new.life -=1; } if new.stats.loops_completed < u32::MAX { new.stats.loops_completed +=1; } if new.life==0 { break; } }
                                continue;
                            }
                            if remaining >= dist { e.x = tx; e.y = ty; remaining -= dist; }
                            else { let ratio = remaining / dist; e.x += dx*ratio; e.y += dy*ratio; remaining = 0.0; }
                        }
                        // smooth direction based on displacement
                        let raw_dx = e.x - old_x; let raw_dy = e.y - old_y; let raw_mag = (raw_dx*raw_dx + raw_dy*raw_dy).sqrt();
                        if raw_mag > 1e-5 { let ndx = raw_dx/raw_mag; let ndy = raw_dy/raw_mag; // blend
                            let blend = 0.25; e.dir_dx = (1.0-blend)*e.dir_dx + blend*ndx; e.dir_dy = (1.0-blend)*e.dir_dy + blend*ndy; // renormalize
                            let nmag = (e.dir_dx*e.dir_dx + e.dir_dy*e.dir_dy).sqrt(); if nmag>1e-6 { e.dir_dx/=nmag; e.dir_dy/=nmag; }
                        }
                    }
                }
                // Spawn logic uses first node of loop if available
                let t = new.stats.time_survived_secs;
                let path_available = if !new.path_loop.is_empty() { new.path_loop.len() } else { new.path.len() };
                let need_spawn = path_available >= 1 && (new.enemies.is_empty() || t.saturating_sub(new.last_enemy_spawn_time_secs) >= 2);
                if need_spawn {
                    let speed = 1.0 + 0.002 * (t as f64);
                    let hp = 5 + (t / 10) as u32;
                    let phase = js_sys::Math::random() * std::f64::consts::TAU;
                    let amp = 0.08 + js_sys::Math::random()*0.06; // 0.08..0.14 tiles
                    let rscale = 0.85 + js_sys::Math::random()*0.3; // 0.85..1.15
                    let (init_dx, init_dy) = if new.path_loop.len() > 1 { let a=new.path_loop[0]; let b=new.path_loop[1]; let mut dx = b.x as f64 - a.x as f64; let mut dy = b.y as f64 - a.y as f64; let mag=(dx*dx+dy*dy).sqrt(); if mag>1e-6 { dx/=mag; dy/=mag; } (dx,dy) } else { (1.0,0.0) };
                    // Spawn at Start tile center (path_loop[0]) and target next waypoint (index 1 if exists)
                    let path_index = if new.path_loop.len() > 1 { 1 } else { 0 };
                    new.enemies.push(Enemy { x: new.path_loop[0].x as f64 + 0.5, y: new.path_loop[0].y as f64 + 0.5, speed_tps: speed, hp, spawned_at: t, path_index, wobble_phase: phase, wobble_amp: amp, radius_scale: rscale, dir_dx: init_dx, dir_dy: init_dy });
                    new.last_enemy_spawn_time_secs = t;
                }
                if new.life == 0 && !new.game_over { new.game_over = true; new.is_paused = true; }
            }
        }
        new.version = new.version.saturating_add(1);
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
fn adjust_enemies_after_path_change(rs: &mut RunState) {
    if rs.enemies.is_empty() { return; }
    let nodes = if !rs.path_loop.is_empty() { &rs.path_loop } else { &rs.path };
    let len = nodes.len();
    if len == 0 { return; }
    let gs = rs.grid_size;
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    let is_walkable = |tile: &TileKind| matches!(tile, TileKind::Empty | TileKind::Start | TileKind::Direction{..});
    // Precompute a map from (x,y) -> node index for O(1) target detection
    let mut node_map = vec![usize::MAX; (gs.width * gs.height) as usize];
    for (i,n) in nodes.iter().enumerate() { let idx = (n.y * gs.width + n.x) as usize; node_map[idx] = i; }
    for e in &mut rs.enemies {
        let prev_node = if e.path_index == 0 { len.saturating_sub(1) } else { e.path_index - 1 };
        let ex_tile = e.x.floor() as i32; let ey_tile = e.y.floor() as i32;
        if !in_bounds(ex_tile, ey_tile) { continue; }
        let start_idx = (ey_tile as u32 * gs.width + ex_tile as u32) as usize;
        use std::collections::VecDeque;
        let mut q = VecDeque::new();
        let mut visited = vec![false; (gs.width * gs.height) as usize];
        q.push_back(start_idx); visited[start_idx]=true;
        let mut found_node: Option<usize> = None;
        while let Some(idx) = q.pop_front() {
            let node_id = node_map[idx];
            if node_id != usize::MAX { found_node = Some(node_id); break; }
            let x = (idx as u32 % gs.width) as i32; let y = (idx as u32 / gs.width) as i32;
            for (dx,dy) in [(1,0),(-1,0),(0,1),(0,-1)] { let nx=x+dx; let ny=y+dy; if !in_bounds(nx,ny){continue;} let nidx=(ny as u32 * gs.width + nx as u32) as usize; if visited[nidx]{continue;} if !is_walkable(&rs.tiles[nidx].kind){continue;} visited[nidx]=true; q.push_back(nidx);}        }
        if let Some(mut node_i) = found_node {
            // compute forward distance from prev_node to candidate (cyclic)
            let forward_dist = if node_i >= prev_node { node_i - prev_node } else { node_i + len - prev_node };
            let backward_dist = if prev_node >= node_i { prev_node - node_i } else { prev_node + len - node_i };
            // If candidate is strictly behind and not a simple wrap (forward_dist > backward_dist), keep previous node
            if backward_dist < forward_dist && backward_dist > 0 { node_i = prev_node; }
            let pos = nodes[node_i];
            e.x = pos.x as f64 + 0.5; e.y = pos.y as f64 + 0.5;
            e.path_index = if len > 1 { (node_i + 1) % len } else { 0 };
        } else {
            let mut best = prev_node; let mut best_d2 = f64::MAX;
            for (i,n) in nodes.iter().enumerate(){ let dx=(n.x as f64+0.5)-e.x; let dy=(n.y as f64+0.5)-e.y; let d2=dx*dx+dy*dy; if d2<best_d2 { best_d2=d2; best=i; } }
            let pos = nodes[best]; e.x = pos.x as f64 + 0.5; e.y = pos.y as f64 + 0.5; e.path_index = if len>1 { (best+1)%len } else {0};
        }
    }
}
