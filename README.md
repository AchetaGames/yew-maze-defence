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
6. End run (death or manual) to access the Upgrade screen and spend meta-currencies.

## Key Mechanics
### Grid and Pathing
- Grid-based map.
- Initial setup: a short loop near the center (with a slight random offset and randomized entrance/exit orientation); enemies spawn at Start and attempt to loop back to End (completing a lap damages the player/"base").
- Players place walls/tiles to modify enemy pathing. It is not permitted to completely block the path; pathfinding must always have at least one valid route from Start to End.
- Pathfinding: A* or BFS on a 4-connected grid is sufficient. For MVP, we can run pathfinding whenever terrain changes (placement/mining).

### Mining
- Player can "mine" a tile by holding a button to gradually destroy it. Mining time depends on tile hardness and upgrades.
- Some tiles contain gold; some provide tower boosts (e.g., range, damage, fire-rate, slow). This information should be visible prior to mining.
- Mining yields gold and removes the tile (or converts it to a traversable floor), opening up new path possibilities.

### Towers
- Built on traversable tiles using gold.
- Tower types (MVP suggestion):
  - Basic Shooter (single target, moderate fire rate)
  - Slower (applies slow debuff)
- Towers can be reclaimed at 100% refund (MVP), encouraging experimentation.
- Placing on boosted tiles grants permanent bonuses to that tower (e.g., +range/+damage/+fire-rate).

### Enemies & Damage
- Enemies spawn at Start and navigate to End.
- Completing a loop damages the player. Damage per loop and enemy stats scale over time/waves/difficulty ticks.
- Difficulty scales per elapsed time and per number of loops completed.

## Currencies
- Gold (run-only): Earned from mining and possibly from enemies; used to place towers.
- Research Points (meta): Earned per run (based on time survived, loops completed, etc.); used to purchase permanent upgrades.
- Tile Credits (meta/run carryover): Track how many blocks the player mined during a run; in subsequent runs, allow placement of the same number of tiles (or used as a constraint within a run). For MVP, we’ll track blocks mined and map that to allowed placements during the current run.

## Run & Progression
- Runs are endless; track total time survived and max records.
- Player may end a run early to return to Upgrades.
- On death or early end:
  - Convert performance into Research Points.
  - Persist meta-progression (upgrades, records).
  - Reset run currencies (gold) and transient state.

## UI/UX (Initial)
- HUD overlays: Time in the top-center (no label), Resources (Gold, Life, Research) stacked in the top-left; menu buttons (Pause/Resume, Upgrades) stacked in the top-right.
- Center: Game Grid rendered to a canvas; shows enemies, towers, tiles.
- Right Panel: Build menu (towers, walls), tile info/boosts, mining interaction.
- Bottom: Controls (Start/Pause, Speed, End Run).
- Separate "Upgrades" view for meta-progression between runs.

### Controls (MVP, implemented)
- Zoom: Mouse wheel.
- Pan: Middle or Right mouse drag. Right-click context menu is disabled on the canvas.
- Mine: Left mouse button hold on a Rock tile. A progress bar fills inside the tile; moving the cursor off the tile resets progress. On completion the tile becomes Empty and may award gold.
- Pause: Spacebar or the Pause/Resume button in the top-right HUD panel. While paused, the run timer stops and mining is disabled, but you can still pan/zoom to plan. The timer starts automatically on the first mining action.
- On-screen camera controls: bottom-left overlay with Zoom −/+, pan arrows (←↑↓→), and a Center button to re-center on Start. These work while paused.
- Legend: Bottom-right overlay shows only the tile types currently present on the board (Start, Entrance/Exit, Indestructible, Rock, Gold, Empty).
- Tile generation (MVP): Rock vs. Gold is randomized; boost tiles are disabled initially and will unlock later. Gold yield per gold tile is a placeholder value and will scale with upgrades.
- Enemies (MVP): Once the run starts (first mining), enemies spawn near the Entrance every ~2s and follow the current Path to the Exit. Each enemy’s speed and HP are set when it spawns based on elapsed run time and do not change afterwards.

## MVP Scope
- Static grid with initial small loop and Start/End.
- Mining a tile (hold-to-mine) with visible pre-mining info (tooltips/overlay).
- Place walls to divert enemies; enforce path-not-blocked constraint via pathfinding.
- One basic tower type; enemies follow path and tick damage upon loop completion.
- Endless scaling difficulty; record time survived and loops completed.
- Basic Upgrades screen with a handful of upgrades (mining speed, starting gold, tower stat bumps).

## Technical Architecture
- Frontend: Rust + Yew (CSR) rendered to a canvas (HTML5 Canvas 2D via web-sys for drawing). Consider an ECS later; MVP can be struct-based simulation tick.
- State:
  - RunState (transient): grid, enemies, towers, currencies, stats, current path.
  - UpgradeState (meta): purchased upgrades, unlocked content, records.
- Pathfinding: BFS/A* on a 2D array. Recompute on terrain change and periodically as needed.
- Persistence: Store UpgradeState and records in localStorage (via web-sys or gloo-storage). MVP: serialize using JSON.

## Data Model (Initial Draft)
- Position { x, y }, GridSize { width, height }
- TileKind: Empty | Rock { has_gold, boost? } | Wall | Start | End | Direction { dir, role } | Indestructible
  - Direction: a non-interactive tile placed one tile away from Start on two sides to mark the initial maze entrance and exit.
    - dir: Up | Down | Left | Right, indicating arrow direction.
    - role: Entrance | Exit (Entrance arrow points away from Start; Exit arrow points towards Start).
  - Indestructible: non-interactive, unmineable tiles used to enforce a clear entrance/exit.
  - Rock variants for visuals/effects:
    - basic (no boost),
    - gold (has_gold=true, grants gold when mined),
    - cold (boost=Slow),
    - fire (boost=Damage).
- BoostKind: Range | Damage | FireRate | Slow
- Currencies: gold, research, tile_credits
- RunStats: time_survived_secs, loops_completed, blocks_mined
- UpgradeState: mining_speed_level, starting_gold_bonus, tower_refund_rate (100% for MVP)

## Development Roadmap
1. Rendering & Views
   - Basic App with two views: Run and Upgrades.
   - Canvas render loop (tick + draw) with placeholder sprites/colors.
2. Grid & Mining
   - Grid model; mining interaction; reveal and collect resources/boosts.
3. Pathfinding & Constraint
   - BFS/A*; validation preventing fully blocked path; UI feedback when placement is invalid.
4. Towers & Enemies
   - Place/remove towers with cost/refund; enemy movement along current path; combat.
5. Meta & Persistence
   - End run flow; convert stats to research; upgrades; localStorage save/load.

## Notes & Assumptions
- Runs are endless; scaling is time/loop-based for MVP.
- Tile info is visible before mining to encourage planning.
- Full tower refund is intentional to promote experimentation.
- Balance values are placeholders; expect iteration.

## Build & Run
- Dependencies: Rust, Trunk, wasm32-unknown-unknown target.
- Commands:
  - Install target: `rustup target add wasm32-unknown-unknown`
  - Install trunk: `cargo install trunk`
  - Run dev server: `trunk serve --open`

## License
TBD by project owner.
