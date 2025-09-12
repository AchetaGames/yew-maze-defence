# Maze Defence (Yew + Rust) — Game Design Document (GDD)

## Overview
Maze Defence is a 2D grid-based tower defence and incremental hybrid running in the browser, built with Rust and Yew, bundled by Trunk. Players construct mazes by placing blocks that divert enemies. Enemies start at a defined point and try to complete loops; each completed loop damages the player. Losing is part of progression—upon death or early ending a run, players spend meta-currencies to improve future runs.

Goals:
- Create a satisfying loop of mining, building, and defending.
- Encourage iterative improvement via upgrades and multiple currencies.
- Runs are endless (increasing difficulty) with strong record keeping.

## Core Loop
1. Start a run with a small initial loop in the grid’s center (start and end points included).
2. Mine tiles to gain gold and reveal/harvest special boosts.
3. Place tiles/walls to alter the maze path (cannot fully block the path).
4. Build towers using gold; towers can be reclaimed at full refund.
5. Survive as long as possible while enemies scale in difficulty.
6. End run (death or manual) to access the Upgrade Web and spend meta-currencies.

## Key Mechanics
### Grid and Pathing
- Grid-based map.
- Initial setup: a short loop near the center (with a slight random offset and randomized entrance/exit orientation); enemies spawn at Start and attempt to loop back to End (completing a lap damages the player/base).
- Players place walls/tiles to modify enemy pathing. It is not permitted to completely block the path; pathfinding must always have at least one valid route from Start to End.
- Pathfinding: A* on a 4‑connected grid; re‑computed on terrain changes.

### Mining
- Hold LMB on a Rock or Wall tile to mine it; progress bar animates inside the tile.
- Mining speed scales with upgrades; tile hardness determines total required time.
- Rock variants may contain gold or (once unlocked) boosts.

### Towers
- Build on Rock tiles (after verifying rules) using gold. (Current: hotkey driven – see Controls.)
- Towers can be removed (refunded) via hotkey interaction on the same tile.
- Boost tiles (once unlocked) give attribute modifiers when a tower sits on them.

### Enemies & Damage
- Enemies spawn from the Start/Entrance loop region and traverse the current loop path.
- Completing a loop subtracts Life; when Life hits 0 the run ends (Game Over overlay).
- Enemy stats scale with elapsed time and loops.

## Currencies
- Gold (run‑only): Mining & bounties; used for tower placement.
- Research (meta): Earned per run; spent on Upgrade Web purchases; persists via localStorage.
- (Planned/Placeholder) Tile Credits: counted via blocks mined; may govern free placements in future iterations.

## Run & Meta Progression
- Endless scaling; track time survived, loops, blocks mined.
- On death or manual exit: convert performance -> Research; keep UpgradeState.
- Upgrades permanently (meta) enhance base parameters or future world generation.

## UI / HUD Summary
- Time: Center top (large, minimal UI clutter).
- Left top panel: Gold, Life, Research, Run id, debugging path stats.
- Right top panel: Pause/Resume (or Game Over), Path toggle (Show/Hide Path), Upgrades button, contextual tower hover feedback line.
- Bottom left: Camera controls (Zoom − / +, directional pan arrows, Center on Start).
- Bottom right: Dynamic tile legend (only shows tile types present) plus colors.
- Game Over modal: key stats + quick actions (Restart Run / Upgrades).

## Controls (Implemented)
- Pan: Drag with secondary/middle mouse OR arrow buttons in overlay.
- Zoom: Mouse wheel (focus‑point zoom). Buttons: − / + in bottom-left.
- Center: Button to recenter on Start spawn cluster.
- Mine: Hold LMB on Rock / Wall; move off tile to cancel.
- Tower Place / Remove: Hover a Rock tile and press `T`.
  - Shows contextual hover highlight (green = can place, red = invalid, orange = tower present, gray = out of reach or paused).
  - If a tower exists at the hover tile: `T` removes it (full refund per MVP design).
  - If no tower and requirements met: `T` places a tower.
- Pause/Resume: Spacebar or on‑screen button.
- Path Visualization: Toggle Show/Hide Path; draws simplified polyline or reports (empty) if none.

## New: Upgrade Web (Replaces Linear Tree)
Instead of a strict top‑down depth tree, upgrades are organized into a radial/web layout with **four color‑coded categories** placed around a virtual origin. This improves readability and reduces edge overlap.

