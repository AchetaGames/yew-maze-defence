# Copilot Instructions for Maze Defence (v0.2)

Purpose: Guide AI code suggestions to align with this project's architecture, style, gameplay design, and clarified constraints.

## 1. Project Snapshot
- Name: Maze Defence (Rust + Yew, browser tower / maze defence + incremental hybrid)
- Target: WebAssembly (wasm32-unknown-unknown) via Trunk
- Core Loop: Mine -> Place walls (reroute path) -> Place/remove towers -> Survive looping enemies -> Run ends -> Spend research -> Retry
- Rendering: Single <canvas> (Canvas 2D) managed in RunView component
- State Layers:
  - RunState: Per-run transient simulation (grid, enemies, towers, gold, life, research gain accumulator, timers, stats)
  - UpgradeState: Persistent meta progression (localStorage)
  - Ephemeral: CameraState, MiningState, TouchState, Interactable mask

## 2. Tech & Dependencies
- yew 0.21 (csr)
- wasm-bindgen, web-sys (granular feature selection: events, canvas, storage, etc.)
- serde / serde_json (persistence of UpgradeState + research)
- js-sys for low-level interop
- No external pathfinding crate (custom A* 4-connected; no diagonals)

## 3. Architectural Patterns
- Yew function components + hooks (use_reducer for RunState; use_state for camera, flags)
- Simulation ticks on fixed interval (~16ms) dispatch RunAction mutations (pure logic cluster)
- requestAnimationFrame strictly for rendering (read-only snapshot usage)
- Event listeners (mouse, wheel, key, touch) manually registered with cleanup on unmount
- Coordinate space: Tile grid integer indices; world units = tile-sized (1.0); entities have continuous f64 positions centered in tiles

## 4. Game Logic Highlights
- Mining: Time-based progress with hardness vs mining_speed. Cancels on pointer exit or pause. Converts Rock/Wall -> Empty; may yield gold or future boosts.
- Path Integrity: Terrain mutations must preserve Start→End connectivity (A* recompute after each mutation; reject blocking changes)
- Towers: Place/remove via hotkey 'T' on Rock. Refund 100% on removal. Costs gold.
- Enemies: Follow current path loop; each completed loop reduces Life. Life <= 0 => game_over halts simulation.
- Upgrades: Modify initial RunState parameters (mining speed, life, spawn scaling, boosts unlock/frequency, etc.)

