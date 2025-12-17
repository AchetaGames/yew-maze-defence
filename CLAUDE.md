# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Maze Defence is a browser-based 2D tower defense and incremental game hybrid built with Rust and Yew, compiled to WebAssembly. Players construct mazes by placing blocks to divert enemies, mine resources, build towers, and survive endless waves. The game features a meta-progression system where players spend research points on permanent upgrades between runs.

## Build & Development Commands

**Prerequisites:** Rust stable, Trunk, wasm32 target

```bash
# Install wasm32 target (one-time setup)
rustup target add wasm32-unknown-unknown

# Install Trunk (one-time setup)
cargo install trunk

# Development server with hot reload
trunk serve --open

# Production build (outputs to dist/)
trunk build --release

# Check code (no wasm compilation)
cargo check

# Format code
cargo fmt

# Run tests (currently minimal; see testing strategy in copilot-instructions.md)
cargo test
```

## Architecture

### State Management

The application uses a layered state architecture:

1. **RunState** (`src/model.rs`): Transient per-run simulation state
   - Grid tiles, enemies, towers, projectiles
   - Currencies (gold, research accumulator), life, stats
   - Path data, mining timers, simulation time
   - **Never persisted** - ephemeral by design

2. **UpgradeState** (`src/model.rs`): Persistent meta-progression
   - Upgrade levels for each UpgradeId
   - Stored in localStorage as `md_upgrade_state` (JSON)
   - Applied to new runs via `RunState::new_with_upgrades()`

3. **Ephemeral UI State** (`src/state/*.rs`):
   - CameraState: pan, zoom, viewport
   - MiningState: progress bar, target tile
   - TouchState: touch gesture tracking
   - Interactable mask: hover feedback

### Component Structure