### Categories
| Category | Color | Position | Focus |
|----------|-------|----------|-------|
| Health | #2ea043 (green) | Up (north) | Survivability (Max Life, Regeneration) |
| Mining / Gold | #d29922 (amber) | Down (south) | Resource generation, map expansion |
| Damage / Offense | #f85149 (red) | Right (east) | Direct tower DPS, crits, ramping |
| Boost / Utility | #58a6ff (blue) | Left (west) | Unlocking & improving map boost tiles |

#### Health Upgrades
- Max Life (Health)
- Life Regeneration (LifeRegen)

#### Mining / Gold Upgrades
- Mining Speed (MiningSpeed)
- Gold Bounty (GoldGain)
- Gold Spawn Chance (GoldSpawn)
- Starting Gold (StartingGold)
- Grid Expansion (GridExpand)

#### Damage / Offense Upgrades
- Tower Damage I (TowerDamage)
- Tower Damage II (TowerDamage2)
- Tower Range (TowerRange)
- Fire Rate (FireRate)
- Damage Ramp (DamageRamp)
- Crit Chance (CritChance)
- Crit Damage (CritDamage)

#### Boost / Utility Upgrades
- Unlock Boost Tiles (BoostTilesUnlock)
- Boost Frequency (BoostTileFrequency)
- Boost Diversity (BoostTileDiversity)

### Web Navigation
- **Pan**: Click‑drag anywhere (grab / grabbing cursor states).
- **Zoom**: Mouse wheel (centered under cursor) or category panel +/- buttons.
- **Reset**: Resets zoom to 1.0 and recenters offsets.
- **Center**: Recenter without altering zoom.

### Edge Semantics
Arrows (curved Bézier paths) run from prerequisite to dependent upgrade:
- AnyLevel(X) dependency: Appears once you have at least 1 level in X.
- Maxed(X) dependency: Requires X at max level before target becomes purchasable.

### Node Cards
Each upgrade node shows:
- Name (color‑accented header)
- Description (multi‑line preserved)
- Level progress (current / max)
- Purchase button (“Buy (cost)” or “MAX” when maxed)
- Progress bar at bottom (fill percentage of levels)
- Faded card if locked; stronger gradient when maxed.

### Category Legend
Top-left overlay in the Upgrade view: swatches for each category with names for quick orientation.

### Persistence
- UpgradeState JSON: `localStorage["md_upgrade_state"]`
- Research value: `localStorage["md_research"]`
- (Future) Additional run records may be added under new keys.

## Balance & Design Notes (Current State)
- Most multipliers are % additive stacked (e.g., Mining Speed +15% per level).
- Damage I + Damage II additively influence base tower damage before other effects (simple linear combination now; may revisit with multiplicative layering later).
- Crit Damage multiplies final damage after crit trigger.
- Life Regeneration now correctly belongs to Health category (moved from previous provisional grouping under mining in earlier iteration).
- Grid Expansion influences new run map size prior to generation; thus expansion impacts spawn distribution and potential path complexity.

## Technical Architecture
- Frontend: Rust + Yew (CSR) with Canvas 2D rendering.
- State separation: transient `RunState` vs persistent `UpgradeState`.
- Simulation ticks: fixed timestep ~16ms for mining/progression; animation frame for rendering.
- Hotkey system: Global keydown listener (currently for `T` + Pause via Space).

## Data Model (Snapshot)
See `src/model.rs` for authoritative definitions (UpgradeId, TileKind, RunState, etc.).

## Development Roadmap (Updated Excerpts)
1. Visual polish for Upgrade Web (edge coloring by category, hover details).  
2. Additional tower types influenced by specific boost tiles.  
3. Enemy variety & modifiers; loop-based difficulty scaling curves.  
4. Meta records panel (best time, deepest loop, richest run).  
5. Export/import of meta progression.  
6. Achievements / milestone unlock prompts.

## Recent Additions / Changelog (High Level)
- Introduced **Upgrade Web** with four category clusters (replacing linear tree).
- Added curved dependency edges to reduce overlap.
- Contextual hover feedback & hotkey `T` for tower place/remove (removed separate placement mode UI).
- Path visualization toggle with lightweight polyline.
- Center & camera control overlay; recenter on Game Over.
- Game Over modal with core statistics and quick access to restart & upgrades.

## Build & Run
Dependencies: Rust stable, Trunk, wasm32 target.

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve --open
```

## Troubleshooting
- If canvas appears blank: open devtools console for any panic; ensure trunk compiled latest changes.
- If upgrades fail to appear purchased after refresh: check localStorage keys; clear them to reset.
- Performance dips: ensure browser tab is active; background throttling may slow timers (expected in inactive tabs).

## License
Released under the MIT License. See the LICENSE file for details.
