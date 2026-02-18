# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-18
**Commit:** 28bd8fb
**Branch:** main

## OVERVIEW

Rust + Yew browser tower-defense/incremental hybrid compiled to WASM via Trunk. Single-crate, Canvas 2D rendering, ~6.4k lines across 25 `.rs` files.

## STRUCTURE

```
./
├── src/
│   ├── main.rs              # Entry: declares mods, renders App
│   ├── model.rs             # ALL game logic + data (2.2k lines) — the brain
│   ├── util.rs              # format_time, clog (disabled debug logger)
│   ├── components/          # 17 Yew function components (see components/AGENTS.md)
│   └── state/               # 4 ephemeral UI state helpers
│       ├── camera.rs        # Pan/zoom struct (25 lines)
│       ├── mining.rs        # Mining progress tracking (12 lines)
│       ├── touch.rs         # Touch gesture state (13 lines)
│       └── interactable.rs  # BFS reachability mask from path (101 lines)
├── index.html               # Bare entry; inline CSS, no external assets
├── Cargo.toml               # edition 2024, yew 0.21, wasm-bindgen, web-sys, serde
├── CLAUDE.md                # Detailed project guide (primary reference)
└── .github/copilot-instructions.md  # Coding conventions v0.2
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Game data structures | `src/model.rs:18-211` | GridSize, TileKind, Enemy, Tower, Projectile, RunState |
| Upgrade system | `src/model.rs:741-1280` | UpgradeId (35 variants), UPGRADE_DEFS array, UpgradeState |
| Simulation tick / reducer | `src/model.rs:1449-2095` | RunAction enum + `impl Reducible for RunState` |
| Pathfinding (A*) | `src/model.rs:540-628` | `a_star()` — 4-connected, no diagonals |
| Path loop construction | `src/model.rs:629-715` | `compute_path()`, `build_loop_path()` |
| Grid generation | `src/model.rs:252-472` | `create_run_base()` — initial grid + Start/End placement |
| Upgrade application | `src/model.rs:1368-1447` | `apply_upgrades_to_run()` — modifies RunState from UpgradeState |
| Boost/debuff calculation | `src/model.rs:1280-1366` | `calculate_boost_multipliers()`, `calculate_debuff_from_boost()` |
| App routing + persistence | `src/components/app.rs` | View::Run / View::Upgrades, localStorage load/save |
| Canvas rendering + input | `src/components/run_view.rs` | 1.9k lines — game loop, event handlers, all drawing |
| Upgrade Web UI | `src/components/upgrades_view.rs` | Radial layout, pan/zoom, purchase flow |
| Hover reachability | `src/state/interactable.rs` | BFS flood from path tiles; marks adjacent Rock/Wall |

## CODE MAP — model.rs (core)

| Symbol | Type | Line | Role |
|--------|------|------|------|
| `RunState` | struct | 129 | Per-run simulation state (grid, enemies, towers, currencies) |
| `UpgradeState` | struct | 1215 | Persistent meta-progression (levels HashMap) |
| `RunAction` | enum | 1449 | All mutations: SimTick, PlaceTower, MiningComplete, etc. |
| `impl Reducible` | impl | 1467 | Yew reducer — THE place all RunState mutations happen |
| `UpgradeId` | enum | 741 | 35 upgrade variants across 4 categories |
| `UPGRADE_DEFS` | const | 800 | Metadata array: costs, effects, prerequisites |
| `TileKind` | enum | 50 | Rock, Wall, Empty, Start, End, Direction, Indestructible |
| `TowerKind` | enum | 184 | Basic, Slow, Damage |
| `BoostKind` | enum | 29 | Range, Damage, FireRate, Slow, Fire |
| `a_star()` | fn | 540 | Pathfinding (4-connected grid) |
| `apply_upgrades_to_run()` | fn | 1368 | Maps UpgradeState levels → RunState parameters |
| `create_run_base()` | fn | 253 | Grid generation with random Start/End placement |

## CONVENTIONS

- **Mutations via reducer ONLY** — never mutate RunState outside `impl Reducible::reduce`
- **No panic in WASM** — return early on invalid indices; never `unwrap()` on user/storage data
- **Exhaustive match** — adding enum variant requires auditing ALL match sites
- **f64 for world math** — positions are continuous; grid indices are u32
- **Cleanup everything** — listeners, intervals, animation frames in effect teardowns
- **Minimal per-frame allocs** — reuse Vec buffers; short-lived clones OK
- **localStorage prefix** — all keys start with `md_` (md_upgrade_state, md_research, md_intro_seen, etc.)
- **Coordinate systems** — grid: integer (x,y); world: f64 tile-centered (1.0 = one tile)

## ANTI-PATTERNS (THIS PROJECT)

- `as any` / type suppression — not applicable (Rust), but: avoid `#[allow(unused)]` without reason
- Empty error handling — never `let _ = result;` on localStorage ops without reason
- Path recompute forgotten — EVERY terrain mutation must call path recompute; reject if path breaks
- Mining while paused — always check `is_paused` before advancing mining progress
- Duplicate intervals after hot-reload — ensure cleanup closures drop previous interval/RAF handles
- Adding UpgradeId without updating: `UPGRADE_DEFS`, `apply_upgrades_to_run()`, `UpgradeId::key()`, serialization defaults

## STATE ARCHITECTURE

```
App (root)
├── RunState         use_reducer    Transient per-run (grid, enemies, towers, currencies)
├── UpgradeState     use_state      Persistent meta (localStorage: md_upgrade_state)
├── hard_reset_counter  use_state   Forces full remount on hard reset
│
├── RunView
│   ├── Camera       use_state      Pan/zoom (offset_x/y, zoom level)
│   ├── Mining       use_state      Progress bar state (tile, elapsed, active)
│   ├── TouchState   use_state      Gesture tracking
│   ├── interactable_mask  computed  BFS reachability from path
│   └── 10+ child overlay components (props only, no own state)
│
└── UpgradesView
    ├── zoom/offset  use_state      Web navigation state
    └── UpgradeSummaryPanel (props only)
```

**Data flow:** App owns RunState + UpgradeState → passes as props/handles → components dispatch RunAction → reducer mutates → re-render.

## COMMANDS

```bash
trunk serve --open          # Dev server + hot reload
trunk build --release       # Production build → dist/
cargo check                 # Type check (no WASM)
cargo fmt                   # Format
cargo test                  # Unit tests (model.rs only, 4 tests)
```

## NOTES

- **model.rs is 2.2k lines** — monolithic by design; holds ALL game logic. Future split candidates: pathfinding, upgrades, simulation.
- **run_view.rs is 1.9k lines** — rendering + input + game loop. Complex but unavoidable with Canvas 2D.
- **No Trunk.toml** — uses Trunk defaults. Build config is implicit.
- **No CI workflows** — `.github/` has copilot-instructions only.
- **Tests are minimal** — 4 unit tests in model.rs (grid generation, enemy spawning, starting gold). Testing strategy documented in CLAUDE.md §Testing.
- **clog() is disabled** — `src/util.rs` logging is a no-op. For debug, temporarily set `DEBUG_LOG = true` in model.rs.
- **Touch support partial** — pinch zoom not implemented; single-touch mining/panning works.