- **src/main.rs**: Entry point, renders App component
- **src/components/app.rs**: Root router switching between Run and Upgrades views
- **src/components/run_view.rs**: Main game canvas + simulation tick loop + input handling
- **src/components/upgrades_view.rs**: Upgrade Web (radial cluster layout, pan/zoom, purchase logic)
- **src/components/**: UI overlays (stats panels, controls, legend, game over, intro)
- **src/model.rs**: Core data structures, enums, game logic (pathfinding, simulation)
- **src/state/**: Modular state helpers (camera, mining, touch, interactable)
- **src/util.rs**: Utility functions (formatting, logging wrapper `clog`)

### Simulation Loop

- **Fixed timestep** (~16ms): Dispatches `RunAction` mutations via Yew reducer
  - Enemy movement, tower firing, projectile updates
  - Mining progress, life regeneration, research accumulation
  - **All mutation logic is pure** - centralized in reducer actions
- **requestAnimationFrame**: Rendering only (read-only snapshot of RunState)
- **Event listeners**: Manually registered with cleanup on component unmount

### Coordinate Systems

- **Grid**: Integer indices (x, y) in range [0, width) × [0, height)
- **World units**: Tile-centered continuous f64 positions (1.0 = one tile)
- Entities (enemies, projectiles) use continuous world coordinates

### Pathfinding

- Custom **A\* implementation** (4-connected, no diagonals) in `src/model.rs`
- Recomputed on every terrain mutation (mining, wall placement)
- **Path integrity requirement**: Must always maintain Start→End connectivity
  - Blocking actions are rejected (previous state preserved)
- Path stored as `Vec<Position>` in RunState
- Loop path includes entrance/exit direction tiles for enemy spawning

## Key Game Mechanics

### Mining
- Hold LMB on Rock/Wall tile → progress bar fills based on hardness vs mining_speed
- Cancels on pointer exit, pause, or tile destruction
- Rock tiles may contain gold or boost tiles (if unlocked)
- Mining converts Rock/Wall → Empty

### Tower Placement
- Hotkey: Press **T** while hovering a Rock tile
- Places tower if valid (sufficient gold, path not blocked)
- Pressing **T** on existing tower → removes + full refund
- Towers automatically target and fire at enemies in range
- Boost tiles (Cold, Poison, Healing - if unlocked) modify tower stats when tower sits on boost

### Enemy Behavior
- Spawn from Start, follow path_loop, return to Start
- Each completed loop reduces player Life
- Life ≤ 0 → game_over (simulation halts, Game Over overlay shown)
- Enemy stats scale with elapsed time

### Currencies & Progression
- **Gold** (run-only): Mining, bounties → spend on towers
- **Research** (meta): Earned per run → spend on Upgrade Web → persists in localStorage
- On run end: Convert performance stats → research points

### Upgrade Web
- Four color-coded categories positioned radially:
  - **Health** (green, north): Max Life, Life Regen
  - **Mining/Gold** (amber, south): Mining Speed, Gold Gain, Grid Expansion, Starting Gold
  - **Damage/Offense** (red, east): Tower Damage, Range, Fire Rate, Crit Chance/Damage
  - **Boost/Utility** (blue, west): Unlock & improve boost tiles (Cold, Poison, Healing)
- Curved Bézier dependency edges (prerequisite → dependent)
- Pan (click-drag), zoom (mouse wheel), reset view controls

## Persistence

Uses localStorage with `md_` prefix:
- `md_upgrade_state`: JSON serialized UpgradeState
- `md_research`: Research currency value

**No mid-run save/resume** - runs are ephemeral by design.

## Controls

- **Space**: Pause/Resume (or dismiss overlays)
- **T**: Place/Remove tower at hovered Rock tile
- **LMB hold**: Mine Rock/Wall tile
- **Mouse wheel**: Zoom (cursor focal point)
- **Middle/Secondary mouse drag**: Pan camera
- **Arrow buttons**: Pan camera (UI overlay)
- **+/- buttons**: Zoom in/out (UI overlay)
- **Center button**: Recenter camera on Start tile

## Adding Features

### New Upgrade
1. Add variant to `UpgradeId` enum in `src/model.rs`
2. Add metadata to `UPGRADE_DEFS` array (name, category, max level, cost curve, prereqs)
3. Integrate effect:
   - Initial stat modification: `RunState::new_with_upgrades()`
   - Per-tick effect: Simulation tick logic in reducer
4. Add node position in `src/components/upgrades_view.rs` layout
5. Ensure default level = 0 for backward compatibility

### New Tower Type
1. Add variant to `TowerKind` enum in `src/model.rs`
2. Update `Tower::new()` constructor with stat multipliers
3. Add rendering branch in `src/components/run_view.rs` (color, glyph, projectile)
4. Extend firing/targeting logic in simulation tick
5. Update legend dynamic inclusion logic

### New Tile Type / Boost
1. Add variant to `TileKind` enum in `src/model.rs`
2. Add rendering (color, hatch pattern) in `src/components/run_view.rs`
3. Update placement/mining eligibility rules
4. Update path-blocking logic (if applicable)
5. Apply boost effects during tower stat calculations
6. Update legend inclusion logic

### Pathfinding Changes
- All terrain mutations must call `recompute_path()` or equivalent
- If path becomes invalid (None), reject the mutation
- Current implementation: 4-connected A\* (no diagonals planned)
- For large grids, consider caching or incremental path repairs

## Coding Conventions

From `.github/copilot-instructions.md`:

- **Mutate RunState only via reducer dispatch** - no bypassing actions
- **Avoid panic! in wasm callbacks** - return early on invalid indices
- **Exhaustive match on enums** - audit all usage sites when adding variants
- Use **f64 for world math**; clamp zoom levels
- **Minimize per-frame allocations** - reuse buffers, short-lived clones acceptable
- **Always clean up**: event listeners, intervals, animation frames in effect teardown
- Use `clog()` from `src/util.rs` for debug logging (respects DEBUG_LOG flag)

## Performance Considerations

- Practical grid target: ≤ 2048² tiles (typical runs << 512²)
- A\* pathfinding is O(V log V):
  - For large grids, consider early-exit A\* or incremental repairs
  - Only recompute path when terrain affecting path changes
- Simulation tick should remain under ~2ms for 60 FPS headroom
- If >1k enemies: batch collision checks or use spatial indexing

## Testing Strategy

Focus on pure logic extraction:
- **Pathfinding**: Validate connectivity after permissible changes; reject blocking
- **Upgrade math**: Verify stacking (additive/multiplicative) yields correct stats
- **Tower DPS**: Baseline damage + crit path correctness
- **Mining progression**: Hardness scaling & cancellation logic
- **Enemy scaling**: Formula stability (regression tests)

Use small synthetic grids (5×5, 9×9) for path tests.

**Future**: Introduce `RngLike` trait for deterministic test mode with seeded RNG.

## Common Pitfalls

- Forgetting path recompute after mining or wall placement
- Duplicate intervals after hot reload (ensure cleanup in effect teardown)
- Mining continuing while paused (check is_paused flag)
- Large allocations per frame (Vec re-init loops)
- Adding enum variant without updating serialization defaults / match arms

## Visual Style

Based on GitHub dark palette:
- Background: `#0e1116`
- Panels: `#161b22`
- Borders: `#30363d`
- Accents: `#58a6ff` (blue), `#2ea043` (green), `#f85149` (red), `#d29922` (amber)
- Overlays: 1px borders, 8–14px border radius, semi-opaque backgrounds

## Planned Features

See **FEATURES_TODO.md** for a comprehensive, prioritized list of planned features and improvements. This file tracks:
- Features marked as "todo" in upgrade descriptions
- Partially implemented systems that need completion
- New features from the design document
- Estimated implementation time for each feature

**Quick wins** (< 1 hour each):
- Boost frequency upgrades
- Tower refund percentage
- Mining crit system

## License

MIT (see LICENSE file)
