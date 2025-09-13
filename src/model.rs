//! Core data models (reconstructed after upgrade system refactor)
//! This module defines the initial types aligning with the GDD.
//! TODOs are included to guide future implementation.

use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::JsValue;

#[allow(dead_code)]
const DEBUG_LOG: bool = false;
#[allow(dead_code)]
fn dlog(msg: &str) {
    if DEBUG_LOG {
        web_sys::console::log_1(&JsValue::from_str(msg));
    }
}

// -------- Basic structs --------
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
    Empty,
    Rock {
        has_gold: bool,
        boost: Option<BoostKind>,
    },
    Wall,
    Start,
    Direction {
        dir: ArrowDir,
        role: DirRole,
    },
    Indestructible,
    End,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    pub hardness: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Currencies {
    pub gold: u64,
    pub research: u64,
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
    pub path_index: usize,
    pub dir_dx: f64,
    pub dir_dy: f64,
    pub radius_scale: f64,
    pub loop_dist: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DamageNumber {
    pub x: f64,
    pub y: f64,
    pub amount: u32,
    pub ttl: f64,
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
    pub path_loop: Vec<Position>,
    pub loop_cum_lengths: Vec<f64>,
    pub loop_total_length: f64,
    pub enemies: Vec<Enemy>,
    pub last_enemy_spawn_time_secs: f64,
    pub version: u64,
    pub game_over: bool,
    pub last_mined_idx: Option<usize>,
    pub sim_time: f64,
    pub towers: Vec<Tower>,
    pub tower_base_range: f64,
    pub tower_base_damage: u32,
    pub tower_cost: u64,
    pub projectiles: Vec<Projectile>,
    pub run_id: u64,
    pub life_max: u32,
    pub life_regen_per_sec: f64,
    pub life_regen_accum: f64,
    pub tower_fire_rate_global: f64,
    pub crit_chance: f64,
    pub crit_damage_mult: f64,
    pub gold_bounty_per_kill: u64,
    pub gold_bounty_mul: f64,
    pub damage_ramp_per_sec: f64,
    pub damage_numbers: Vec<DamageNumber>,
    pub projectile_speed: f64,
    pub vampiric_heal_percent: f64,
    pub mining_gold_mul: f64,
    pub mining_crit_chance: f64,
    // NEW: track how many levels of StartingGold have already been applied to prevent repeated additive grants
    pub starting_gold_applied_level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TowerKind {
    Basic,
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
    pub fire_rate: f64,
    pub cooldown_remaining: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub remaining: f64,
    pub damage: u32,
    pub splash_radius: f64,
}
impl Tower {
    pub fn new(x: u32, y: u32, kind: TowerKind, base_range: f64, base_damage: u32) -> Self {
        let (r_mul, d_mul, fr) = match kind {
            TowerKind::Basic => (1.0, 1.0, 1.0),
            TowerKind::Slow => (1.1, 0.5, 0.75),
            TowerKind::Damage => (0.9, 1.5, 0.8),
        };
        Self {
            x,
            y,
            kind,
            range: base_range * r_mul,
            damage: (base_damage as f64 * d_mul).round() as u32,
            fire_rate: fr,
            cooldown_remaining: 0.0,
        }
    }
}

impl RunState {
    fn create_run_base(
        gs: GridSize,
        gold_chance: f64,
        boost_kinds: &[BoostKind],
        boost_freq_weight: f64,
    ) -> Self {
        let mut tiles = Vec::with_capacity((gs.width * gs.height) as usize);
        for _y in 0..gs.height {
            for _x in 0..gs.width {
                let r = js_sys::Math::random();
                let has_gold = r < gold_chance;
                let boost = if boost_kinds.is_empty() {
                    None
                } else {
                    let spawn_chance = 0.05 * boost_freq_weight;
                    if js_sys::Math::random() < spawn_chance.min(0.90) {
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
        // carve start cluster centrally with corridor similar to original implementation
        let sx = (gs.width / 2) as i32;
        let sy = (gs.height / 2) as i32; // center
        let orient = (js_sys::Math::random() * 4.0).floor() as i32;
        let (dx1, dy1, adir) = match orient {
            0 => (1, 0, ArrowDir::Right),
            1 => (0, 1, ArrowDir::Down),
            2 => (-1, 0, ArrowDir::Left),
            _ => (0, -1, ArrowDir::Up),
        };
        let set_kind = |tiles: &mut Vec<Tile>, x: i32, y: i32, kind: TileKind| {
            if x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height {
                let idx = (y as u32 * gs.width + x as u32) as usize;
                tiles[idx].kind = kind;
                tiles[idx].hardness = 255;
            }
        };
        let make_empty = |tiles: &mut Vec<Tile>, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height {
                let idx = (y as u32 * gs.width + x as u32) as usize;
                if matches!(tiles[idx].kind, TileKind::Rock { .. } | TileKind::Wall) {
                    tiles[idx].kind = TileKind::Empty;
                    tiles[idx].hardness = 1;
                }
            }
        };
        set_kind(&mut tiles, sx, sy, TileKind::Start);
        set_kind(
            &mut tiles,
            sx + dx1,
            sy + dy1,
            TileKind::Direction {
                dir: adir,
                role: DirRole::Entrance,
            },
        );
        set_kind(
            &mut tiles,
            sx - dx1,
            sy - dy1,
            TileKind::Direction {
                dir: adir,
                role: DirRole::Exit,
            },
        );
        // indestructibles perpendicular to force single corridor start
        match adir {
            ArrowDir::Left | ArrowDir::Right => {
                set_kind(&mut tiles, sx, sy - 1, TileKind::Indestructible);
                set_kind(&mut tiles, sx, sy + 1, TileKind::Indestructible);
            }
            _ => {
                set_kind(&mut tiles, sx - 1, sy, TileKind::Indestructible);
                set_kind(&mut tiles, sx + 1, sy, TileKind::Indestructible);
            }
        }
        // carve short L-shaped corridor outwards from entrance & exit directions
        make_empty(&mut tiles, sx + 2 * dx1, sy + 2 * dy1);
        let sign = if js_sys::Math::random() < 0.5 { 1 } else { -1 };
        let px = -dy1 * sign;
        let py = dx1 * sign;
        for k in 1..=3 {
            make_empty(&mut tiles, sx + 2 * dx1 + k * px, sy + 2 * dy1 + k * py);
        }
        for k in 1..=4 {
            make_empty(
                &mut tiles,
                sx + 2 * dx1 + 3 * px - k * dx1,
                sy + 2 * dy1 + 3 * py - k * dy1,
            );
        }
        for k in 1..=3 {
            make_empty(
                &mut tiles,
                sx - 2 * dx1 + (3 - k) * px,
                sy - 2 * dy1 + (3 - k) * py,
            );
        }
        make_empty(&mut tiles, sx - 2 * dx1, sy - 2 * dy1);
        // build initial state
        let mut rs = RunState {
            grid_size: gs,
            tiles,
            currencies: Currencies {
                gold: 2, // lowered starting gold (was 5)
                ..Default::default()
            },
            stats: RunStats::default(),
            life: 10, // lowered starting life (was 20)
            // Slowed baseline mining speed (was 6.0); higher hardness now takes meaningfully longer
            mining_speed: 1.0,
            started: false,
            is_paused: false,
            path: Vec::new(),
            path_loop: Vec::new(),
            loop_cum_lengths: Vec::new(),
            loop_total_length: 0.0,
            enemies: Vec::new(),
            last_enemy_spawn_time_secs: 0.0,
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
            life_max: 10, // lowered base life max
            life_regen_per_sec: 0.0,
            life_regen_accum: 0.0,
            tower_fire_rate_global: 1.0,
            crit_chance: 0.0,
            crit_damage_mult: 1.0,
            gold_bounty_per_kill: 0,
            gold_bounty_mul: 1.0,
            damage_ramp_per_sec: 0.0,
            damage_numbers: Vec::new(),
            projectile_speed: 8.0,
            vampiric_heal_percent: 0.0,
            mining_gold_mul: 1.0,
            mining_crit_chance: 0.0,
            starting_gold_applied_level: 0,
        };
        rs.path = compute_path(&rs);
        rs.path_loop = build_loop_path(&rs);
        update_loop_geometry(&mut rs);
        rs
    }
    pub fn new_basic(gs: GridSize) -> Self {
        Self::create_run_base(gs, 0.12, &[], 1.0)
    }
    pub fn new_with_upgrades(base: GridSize, ups: &UpgradeState) -> Self {
        let grid = base; // no expansion yet
        let gold_chance = (0.12 + 0.05 * ups.level(UpgradeId::GoldTileChance) as f64).min(0.95);
        let mut boosts: Vec<BoostKind> = Vec::new();
        if ups.level(UpgradeId::BoostColdUnlock) > 0 {
            boosts.push(BoostKind::Slow);
        }
        if ups.level(UpgradeId::BoostPoisonUnlock) > 0 {
            boosts.push(BoostKind::Damage);
        }
        if ups.level(UpgradeId::BoostHealingUnlock) > 0 {
            boosts.push(BoostKind::Range);
        }
        let freq = 1.0
            + 0.05
                * (ups.level(UpgradeId::BoostColdFrequency)
                    + ups.level(UpgradeId::BoostPoisonFrequency)
                    + ups.level(UpgradeId::BoostHealingFrequency)) as f64;
        let mut rs = Self::create_run_base(grid, gold_chance, &boosts, freq);
        apply_upgrades_to_run(&mut rs, ups);
        rs
    }
}

// ---- Pathfinding (A*) ----
fn find_entrance_exit(rs: &RunState) -> Option<((i32, i32, ArrowDir), (i32, i32, ArrowDir))> {
    let mut ent = None;
    let mut exit = None;
    for y in 0..rs.grid_size.height {
        for x in 0..rs.grid_size.width {
            let idx = (y * rs.grid_size.width + x) as usize;
            if let TileKind::Direction { dir, role } = rs.tiles[idx].kind {
                match role {
                    DirRole::Entrance => ent = Some((x as i32, y as i32, dir)),
                    DirRole::Exit => exit = Some((x as i32, y as i32, dir)),
                }
            }
        }
    }
    match (ent, exit) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}
fn a_star(rs: &RunState, start: (i32, i32), goal: (i32, i32)) -> Vec<Position> {
    use std::cmp::Ordering;
    use std::collections::{BinaryHeap, HashMap};
    let (sx, sy) = start;
    let (gx, gy) = goal;
    let gs = rs.grid_size;
    let inb = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    if !inb(sx, sy) || !inb(gx, gy) {
        return vec![];
    }
    let idx = |x: i32, y: i32| (y as u32 * gs.width + x as u32) as usize;
    if !matches!(rs.tiles[idx(sx, sy)].kind, TileKind::Empty)
        || !matches!(rs.tiles[idx(gx, gy)].kind, TileKind::Empty)
    {
        return vec![];
    }
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        f: u32,
        idx: usize,
    }
    impl Ord for Node {
        fn cmp(&self, o: &Self) -> Ordering {
            o.f.cmp(&self.f).then_with(|| self.idx.cmp(&o.idx))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }
    let mut open = BinaryHeap::new();
    let mut g = HashMap::new();
    let mut parent = vec![None; (gs.width * gs.height) as usize];
    let h = |x: i32, y: i32| ((x - gx).abs() + (y - gy).abs()) as u32;
    let sidx = idx(sx, sy);
    let gidx = idx(gx, gy);
    g.insert(sidx, 0u32);
    open.push(Node {
        f: h(sx, sy),
        idx: sidx,
    });
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    while let Some(Node { idx: ci, .. }) = open.pop() {
        if ci == gidx {
            break;
        }
        let cx = (ci as u32 % gs.width) as i32;
        let cy = (ci as u32 / gs.width) as i32;
        let g_here = *g.get(&ci).unwrap();
        for (dx, dy) in dirs {
            let nx = cx + dx;
            let ny = cy + dy;
            if !inb(nx, ny) {
                continue;
            }
            let ni = idx(nx, ny);
            if !matches!(rs.tiles[ni].kind, TileKind::Empty) {
                continue;
            }
            let tentative = g_here + 1;
            if tentative < *g.get(&ni).unwrap_or(&u32::MAX) {
                g.insert(ni, tentative);
                parent[ni] = Some(ci);
                let f = tentative + h(nx, ny);
                open.push(Node { f, idx: ni });
            }
        }
    }
    if parent[gidx].is_none() && sidx != gidx {
        return vec![];
    }
    let mut rev = Vec::new();
    let mut cur = Some(gidx);
    while let Some(i) = cur {
        rev.push(i);
        if i == sidx {
            break;
        }
        cur = parent[i];
    }
    rev.reverse();
    rev.into_iter()
        .map(|i| Position {
            x: (i as u32 % gs.width) as u32,
            y: (i as u32 / gs.width) as u32,
        })
        .collect()
}
pub fn compute_path(rs: &RunState) -> Vec<Position> {
    let Some(((ex, ey, _), (xx, xy, _))) = find_entrance_exit(rs) else {
        return vec![];
    }; // neighbors of entrance/exit dir tiles
    let mut starts = Vec::new();
    let mut goals = Vec::new();
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let sx = ex + dx;
        let sy = ey + dy;
        let gx = xx + dx;
        let gy = xy + dy;
        let inb = |x: i32, y: i32| {
            x >= 0 && y >= 0 && (x as u32) < rs.grid_size.width && (y as u32) < rs.grid_size.height
        };
        if inb(sx, sy) {
            let idx = (sy as u32 * rs.grid_size.width + sx as u32) as usize;
            if matches!(rs.tiles[idx].kind, TileKind::Empty) {
                starts.push((sx, sy));
            }
        }
        if inb(gx, gy) {
            let idx = (gy as u32 * rs.grid_size.width + gx as u32) as usize;
            if matches!(rs.tiles[idx].kind, TileKind::Empty) {
                goals.push((gx, gy));
            }
        }
    }
    if starts.is_empty() || goals.is_empty() {
        return vec![];
    }
    let mut best: Option<Vec<Position>> = None;
    for s in &starts {
        for g in &goals {
            let p = a_star(rs, *s, *g);
            if p.len() > 1 {
                if best.as_ref().map(|b| p.len() < b.len()).unwrap_or(true) {
                    best = Some(p);
                }
            }
        }
    }
    best.unwrap_or_default()
}
fn build_loop_path(rs: &RunState) -> Vec<Position> {
    let mut start = None;
    let mut ent = None;
    let mut exit = None;
    for y in 0..rs.grid_size.height {
        for x in 0..rs.grid_size.width {
            let idx = (y * rs.grid_size.width + x) as usize;
            match rs.tiles[idx].kind {
                TileKind::Start => start = Some(Position { x, y }),
                TileKind::Direction {
                    role: DirRole::Entrance,
                    ..
                } => ent = Some(Position { x, y }),
                TileKind::Direction {
                    role: DirRole::Exit,
                    ..
                } => exit = Some(Position { x, y }),
                _ => {}
            }
        }
    }
    let (Some(s), Some(en), Some(ex)) = (start, ent, exit) else {
        return vec![];
    };
    let mut nodes = Vec::new();
    nodes.push(s);
    if nodes.last() != Some(&en) {
        nodes.push(en);
    }
    for p in &rs.path {
        if *p != s && *p != en && *p != ex {
            nodes.push(*p);
        }
    }
    if nodes.last() != Some(&ex) {
        nodes.push(ex);
    } // dedupe immediate
    let mut clean = Vec::new();
    for n in nodes {
        if clean.last() != Some(&n) {
            clean.push(n);
        }
    }
    clean
}
fn update_loop_geometry(rs: &mut RunState) {
    rs.loop_cum_lengths.clear();
    rs.loop_total_length = 0.0;
    if rs.path_loop.len() < 2 {
        return;
    }
    rs.loop_cum_lengths.push(0.0);
    let mut acc = 0.0;
    for i in 1..rs.path_loop.len() {
        let a = rs.path_loop[i - 1];
        let b = rs.path_loop[i];
        let dx = b.x as f64 - a.x as f64;
        let dy = b.y as f64 - a.y as f64;
        let d = (dx * dx + dy * dy).sqrt();
        acc += d;
        rs.loop_cum_lengths.push(acc);
    }
    let first = rs.path_loop[0];
    let last = *rs.path_loop.last().unwrap();
    let dx = first.x as f64 - last.x as f64;
    let dy = first.y as f64 - last.y as f64;
    rs.loop_total_length = acc + (dx * dx + dy * dy).sqrt();
}

// -------- Upgrades (new tree) --------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeId {
    TowerDamage1,
    FireRate,
    CritChance,
    CritDamage,
    ProjectileSpeed,
    AoeDamage,
    Bounce,
    HealthStart,
    VampiricHealing,
    LifeRegen,
    MiningSpeed,
    ResourceRecovery,
    GoldTileChance,
    GoldTileReward,
    StartingGold,
    Bank,
    MiningCrit,
    BoostColdUnlock,
    BoostColdFrequency,
    BoostColdSlowAmount,
    BoostColdSlowDuration,
    BoostColdFreezeChance,
    BoostPoisonUnlock,
    BoostPoisonFrequency,
    BoostPoisonDamage,
    BoostPoisonDuration,
    BoostPoisonSpread,
    BoostHealingUnlock,
    BoostHealingFrequency,
    BoostHealingPower,
    BoostHealingRadius,
    BoostHealingShield,
    // New meta progression upgrade for expanding grid
    PlayAreaSize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prereq {
    pub id: UpgradeId,
    pub level: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UpgradeDef {
    pub id: UpgradeId,
    pub display_name: &'static str,
    pub category: &'static str,
    pub max_level: u8,
    pub base_cost: u64,
    pub cost_multiplier: f64,
    pub effect_per_level: &'static str,
    pub prerequisites: &'static [Prereq],
}
macro_rules! prereqs { ($($id:ident : $lvl:literal),*$(,)?) => { &[ $( Prereq { id: UpgradeId::$id, level: $lvl }, )* ] }; }
pub static UPGRADE_DEFS: &[UpgradeDef] = &[
    UpgradeDef {
        id: UpgradeId::TowerDamage1,
        display_name: "Tower Damage",
        category: "Damage",
        max_level: 5,
        base_cost: 12,
        cost_multiplier: 1.6,
        effect_per_level: "+12% tower damage",
        prerequisites: &[],
    },
    UpgradeDef {
        id: UpgradeId::HealthStart,
        display_name: "Max Health",
        category: "Health",
        max_level: 5,
        base_cost: 14,
        cost_multiplier: 1.55,
        effect_per_level: "+5 max health",
        prerequisites: prereqs!(TowerDamage1:1),
    },
    UpgradeDef {
        id: UpgradeId::VampiricHealing,
        display_name: "Vampiric Healing",
        category: "Health",
        max_level: 3,
        base_cost: 40,
        cost_multiplier: 1.8,
        effect_per_level: "1% lifesteal",
        prerequisites: prereqs!(HealthStart:5),
    },
    UpgradeDef {
        id: UpgradeId::LifeRegen,
        display_name: "Life Regeneration",
        category: "Health",
        max_level: 5,
        base_cost: 35,
        cost_multiplier: 1.65,
        effect_per_level: "+0.5 HP/s",
        prerequisites: prereqs!(HealthStart:3),
    },
    UpgradeDef {
        id: UpgradeId::FireRate,
        display_name: "Fire Rate",
        category: "Damage",
        max_level: 5,
        base_cost: 16,
        cost_multiplier: 1.55,
        effect_per_level: "+8% fire rate",
        prerequisites: prereqs!(TowerDamage1:1),
    },
    UpgradeDef {
        id: UpgradeId::CritChance,
        display_name: "Crit Chance",
        category: "Damage",
        max_level: 5,
        base_cost: 25,
        cost_multiplier: 1.6,
        effect_per_level: "+3% crit chance",
        prerequisites: prereqs!(FireRate:3),
    },
    UpgradeDef {
        id: UpgradeId::CritDamage,
        display_name: "Crit Damage",
        category: "Damage",
        max_level: 5,
        base_cost: 40,
        cost_multiplier: 1.7,
        effect_per_level: "+25% crit dmg",
        prerequisites: prereqs!(CritChance:5),
    },
    UpgradeDef {
        id: UpgradeId::ProjectileSpeed,
        display_name: "Projectile Speed",
        category: "Damage",
        max_level: 3,
        base_cost: 20,
        cost_multiplier: 1.5,
        effect_per_level: "+15% projectile speed",
        prerequisites: prereqs!(FireRate:2),
    },
    UpgradeDef {
        id: UpgradeId::AoeDamage,
        display_name: "AoE Damage",
        category: "Damage",
        max_level: 3,
        base_cost: 45,
        cost_multiplier: 1.75,
        effect_per_level: "+1.5 AoE radius (todo)",
        prerequisites: prereqs!(ProjectileSpeed:3),
    },
    UpgradeDef {
        id: UpgradeId::Bounce,
        display_name: "Projectile Bounce",
        category: "Damage",
        max_level: 3,
        base_cost: 50,
        cost_multiplier: 1.8,
        effect_per_level: "+1 bounce (todo)",
        prerequisites: prereqs!(ProjectileSpeed:3),
    },
    UpgradeDef {
        id: UpgradeId::MiningSpeed,
        display_name: "Mining Speed",
        category: "Economy",
        max_level: 5,
        base_cost: 10,
        cost_multiplier: 1.5,
        // Adjusted from +10% to +8% to keep overall progression slower after baseline nerf
        effect_per_level: "+8% mining speed",
        prerequisites: prereqs!(TowerDamage1:1),
    },
    UpgradeDef {
        id: UpgradeId::ResourceRecovery,
        display_name: "Tower Refund",
        category: "Economy",
        max_level: 5,
        base_cost: 18,
        cost_multiplier: 1.55,
        effect_per_level: "+20% tower refund (todo)",
        prerequisites: prereqs!(MiningSpeed:3),
    },
    UpgradeDef {
        id: UpgradeId::GoldTileChance,
        display_name: "Gold Tile Chance",
        category: "Economy",
        max_level: 5,
        base_cost: 22,
        cost_multiplier: 1.6,
        effect_per_level: "+5% gold tile chance",
        prerequisites: prereqs!(MiningSpeed:2),
    },
    UpgradeDef {
        id: UpgradeId::GoldTileReward,
        display_name: "Gold Tile Reward",
        category: "Economy",
        max_level: 5,
        base_cost: 28,
        cost_multiplier: 1.65,
        effect_per_level: "+15% mined gold",
        prerequisites: prereqs!(GoldTileChance:3),
    },
    UpgradeDef {
        id: UpgradeId::StartingGold,
        display_name: "Starting Gold",
        category: "Economy",
        max_level: 5,
        base_cost: 20,
        cost_multiplier: 1.55,
        effect_per_level: "+2 starting gold",
        prerequisites: prereqs!(GoldTileChance:2),
    },
    UpgradeDef {
        id: UpgradeId::Bank,
        display_name: "Bank Interest",
        category: "Economy",
        max_level: 3,
        base_cost: 50,
        cost_multiplier: 1.8,
        effect_per_level: "+3% interest (todo)",
        prerequisites: prereqs!(StartingGold:5),
    },
    UpgradeDef {
        id: UpgradeId::MiningCrit,
        display_name: "Mining Crit",
        category: "Economy",
        max_level: 3,
        base_cost: 60,
        cost_multiplier: 1.85,
        effect_per_level: "5% mining crit (x2)",
        prerequisites: prereqs!(GoldTileReward:5),
    },
    UpgradeDef {
        id: UpgradeId::BoostColdUnlock,
        display_name: "Unlock Cold Tiles",
        category: "Boost",
        max_level: 1,
        base_cost: 30,
        cost_multiplier: 1.0,
        effect_per_level: "Unlock Cold tiles",
        prerequisites: prereqs!(TowerDamage1:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostColdFrequency,
        display_name: "Cold Frequency",
        category: "Boost",
        max_level: 5,
        base_cost: 20,
        cost_multiplier: 1.6,
        effect_per_level: "+5% Cold freq (todo)",
        prerequisites: prereqs!(BoostColdUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostColdSlowAmount,
        display_name: "Cold Slow Amount",
        category: "Boost",
        max_level: 5,
        base_cost: 25,
        cost_multiplier: 1.65,
        effect_per_level: "+10% slow amount (todo)",
        prerequisites: prereqs!(BoostColdUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostColdSlowDuration,
        display_name: "Cold Slow Duration",
        category: "Boost",
        max_level: 3,
        base_cost: 35,
        cost_multiplier: 1.7,
        effect_per_level: "+1s slow dur (todo)",
        prerequisites: prereqs!(BoostColdSlowAmount:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostColdFreezeChance,
        display_name: "Cold Freeze Chance",
        category: "Boost",
        max_level: 3,
        base_cost: 50,
        cost_multiplier: 1.85,
        effect_per_level: "+2% freeze (todo)",
        prerequisites: prereqs!(BoostColdSlowAmount:5),
    },
    UpgradeDef {
        id: UpgradeId::BoostPoisonUnlock,
        display_name: "Unlock Poison Tiles",
        category: "Boost",
        max_level: 1,
        base_cost: 35,
        cost_multiplier: 1.0,
        effect_per_level: "Unlock Poison tiles",
        prerequisites: prereqs!(TowerDamage1:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostPoisonFrequency,
        display_name: "Poison Frequency",
        category: "Boost",
        max_level: 5,
        base_cost: 25,
        cost_multiplier: 1.6,
        effect_per_level: "+5% Poison freq (todo)",
        prerequisites: prereqs!(BoostPoisonUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostPoisonDamage,
        display_name: "Poison Damage",
        category: "Boost",
        max_level: 5,
        base_cost: 30,
        cost_multiplier: 1.65,
        effect_per_level: "+5% poison dmg (todo)",
        prerequisites: prereqs!(BoostPoisonUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostPoisonDuration,
        display_name: "Poison Duration",
        category: "Boost",
        max_level: 3,
        base_cost: 40,
        cost_multiplier: 1.7,
        effect_per_level: "+1s poison dur (todo)",
        prerequisites: prereqs!(BoostPoisonDamage:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostPoisonSpread,
        display_name: "Poison Spread",
        category: "Boost",
        max_level: 3,
        base_cost: 55,
        cost_multiplier: 1.85,
        effect_per_level: "+1 poison spread (todo)",
        prerequisites: prereqs!(BoostPoisonDamage:5),
    },
    UpgradeDef {
        id: UpgradeId::BoostHealingUnlock,
        display_name: "Unlock Healing Tiles",
        category: "Boost",
        max_level: 1,
        base_cost: 35,
        cost_multiplier: 1.0,
        effect_per_level: "Unlock Healing tiles",
        prerequisites: prereqs!(VampiricHealing:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostHealingFrequency,
        display_name: "Healing Frequency",
        category: "Boost",
        max_level: 5,
        base_cost: 25,
        cost_multiplier: 1.6,
        effect_per_level: "+5% Healing freq (todo)",
        prerequisites: prereqs!(BoostHealingUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostHealingPower,
        display_name: "Healing Power",
        category: "Boost",
        max_level: 5,
        base_cost: 30,
        cost_multiplier: 1.65,
        effect_per_level: "+10% heal power (todo)",
        prerequisites: prereqs!(BoostHealingUnlock:1),
    },
    UpgradeDef {
        id: UpgradeId::BoostHealingRadius,
        display_name: "Healing Radius",
        category: "Boost",
        max_level: 3,
        base_cost: 40,
        cost_multiplier: 1.7,
        effect_per_level: "+1 heal radius (todo)",
        prerequisites: prereqs!(BoostHealingPower:3),
    },
    UpgradeDef {
        id: UpgradeId::BoostHealingShield,
        display_name: "Healing Shield",
        category: "Boost",
        max_level: 3,
        base_cost: 55,
        cost_multiplier: 1.85,
        effect_per_level: "5% shield on heal (todo)",
        prerequisites: prereqs!(BoostHealingPower:5),
    },
    UpgradeDef {
        id: UpgradeId::PlayAreaSize,
        display_name: "Play Area Size",
        category: "PlayArea",
        max_level: 19, // 0..19 -> 20 sizes
        base_cost: 30,
        cost_multiplier: 1.4,
        effect_per_level: "Expand play area size",
        prerequisites: prereqs!(TowerDamage1:1),
    },
];
// Progression of square grid sizes for PlayAreaSize levels 0..=19
pub const PLAY_AREA_SIZES: &[u32] = &[
    10, 14, 18, 24, 32, 40, 52, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 255,
];
pub fn play_area_size_for_level(level: u8) -> u32 {
    let i = level as usize;
    if i >= PLAY_AREA_SIZES.len() {
        *PLAY_AREA_SIZES.last().unwrap()
    } else {
        PLAY_AREA_SIZES[i]
    }
}
impl UpgradeId {
    pub fn key(self) -> &'static str {
        match self {
            UpgradeId::TowerDamage1 => "TowerDamage1",
            UpgradeId::FireRate => "FireRate",
            UpgradeId::CritChance => "CritChance",
            UpgradeId::CritDamage => "CritDamage",
            UpgradeId::ProjectileSpeed => "ProjectileSpeed",
            UpgradeId::AoeDamage => "AoeDamage",
            UpgradeId::Bounce => "Bounce",
            UpgradeId::HealthStart => "HealthStart",
            UpgradeId::VampiricHealing => "VampiricHealing",
            UpgradeId::LifeRegen => "LifeRegen",
            UpgradeId::MiningSpeed => "MiningSpeed",
            UpgradeId::ResourceRecovery => "ResourceRecovery",
            UpgradeId::GoldTileChance => "GoldTileChance",
            UpgradeId::GoldTileReward => "GoldTileReward",
            UpgradeId::StartingGold => "StartingGold",
            UpgradeId::Bank => "Bank",
            UpgradeId::MiningCrit => "MiningCrit",
            UpgradeId::BoostColdUnlock => "BoostColdUnlock",
            UpgradeId::BoostColdFrequency => "BoostColdFrequency",
            UpgradeId::BoostColdSlowAmount => "BoostColdSlowAmount",
            UpgradeId::BoostColdSlowDuration => "BoostColdSlowDuration",
            UpgradeId::BoostColdFreezeChance => "BoostColdFreezeChance",
            UpgradeId::BoostPoisonUnlock => "BoostPoisonUnlock",
            UpgradeId::BoostPoisonFrequency => "BoostPoisonFrequency",
            UpgradeId::BoostPoisonDamage => "BoostPoisonDamage",
            UpgradeId::BoostPoisonDuration => "BoostPoisonDuration",
            UpgradeId::BoostPoisonSpread => "BoostPoisonSpread",
            UpgradeId::BoostHealingUnlock => "BoostHealingUnlock",
            UpgradeId::BoostHealingFrequency => "BoostHealingFrequency",
            UpgradeId::BoostHealingPower => "BoostHealingPower",
            UpgradeId::BoostHealingRadius => "BoostHealingRadius",
            UpgradeId::BoostHealingShield => "BoostHealingShield",
            UpgradeId::PlayAreaSize => "PlayAreaSize",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpgradeState {
    pub levels: std::collections::HashMap<String, u8>,
    pub tower_refund_rate_percent: u8,
}
impl Default for UpgradeState {
    fn default() -> Self {
        use std::collections::HashMap;
        let mut levels = HashMap::new();
        for d in UPGRADE_DEFS {
            levels.insert(d.id.key().into(), 0);
        }
        Self {
            levels,
            tower_refund_rate_percent: 100,
        }
    }
}
impl UpgradeState {
    pub fn level(&self, id: UpgradeId) -> u8 {
        *self.levels.get(id.key()).unwrap_or(&0)
    }
    pub fn is_unlocked(&self, id: UpgradeId) -> bool {
        let def = UPGRADE_DEFS.iter().find(|d| d.id == id).unwrap();
        def.prerequisites
            .iter()
            .all(|p| self.level(p.id) >= p.level)
    }
    pub fn max_level(&self, id: UpgradeId) -> u8 {
        UPGRADE_DEFS.iter().find(|d| d.id == id).unwrap().max_level
    }
    pub fn next_cost(&self, id: UpgradeId) -> Option<u64> {
        let def = UPGRADE_DEFS.iter().find(|d| d.id == id).unwrap();
        let lvl = self.level(id);
        if lvl >= def.max_level {
            None
        } else {
            Some((def.base_cost as f64 * def.cost_multiplier.powi(lvl as i32)).round() as u64)
        }
    }
    pub fn can_purchase(&self, id: UpgradeId) -> bool {
        self.is_unlocked(id) && self.level(id) < self.max_level(id)
    }
    pub fn purchase(&mut self, id: UpgradeId) {
        let c = self.level(id);
        if c < self.max_level(id) {
            self.levels.insert(id.key().into(), c + 1);
        }
    }
    pub fn total_spent(&self) -> u64 {
        let mut sum = 0u64;
        for def in UPGRADE_DEFS {
            let lvl = self.level(def.id) as i32;
            if lvl <= 0 {
                continue;
            }
            for i in 0..lvl {
                let cost = (def.base_cost as f64 * def.cost_multiplier.powi(i)).round() as u64;
                sum = sum.saturating_add(cost);
            }
        }
        sum
    }
}
pub fn apply_upgrades_to_run(run: &mut RunState, ups: &UpgradeState) {
    use UpgradeId::*;
    let l = |id: UpgradeId| ups.level(id) as f64;
    run.mining_speed = 2.0 * (1.0 + 0.08 * l(MiningSpeed));
    run.tower_base_damage = (2.0 * (1.0 + 0.12 * l(TowerDamage1))) as u32;
    run.tower_fire_rate_global = 1.0 + 0.08 * l(FireRate);
    run.crit_chance = 0.03 * l(CritChance);
    run.crit_damage_mult = 1.0 + 0.25 * l(CritDamage);
    run.projectile_speed = 8.0 * (1.0 + 0.15 * l(ProjectileSpeed));
    run.life_regen_per_sec = 0.5 * l(LifeRegen);
    run.vampiric_heal_percent = 0.01 * l(VampiricHealing);
    run.mining_gold_mul = 1.0 + 0.15 * l(GoldTileReward);
    run.mining_crit_chance = 0.05 * l(MiningCrit);
    if run.stats.time_survived_secs == 0 && !run.started {
        // Apply life & starting gold only once while pre-run (before any survival time or start)
        run.life_max = 10 + 5 * ups.level(HealthStart) as u32;
        run.life = run.life_max;
        let sg_level = ups.level(StartingGold);
        if sg_level > run.starting_gold_applied_level {
            let delta_levels = sg_level - run.starting_gold_applied_level;
            // Each level grants +2 starting gold (matches upgrade definition)
            run.currencies.gold = run
                .currencies
                .gold
                .saturating_add(2 * delta_levels as u64);
            run.starting_gold_applied_level = sg_level;
        }
    }
    run.life_max = 10 + 5 * ups.level(HealthStart) as u32; // keep max updated for mid-run effects (no gold change mid-run)
    if run.life > run.life_max {
        run.life = run.life_max;
    }
    for tw in &mut run.towers {
        let (rm, dm, fr) = match tw.kind {
            TowerKind::Basic => (1.0, 1.0, 1.0),
            TowerKind::Slow => (1.1, 0.5, 0.75),
            TowerKind::Damage => (0.9, 1.5, 0.8),
        };
        tw.range = run.tower_base_range * rm;
        tw.damage = ((run.tower_base_damage as f64) * dm).round() as u32;
        tw.fire_rate = fr * run.tower_fire_rate_global;
    }
}

// === Actions & Reducer ===
#[derive(Clone, Debug)]
pub enum RunAction {
    TogglePause,
    StartRun,
    TickSecond,
    MiningComplete { idx: usize },
    SimTick { dt: f64 },
    ResetRun,
    ResetRunWithUpgrades { ups: UpgradeState },
    PlaceWall { x: u32, y: u32 },
    PlaceTower { x: u32, y: u32 },
    RemoveTower { x: u32, y: u32 },
    SpendResearch { amount: u64 },
    ApplyUpgrades { ups: UpgradeState },
    SetResearch { amount: u64 },
}

impl yew::Reducible for RunState {
    type Action = RunAction;
    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        use RunAction::*;
        if let ResetRunWithUpgrades { ups } = &action {
            let prev_r = self.currencies.research;
            let size = play_area_size_for_level(ups.level(UpgradeId::PlayAreaSize));
            let mut fresh = RunState::new_with_upgrades(
                GridSize {
                    width: size,
                    height: size,
                },
                ups,
            );
            fresh.currencies.research = prev_r;
            fresh.run_id = self.run_id + 1;
            return Rc::new(fresh);
        }
        if matches!(action, ResetRun) {
            let prev_r = self.currencies.research;
            let mut fresh = RunState::new_basic(self.grid_size);
            fresh.currencies.research = prev_r;
            fresh.run_id = self.run_id + 1;
            return Rc::new(fresh);
        }
        let mut new = (*self).clone();
        match action {
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
            MiningComplete { idx } => {
                if !new.game_over && idx < new.tiles.len() {
                    new.last_mined_idx = Some(idx);
                    match new.tiles[idx].kind {
                        TileKind::Rock { has_gold, .. } => {
                            new.tiles[idx].kind = TileKind::Empty;
                            new.tiles[idx].hardness = 1;
                            new.stats.blocks_mined = new.stats.blocks_mined.saturating_add(1);
                            new.currencies.tile_credits =
                                new.currencies.tile_credits.saturating_add(1);
                            if has_gold {
                                let mut g = 1.0 * new.mining_gold_mul;
                                if new.mining_crit_chance > 0.0
                                    && js_sys::Math::random() < new.mining_crit_chance
                                {
                                    g *= 2.0;
                                }
                                new.currencies.gold =
                                    new.currencies.gold.saturating_add(g.round() as u64);
                            }
                            new.path = compute_path(&new);
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
                        }
                        TileKind::Wall => {
                            new.tiles[idx].kind = TileKind::Empty;
                            new.tiles[idx].hardness = 1;
                            new.currencies.tile_credits =
                                new.currencies.tile_credits.saturating_add(1);
                            new.path = compute_path(&new);
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
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
                {
                    let t = new.stats.time_survived_secs as f64;
                    let max_interval = 2.0;
                    let min_interval = 0.5;
                    let spawn_interval = (max_interval - t * 0.01).max(min_interval);
                    if (new.stats.time_survived_secs as f64 - new.last_enemy_spawn_time_secs)
                        >= spawn_interval
                        && !new.path_loop.is_empty()
                    {
                        if let Some((idx, _tile)) = new
                            .tiles
                            .iter()
                            .enumerate()
                            .find(|(_, t)| matches!(t.kind, TileKind::Start))
                        {
                            let sx = (idx as u32) % new.grid_size.width;
                            let sy = (idx as u32) / new.grid_size.width;
                            let hp = 5 + (new.stats.loops_completed / 2) as u32;
                            let speed = 1.5 + (new.stats.loops_completed as f64) * 0.05;
                            new.enemies.push(Enemy {
                                x: sx as f64 + 0.5,
                                y: sy as f64 + 0.5,
                                speed_tps: speed,
                                hp,
                                spawned_at: new.stats.time_survived_secs,
                                path_index: 0,
                                dir_dx: 1.0,
                                dir_dy: 0.0,
                                radius_scale: 1.0,
                                loop_dist: 0.0,
                            });
                            new.last_enemy_spawn_time_secs = new.stats.time_survived_secs as f64;
                        }
                    }
                }
                if !new.towers.is_empty() && !new.enemies.is_empty() {
                    for tw in &mut new.towers {
                        if tw.cooldown_remaining > 0.0 {
                            tw.cooldown_remaining -= dt;
                        }
                        if tw.cooldown_remaining > 0.0 {
                            continue;
                        }
                        let cx = tw.x as f64 + 0.5;
                        let cy = tw.y as f64 + 0.5;
                        let mut target = None::<usize>;
                        for (i, e) in new.enemies.iter().enumerate() {
                            let dx = e.x - cx;
                            let dy = e.y - cy;
                            if dx * dx + dy * dy <= tw.range * tw.range {
                                target = Some(i);
                                break;
                            }
                        }
                        if let Some(i) = target {
                            let e = &new.enemies[i];
                            let dx = e.x - cx;
                            let dy = e.y - cy;
                            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
                            let speed = new.projectile_speed;
                            let travel = dist / speed;
                            let mut dmg = tw.damage as f64;
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
                                remaining: travel,
                                damage: dmg.round() as u32,
                                splash_radius: 0.0,
                            });
                            tw.cooldown_remaining =
                                1.0 / (tw.fire_rate * new.tower_fire_rate_global.max(0.01));
                        }
                    }
                }
                if !new.projectiles.is_empty() {
                    let mut kills = 0u64;
                    let mut i = 0;
                    while i < new.projectiles.len() {
                        let mut remove = false;
                        {
                            let p = &mut new.projectiles[i];
                            p.x += p.vx * dt;
                            p.y += p.vy * dt;
                            p.remaining -= dt;
                            if p.remaining <= 0.0 {
                                let ix = p.x;
                                let iy = p.y;
                                let mut hit = None;
                                let mut best = 0.3f64 * 0.3;
                                for (ei, e) in new.enemies.iter().enumerate() {
                                    let dx = e.x - ix;
                                    let dy = e.y - iy;
                                    let d2 = dx * dx + dy * dy;
                                    if d2 <= best {
                                        best = d2;
                                        hit = Some(ei);
                                    }
                                }
                                if let Some(h) = hit {
                                    if let Some(e) = new.enemies.get_mut(h) {
                                        let applied = p.damage.min(e.hp);
                                        if p.damage >= e.hp {
                                            e.hp = 0;
                                        } else {
                                            e.hp -= p.damage;
                                        }
                                        if new.vampiric_heal_percent > 0.0
                                            && new.life < new.life_max
                                        {
                                            let heal = (applied as f64 * new.vampiric_heal_percent)
                                                .floor()
                                                as u32;
                                            if heal > 0 {
                                                new.life = (new.life + heal).min(new.life_max);
                                            }
                                        }
                                        new.damage_numbers.push(DamageNumber {
                                            x: e.x,
                                            y: e.y,
                                            amount: applied,
                                            ttl: 0.8,
                                        });
                                    }
                                }
                                remove = true;
                            }
                        }
                        if remove {
                            new.projectiles.remove(i);
                        } else {
                            i += 1;
                        }
                    }
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
                            new.currencies.research = new.currencies.research.saturating_add(kills);
                            if new.gold_bounty_per_kill > 0 {
                                new.currencies.gold = new
                                    .currencies
                                    .gold
                                    .saturating_add(kills * new.gold_bounty_per_kill);
                            }
                        }
                    }
                }
                for dn in &mut new.damage_numbers {
                    dn.ttl -= dt;
                }
                new.damage_numbers.retain(|d| d.ttl > 0.0);
                if new.loop_total_length > 0.0
                    && new.path_loop.len() >= 2
                    && !new.enemies.is_empty()
                {
                    let total = new.loop_total_length;
                    let sample_pos = |nodes: &Vec<Position>, cum: &Vec<f64>, total: f64, d: f64| {
                        if nodes.len() < 2 || total <= 0.0 {
                            return (0.0, 0.0, 0.0, 0.0, 0usize);
                        }
                        let dist = d % total;
                        let mut seg_i = 0usize;
                        while seg_i + 1 < cum.len() && cum[seg_i + 1] <= dist {
                            seg_i += 1;
                        }
                        let (ax, ay, bx, by, seg_len) = if seg_i + 1 < nodes.len() {
                            let a = nodes[seg_i];
                            let b = nodes[seg_i + 1];
                            let seg_len = (((b.x as f64 - a.x as f64).powi(2)
                                + (b.y as f64 - a.y as f64).powi(2))
                            .sqrt())
                            .max(1e-6);
                            (
                                a.x as f64 + 0.5,
                                a.y as f64 + 0.5,
                                b.x as f64 + 0.5,
                                b.y as f64 + 0.5,
                                seg_len,
                            )
                        } else {
                            let a = nodes.last().unwrap();
                            let b = nodes[0];
                            let seg_len = (((b.x as f64 - a.x as f64).powi(2)
                                + (b.y as f64 - a.y as f64).powi(2))
                            .sqrt())
                            .max(1e-6);
                            (
                                a.x as f64 + 0.5,
                                a.y as f64 + 0.5,
                                b.x as f64 + 0.5,
                                b.y as f64 + 0.5,
                                seg_len,
                            )
                        };
                        let base = cum.get(seg_i).copied().unwrap_or(0.0);
                        let t = ((dist - base) / seg_len).clamp(0.0, 1.0);
                        let dx = bx - ax;
                        let dy = by - ay;
                        let nx = ax + dx * t;
                        let ny = ay + dy * t;
                        (
                            nx,
                            ny,
                            dx / seg_len,
                            dy / seg_len,
                            (seg_i + 1) % nodes.len(),
                        )
                    };
                    for e in &mut new.enemies {
                        e.loop_dist += e.speed_tps * dt;
                        if e.loop_dist >= total {
                            e.loop_dist %= total;
                            if new.life > 0 {
                                new.life = new.life.saturating_sub(1);
                                if new.life == 0 {
                                    new.game_over = true;
                                }
                            }
                            if new.stats.loops_completed < u32::MAX {
                                new.stats.loops_completed += 1;
                            }
                        }
                        let (nx, ny, dx, dy, next_i) =
                            sample_pos(&new.path_loop, &new.loop_cum_lengths, total, e.loop_dist);
                        e.x = nx;
                        e.y = ny;
                        e.dir_dx = dx;
                        e.dir_dy = dy;
                        e.path_index = next_i;
                    }
                }
            }
            PlaceWall { x, y } => {
                let gs = new.grid_size;
                if x < gs.width && y < gs.height {
                    let idx = (y * gs.width + x) as usize;
                    if matches!(new.tiles[idx].kind, TileKind::Empty) {
                        let old = new.tiles[idx].kind.clone();
                        new.tiles[idx].kind = TileKind::Wall;
                        if compute_path(&new).is_empty() {
                            new.tiles[idx].kind = old;
                        } else {
                            new.path = compute_path(&new);
                            new.path_loop = build_loop_path(&new);
                            update_loop_geometry(&mut new);
                        }
                    }
                }
            }
            PlaceTower { x, y } => {
                let gs = new.grid_size;
                if x < gs.width && y < gs.height && new.currencies.gold >= new.tower_cost {
                    let idx = (y * gs.width + x) as usize;
                    if matches!(new.tiles[idx].kind, TileKind::Rock { .. } | TileKind::Wall)
                        && !new.towers.iter().any(|t| t.x == x && t.y == y)
                    {
                        new.currencies.gold -= new.tower_cost;
                        new.towers.push(Tower::new(
                            x,
                            y,
                            TowerKind::Basic,
                            new.tower_base_range,
                            new.tower_base_damage,
                        ));
                    }
                }
            }
            RemoveTower { x, y } => {
                if let Some(p) = new.towers.iter().position(|t| t.x == x && t.y == y) {
                    new.towers.remove(p);
                    new.currencies.gold = new.currencies.gold.saturating_add(new.tower_cost);
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
            ResetRun | ResetRunWithUpgrades { .. } => unreachable!(),
        }
        new.version = new.version.wrapping_add(1);
        Rc::new(new)
    }
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    fn make_run() -> RunState {
        RunState::new_basic(GridSize {
            width: 25,
            height: 25,
        })
    }

    #[test]
    fn start_and_indestructibles_exist() {
        let rs = make_run();
        let mut start_idx = None;
        for (i, t) in rs.tiles.iter().enumerate() {
            if matches!(t.kind, TileKind::Start) {
                start_idx = Some(i);
                break;
            }
        }
        assert!(start_idx.is_some(), "Start tile missing");
        let idx = start_idx.unwrap();
        let sx = (idx as u32) % rs.grid_size.width;
        let sy = (idx as u32) / rs.grid_size.width;
        // At least one orthogonal indestructible
        let mut found = false;
        for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
            let x = sx as i32 + dx;
            let y = sy as i32 + dy;
            if x >= 0
                && y >= 0
                && (x as u32) < rs.grid_size.width
                && (y as u32) < rs.grid_size.height
            {
                let i2 = (y as u32 * rs.grid_size.width + x as u32) as usize;
                if matches!(rs.tiles[i2].kind, TileKind::Indestructible) {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "No indestructible neighbor to start");
        assert!(!rs.path.is_empty(), "Initial path empty");
        assert!(rs.path_loop.len() >= 2, "Loop path too short");
    }

    #[test]
    fn enemy_spawns_after_time() {
        let mut rs = make_run();
        rs.started = true; // simulate StartRun
        rs.stats.time_survived_secs = 10; // large enough to exceed spawn interval
        let rc = Rc::new(rs);
        let after = rc.reduce(super::RunAction::SimTick { dt: 0.016 });
        assert!(after.enemies.len() >= 1, "Enemy did not spawn");
    }

    #[test]
    fn starting_gold_applied_only_once() {
        // Prepare upgrades with StartingGold level 3
        let mut ups = UpgradeState::default();
        ups.levels.insert(UpgradeId::StartingGold.key().into(), 3);
        let rs = RunState::new_with_upgrades(GridSize { width: 10, height: 10 }, &ups);
        // Base gold is 2, each level +2 => +6 => 8 total
        assert_eq!(rs.currencies.gold, 8, "Starting gold not applied correctly on new run");
        assert_eq!(rs.starting_gold_applied_level, 3, "Applied level tracker incorrect");
        // Simulate mid-run (started) and re-apply upgrades -- should not change gold
        let mut rs2 = rs.clone();
        let before = rs2.currencies.gold;
        rs2.started = true;
        rs2.stats.time_survived_secs = 5;
        apply_upgrades_to_run(&mut rs2, &ups);
        assert_eq!(rs2.currencies.gold, before, "Gold changed mid-run when it should not");
    }

    #[test]
    fn starting_gold_applies_difference_incrementally_pre_run() {
        let mut ups = UpgradeState::default();
        let mut rs = RunState::new_basic(GridSize { width: 10, height: 10 });
        // level 0 -> 1
        ups.levels.insert(UpgradeId::StartingGold.key().into(), 1);
        apply_upgrades_to_run(&mut rs, &ups);
        assert_eq!(rs.currencies.gold, 2 + 2 * 1, "Level 1 starting gold incorrect");
        assert_eq!(rs.starting_gold_applied_level, 1);
        // level 1 -> 3 adds only +4 more (delta levels =2)
        ups.levels.insert(UpgradeId::StartingGold.key().into(), 3);
        apply_upgrades_to_run(&mut rs, &ups);
        assert_eq!(rs.currencies.gold, 2 + 2 * 3, "Incremental starting gold difference not applied correctly");
        assert_eq!(rs.starting_gold_applied_level, 3);
    }
}
