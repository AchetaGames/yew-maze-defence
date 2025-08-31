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
    pub loop_cum_lengths: Vec<f64>, // cumulative lengths per node (same length as path_loop)
    pub loop_total_length: f64,
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
            // Increase mining speed for faster testing (was 1.0)
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
        };
        rs.path = compute_path(&rs);
        rs.path_loop = build_loop_path(&rs);
        update_loop_geometry(&mut rs);
        rs
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

// ---------------- Reducer & Actions -----------------
#[derive(Clone, Debug)]
pub enum RunAction {
    TogglePause,
    StartRun,
    TickSecond, // called once per elapsed real second
    MiningComplete { idx: usize },
    SimTick { dt: f64 },          // ~16ms; advances enemies & spawns
    ResetRun,                     // new
    PlaceWall { x: u32, y: u32 }, // new wall placement
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
                        update_loop_geometry(&mut new);
                        // snapshot geometry for projection to avoid borrow conflict
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
                if !(new.started && !new.is_paused && !new.game_over) {
                    return self;
                }
                new.sim_time += dt;
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
                    // snapshot geometry for movement
                    let nodes = new.path_loop.clone();
                    let cum = new.loop_cum_lengths.clone();
                    let n = nodes.len();
                    for e in &mut new.enemies {
                        let prev_dist = e.loop_dist;
                        e.loop_dist = (e.loop_dist + e.speed_tps * dt) % total_len;
                        if e.loop_dist < prev_dist {
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
                        // find segment index
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
                        }; // closing segment
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
                // Spawn logic
                if new.life > 0 {
                    let t = new.stats.time_survived_secs;
                    let need_spawn = new.path_loop.len() >= 2
                        && (new.enemies.is_empty()
                            || t.saturating_sub(new.last_enemy_spawn_time_secs) >= 2);
                    if need_spawn {
                        if let Some(first) = new.path_loop.get(0) {
                            if let Some(second) = new.path_loop.get(1) {
                                let mut dx = second.x as f64 - first.x as f64;
                                let mut dy = second.y as f64 - first.y as f64;
                                let mag = (dx * dx + dy * dy).sqrt();
                                if mag > 1e-6 {
                                    dx /= mag;
                                    dy /= mag;
                                }
                                let speed = 1.0 + 0.002 * (t as f64);
                                let hp = 5 + (t / 10) as u32;
                                let rscale = 0.85 + js_sys::Math::random() * 0.3;
                                new.enemies.push(Enemy {
                                    x: first.x as f64 + 0.5,
                                    y: first.y as f64 + 0.5,
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
                }
                if new.life == 0 && !new.game_over {
                    new.game_over = true;
                    new.is_paused = true;
                }
            }
            PlaceWall { x, y } => {
                if new.game_over { /* ignore */
                } else {
                    if x < new.grid_size.width && y < new.grid_size.height {
                        let idx = (y * new.grid_size.width + x) as usize;
                        match new.tiles[idx].kind {
                            TileKind::Empty => {
                                // Tentatively place a plain rock (no gold, no boost) so it can be mined again.
                                new.tiles[idx].kind = TileKind::Rock {
                                    has_gold: false,
                                    boost: None,
                                };
                                new.tiles[idx].hardness = 3; // default hardness
                                let test_path = compute_path(&new);
                                if test_path.is_empty() {
                                    // Revert if path blocked
                                    new.tiles[idx].kind = TileKind::Empty;
                                    new.tiles[idx].hardness = 1; // lighter hardness for empty (not mined)
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
                            _ => { /* not allowed */ }
                        }
                    }
                }
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
