# COMPONENTS

17 Yew function components. Flat structure (no subdirs). All use `#[function_component]` macro.

## HIERARCHY

```
App (app.rs)                          # Router: View::Run | View::Upgrades
├── RunView (run_view.rs)             # Canvas game view (1.9k lines)
│   ├── StatsPanel                    # Gold, Life, Research, Run ID
│   ├── SecondaryStatsPanel           # Extended stats (toggle)
│   ├── TimeDisplay                   # Center-top elapsed time
│   ├── ControlsPanel                 # Pause/Resume, Path toggle, Upgrades btn
│   ├── TowerPanel                    # Tower hover feedback
│   ├── TileInfoPanel                 # Tile details on hover
│   ├── CameraControls                # Pan arrows, zoom +/-, center btn
│   ├── LegendPanel → LegendRow      # Dynamic tile type legend
│   ├── IntroOverlay                  # First-run tutorial
│   ├── GameOverOverlay               # Stats + Restart/Upgrades buttons
│   └── SettingsModal                 # Preferences (damage numbers, etc.)
│
└── UpgradesView (upgrades_view.rs)   # Upgrade Web (radial layout)
    └── UpgradeSummaryPanel           # Node card (level/cost/buy btn)
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add game HUD element | `run_view.rs` bottom | Add component + render in RunView html |
| Modify canvas rendering | `run_view.rs` ~line 700+ | `render_game()` closure; drawing order matters |
| Change game loop timing | `run_view.rs` ~line 200+ | `setInterval` for sim tick, `requestAnimationFrame` for render |
| Add overlay/modal | New file + `mod.rs` | Follow GameOverOverlay pattern: show prop + callbacks |
| Modify upgrade purchase | `upgrades_view.rs` | Purchase validation is in `app.rs` callback |
| Change upgrade node card | `upgrade_summary_panel.rs` | 383 lines; category colors, progress bar, buy button |
| Add new HUD stat | `stats_panel.rs` | Props from RunState; add field + render row |

## CONVENTIONS

- **Props pattern**: Components receive data via `#[derive(Properties, PartialEq)]` structs
- **Callbacks up, data down**: Parent owns state; children fire `Callback<T>` props
- **No internal state in overlays**: StatsPanel, LegendPanel, etc. are pure render-from-props
- **Canvas rendering in closures**: `run_view.rs` creates `Closure<dyn FnMut()>` for RAF; drawing helpers take `(&CanvasRenderingContext2d, &RunState)`
- **Manual event listeners**: Registered in `use_effect`, cleaned up in teardown closure. NOT via Yew's `on*` attributes (Canvas 2D requirement)
- **GitHub dark palette**: bg `#0e1116`, panels `#161b22`, borders `#30363d`, accents `#58a6ff`/`#2ea043`/`#f85149`/`#d29922`

## ANTI-PATTERNS

- Adding component without `pub mod` entry in `mod.rs`
- Mutating RunState directly — always dispatch `RunAction` via `run_state.dispatch()`
- Forgetting event listener cleanup → duplicate handlers after hot-reload
- Using `onclick` Yew attribute on canvas — must use manual `addEventListener`
- Large allocations in render closures — reuse buffers, avoid `Vec::new()` per frame

## COMPLEXITY HOTSPOTS

- **run_view.rs** (1971 lines): Game loop setup, 6+ event listeners, full canvas rendering, mining logic, tower interaction. Largest single component.
- **upgrades_view.rs** (770 lines): Radial layout math, Bezier edge drawing, pan/zoom, node positioning.
- **upgrade_summary_panel.rs** (383 lines): Complex conditional rendering per upgrade category/state.