## 5. File Roles (Key)
- src/main.rs: App root + routing between Run and Upgrades views
- src/components/run_view.rs: Canvas render + input + overlays + simulation tick registration
- src/components/upgrades_view.rs: Upgrade Web (radial clusters, panning, zooming, purchase logic)
- src/components/legend.rs: Dynamic legend tile list
- src/model.rs: Core data (RunState, UpgradeState, enums: UpgradeId, TileKind, TowerKind, actions RunAction, enemy/tower structs)
- src/state/*.rs: Camera, Mining, Touch, Interactable mask helpers
- src/util.rs: Helpers (formatting, clog logging wrapper, small math/color utils if added later)

## 6. Coding Conventions & Style
- Mutate RunState only through reducer dispatch; no interior mutation bypassing actions
- Avoid panic! in wasm callbacks; return early on invalid indices / missing elements
- Exhaustive match on enums; when adding variants audit: rendering, interaction, persistence, upgrades, tests
- Prefer f64 for world math; clamp zoom; line widths inverse to zoom for consistent visual weight
- Keep per-frame allocations minimal (reuse small Vec buffers; short-lived clones of simple structs acceptable)
- Always clean up: listeners, intervals, animation frames in effect teardown

## 7. Adding Features (Guidance)
### New Tower Type
1. Extend TowerKind (model.rs)
2. Add rendering branch (color, glyph, projectile) in run_view.rs
3. Extend firing logic / DPS formula in simulation tick
4. Gate scaling via new UpgradeId if needed
5. Update legend dynamic inclusion logic

### Additional Upgrade
1. Add UpgradeId variant + metadata (name, desc, max, category, cost curve)
2. Integrate effect in initial RunState build OR simulation modifiers
3. Ensure default level = 0 for backward compatibility
4. Place node (x,y) in upgrades_view layout cluster; add dependencies
5. Adjust serialization if structure expanded (non-breaking)

### New TileKind / Boost
1. Add enum variant + color/hatch pattern in rendering
2. Update placement/mining eligibility & path blocking rules
3. Apply boost effects (tower stat modification or global) during simulation
4. Legend inclusion logic

### Pathfinding Changes
- All terrain modifications call a centralized recompute_path(state) -> Option<Path>
- Reject action if recompute returns None (maintain previous state)
- A* currently 4-connected (N,E,S,W). Diagonals not planned; altering would require collision & distance adjustments.

## 8. Performance Considerations
- Practical target max grid: up to 2048 x 2048 (≈ 4.19M tiles) but typical runs expected << 512² for now
- Large grids: A* O(V log V) may become heavy; consider:
  - Early exit A* (stop when reaching target)
  - Caching last path & performing incremental repairs
  - Spatial partition / bounding box when limiting modifications
- Avoid full-path recompute unless terrain affecting path changed
- Keep simulation tick under ~2ms at target scale (baseline) to preserve 60 FPS headroom
- If > 1k enemies concurrently: batch projectile collision or switch to cell-based indexing

## 9. Persistence & Storage
- Persistent keys: md_upgrade_state, md_research
- Run state & live simulation are NOT persisted (ephemeral by design)
- No mid-run save/resume planned (explicit)
- Add new keys with md_ prefix (namespaced); apply defensive parse with fallback default struct

## 10. Input & UX Rules
- Space: Pause/Resume (or dismiss intro / game over overlays contextually)
- T: Place/remove tower at hovered valid Rock tile (auto-unpause on placement only)
- Mining: LMB hold on Rock/Wall; release or move off cancels
- Camera: Wheel zoom (cursor focal point), drag (secondary/middle) or overlay buttons, recenter button
- Touch: touchstart -> mine/pan decision; pinch zoom not implemented yet

## 11. Error Handling / Logging
- Use clog for debug prints
- Future: Introduce log levels (feature flag e.g., console_debug / console_trace) so builds can gate verbosity
- Avoid unwrap() on data influenced by user input or storage; use unwrap_or / if let Some

## 12. Visual / UI Consistency
- Palette base: GitHub dark (bg #0e1116 panels #161b22 accents #58a6ff #2ea043 #f85149 #d29922)
- Overlays: 1px #30363d borders, subtle radius (8–14px), semi-opaque backgrounds
- Maintain accessible contrast; consider optional colorblind-friendly mode (future)

## 13. Security / Safety
- No dynamic HTML injection; plain text only
- Validate JSON on load (ignore malformed & reinitialize)
- Avoid storing sensitive data (only meta progression)

## 14. Testing Strategy
Goal: Maximize deterministic coverage for pure logic while rendering stays observational.
Focus Areas:
1. Pathfinding: Ensure path exists after permissible terrain changes; test blocking attempts rejected
2. Upgrade math: Stacking (additive vs multiplicative future) yields correct modified stats
3. Tower DPS: Baseline damage + crit path correctness (simulate few ticks)
4. Mining progression: Hardness scaling & cancellation logic
5. Enemy scaling: Current formula stable (regression tests protect future refactors)
Approach:
- Extract pure helpers (advance_run, recompute_path, apply_upgrades) enabling unit tests
- Use small synthetic grids (5x5, 9x9) for path tests
- Provide optional deterministic test mode by injecting a seeded RNG wrapper (trait RngLike)
Tooling:
- Standard cargo test for logic
- Potential wasm-bindgen-test for integration (optional later)

## 15. Extensibility Hooks (Suggested)
- Pure function: fn advance_run(state: &mut RunState, dt_ms: f64, rng: &mut impl RngLike)
- Trait TowerBehavior if distinct tower classes proliferate
- CachedPath struct { tiles: Vec<(u16,u16)>, length: f32, last_modified: u64 }
- Feature flags: debug_paths, debug_collisions for conditional drawing

## 16. Common Pitfalls (Avoid)
- Forgetting path recompute after mining completes or wall placement
- Duplicate intervals after hot reload (ensure cleanup) 
- Mining continuing while paused
- Large allocations per frame (Vec re-init loops)
- Adding enum variant without updating serialization default / match arms

## 17. When Unsure, Ask For
- Balance intent (enemy HP/time curves) before large formula rewrites
- Upgrade stacking semantics (additive vs multiplicative) for new effects
- Grid growth expectations before optimizing pathfinding aggressively

## 18. Example Prompt Patterns
- "Add GridExpand2 upgrade increasing max grid dimension soft cap."
- "Implement recompute_path rejecting placements that isolate Start."
- "Refactor tower firing into compute_projectile_spawns(&RunState) -> Vec<Projectile>."
- "Add seeded RNG path for tests with RngLike trait."

## 19. Style Examples
// use_effect_with(dep, move |_| { /* setup */; move || { /* cleanup */ }; });
Render helpers accept (&CanvasRenderingContext2d, &RunState) and avoid allocations.

## 20. Clarified Constraints (Answered)
- Max grid practical bound: Aim ≤ 2048²; typical < 512² now
- Pathfinding: Custom 4-way A*; no diagonals planned
- Determinism: Runs not deterministic (random tile distributions + player actions); optional seed only for tests (future)
- Enemy scaling: Current formula acceptable; future variants (speedy, armored) planned
- Persistence: Only upgrades + research; no mid-run save
- Testing: "As much as possible" – prioritize pure logic extraction
- Logging: clog accepted; add feature-gated log level layering later
- License: MIT (add LICENSE file)

## 21. Pending / Open Items
1. Accessibility: Colorblind palette toggle? Larger font scaling? Keyboard-only navigation completeness
2. Additional meta currencies (beyond placeholder Tile Credits) — confirm or remove placeholder
3. Performance targets: Max concurrent enemies/projectiles baseline (define to guide future optimizations)
4. Replay / export format (if deterministic mode added later)

## 22. License
- Project under MIT (see LICENSE). Ensure new contributions compatible; avoid copyleft code introducing conflicts.

## 23. Next Steps (Internal Roadmap Hooks)
- Introduce RngLike abstraction + deterministic test harness
- Implement path blockage preflight check function (simulate placement -> run A* -> revert)
- Add tower behavior modularization once >3 tower kinds
- Provide debug overlay toggles (fps, entity counts)

## 24. Quick Reference (Cheat Sheet)
Tiles: Rock, Wall (after placement), Empty, Start, End, Boost (future variants)
Hotkeys: Space (Pause), T (Place/Remove Tower), +/- or Wheel (Zoom)
Storage Keys: md_upgrade_state, md_research

---
Revision: v0.2 (clarified constraints + testing & license)
