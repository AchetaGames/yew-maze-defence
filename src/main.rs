#![allow(unused_mut)]
use std::cell::RefCell; // added for RAF id storage
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure; // restored for callbacks
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, KeyboardEvent, TouchEvent,
};
use yew::prelude::*; // added

mod model;
use model::{GridSize, RunAction, RunState, TowerKind, UpgradeState};

fn format_time(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{:01}:{:02}:{:02}", h, m, s)
    } else if m > 0 {
        format!("{:02}:{:02}", m, s)
    } else {
        format!("{}s", s)
    }
}

fn clog(msg: &str) {
    // Debug logging disabled to reduce console spam
    let _ = msg; // keep param to avoid warnings
}

#[derive(PartialEq, Clone)]
enum View {
    Run,
    Upgrades,
}

#[derive(Properties, PartialEq, Clone)]
struct RunViewProps {
    pub run_state: UseReducerHandle<RunState>,
    pub to_upgrades: Callback<()>,
}

#[function_component(RunView)]
fn run_view(props: &RunViewProps) -> Html {
    let canvas_ref = use_node_ref();
    let camera = use_mut_ref(|| Camera::default());
    let mining = use_mut_ref(|| Mining::default());
    let draw_ref = use_mut_ref(|| None::<Rc<dyn Fn()>>); // store current draw closure
    let run_state_ref = use_mut_ref(|| props.run_state.clone()); // NEW: always updated handle
    let show_path = use_state(|| false);
    let show_path_flag = use_mut_ref(|| false);
    let touch_state = use_mut_ref(|| TouchState::default());
    // Tower mode removed: always show placement feedback via hover + hotkey
    let tower_feedback = use_state(|| String::new()); // feedback message for tower placement
    let hover_tile = use_mut_ref(|| (-1_i32, -1_i32));
    let tower_feedback_for_effect = tower_feedback.clone();

    // Redraw + log when show_path toggles (ensures canvas updates even if version not changing)
    {
        let draw_ref = draw_ref.clone();
        let flag = *show_path;
        let show_path_flag_ref = show_path_flag.clone();
        use_effect_with(flag, move |_| {
            *show_path_flag_ref.borrow_mut() = flag;
            if let Some(f) = &*draw_ref.borrow() {
                f();
            }
            || ()
        });
    }

    // Effect: on each version update, refresh run_state_ref to latest handle then redraw
    {
        let run_state_ref = run_state_ref.clone();
        let current_handle = props.run_state.clone();
        let draw_ref_local = draw_ref.clone();
        let version = props.run_state.version;
        use_effect_with(version, move |_| {
            *run_state_ref.borrow_mut() = current_handle.clone();
            if let Some(i) = current_handle.last_mined_idx {
                if i < current_handle.tiles.len() {
                    clog(&format!(
                        "Post-reducer: idx={} kind(now)={:?}",
                        i, current_handle.tiles[i].kind
                    ));
                }
            }
            if let Some(f) = &*draw_ref_local.borrow() {
                f();
            }
            || ()
        });
    }

    {
        let canvas_ref = canvas_ref.clone();
        let camera = camera.clone();
        let run_state = props.run_state.clone();
        let draw_ref_setup = draw_ref.clone();
        let mining_setup = mining.clone();

        use_effect_with((), move |_| {
            // hotkey-based interactions (no tower mode toggle)
            let tower_feedback_handle = tower_feedback_for_effect.clone();
            let window = web_sys::window().expect("no global `window` exists");
            let document = window.document().expect("should have a document on window");
            let canvas: HtmlCanvasElement = canvas_ref
                .cast::<HtmlCanvasElement>()
                .expect("canvas_ref not attached to a canvas element");

            let compute_and_apply_canvas_size = {
                let canvas = canvas.clone();
                let document = document.clone();
                let window = window.clone();
                move || {
                    let nav_height: f64 = document
                        .get_element_by_id("top-bar")
                        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
                        .map(|el| el.client_height() as f64)
                        .unwrap_or(0.0);
                    let width = window
                        .inner_width()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(800.0);
                    let height = window
                        .inner_height()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(600.0)
                        - nav_height;
                    canvas.set_width(width.max(0.0) as u32);
                    canvas.set_height(height.max(0.0) as u32);
                }
            };

            compute_and_apply_canvas_size();

            // Initial centering
            {
                let mut cam = camera.borrow_mut();
                if !cam.initialized {
                    let rs = (*run_state).clone();
                    let gs = rs.grid_size;
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;
                    let mut sx = (gs.width / 2) as u32;
                    let mut sy = (gs.height / 2) as u32;
                    for (i, t) in rs.tiles.iter().enumerate() {
                        if let model::TileKind::Start = t.kind {
                            sx = (i as u32) % gs.width;
                            sy = (i as u32) / gs.width;
                            break;
                        }
                    }
                    let cx = sx as f64 + 0.5;
                    let cy = sy as f64 + 0.5;
                    cam.offset_x = w * 0.5 - scale_px * cx;
                    cam.offset_y = h * 0.5 - scale_px * cy;
                    cam.initialized = true;
                }
            }

            // Build draw closure and store in draw_ref
            let draw_closure: Rc<dyn Fn()> = {
                let canvas = canvas.clone();
                let camera = camera.clone();
                let run_state_ref = run_state_ref.clone();
                let mining = mining_setup.clone();
                let show_path_flag = show_path_flag.clone();
                let hover_tile_draw = hover_tile.clone();
                let tower_feedback_draw = tower_feedback_handle.clone();
                Rc::new(move || {
                    if canvas.is_connected() == false {
                        return;
                    }
                    let ctx = match canvas.get_context("2d").ok().flatten() {
                        Some(c) => c.dyn_into::<CanvasRenderingContext2d>().unwrap(),
                        None => return,
                    };
                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;

                    // Acquire state & camera first
                    let cam = camera.borrow();
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    let rs_handle = run_state_ref.borrow();
                    let rs = (**rs_handle).clone();
                    let show_path_on = *show_path_flag.borrow();
                    // Precompute interactable mask
                    let interact_mask = compute_interactable_mask(&rs);
                    // Clear & set transform (always same background)
                    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok();
                    ctx.set_fill_style_str("#0e1116");
                    ctx.fill_rect(0.0, 0.0, w, h);
                    ctx.set_transform(scale_px, 0.0, 0.0, scale_px, cam.offset_x, cam.offset_y)
                        .ok();
                    let gs = rs.grid_size;
                    ctx.set_fill_style_str("#161b22");
                    ctx.fill_rect(0.0, 0.0, gs.width as f64, gs.height as f64);
                    ctx.set_stroke_style_str("#2f3641");
                    let line_w = (1.0 / scale_px).max(0.001);
                    ctx.set_line_width(line_w);
                    for x in 0..=gs.width {
                        ctx.begin_path();
                        ctx.move_to(x as f64, 0.0);
                        ctx.line_to(x as f64, gs.height as f64);
                        ctx.stroke();
                    }
                    for y in 0..=gs.height {
                        ctx.begin_path();
                        ctx.move_to(0.0, y as f64);
                        ctx.line_to(gs.width as f64, y as f64);
                        ctx.stroke();
                    }
                    let margin = 0.1;
                    for y in 0..gs.height {
                        for x in 0..gs.width {
                            let idx = (y * gs.width + x) as usize;
                            match rs.tiles[idx].kind {
                                model::TileKind::Rock { has_gold, boost } => {
                                    let rx = x as f64 + margin;
                                    let ry = y as f64 + margin;
                                    let rw = 1.0 - 2.0 * margin;
                                    let rh = rw;
                                    let fill = if has_gold {
                                        "#4d3b1f"
                                    } else {
                                        match boost {
                                            Some(model::BoostKind::Slow) => "#203a5a",
                                            Some(model::BoostKind::Damage) => "#5a2320",
                                            _ => "#1d2430",
                                        }
                                    };
                                    ctx.set_fill_style_str(fill);
                                    ctx.fill_rect(rx, ry, rw, rh);
                                    ctx.set_stroke_style_str("#3a4455");
                                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Wall => {
                                    let rx = x as f64 + margin;
                                    let ry = y as f64 + margin;
                                    let rw = 1.0 - 2.0 * margin;
                                    let rh = rw;
                                    ctx.set_fill_style_str("#2a2f38");
                                    ctx.fill_rect(rx, ry, rw, rh);
                                    ctx.set_stroke_style_str("#555e6b");
                                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Start => {
                                    // Uniform path background + start marker
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#082235");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                    // Spawn marker (ringed circle)
                                    let cx = rx + 0.5;
                                    let cy = ry + 0.5;
                                    ctx.begin_path();
                                    ctx.set_fill_style_str("#58a6ff");
                                    ctx.arc(cx, cy, 0.30, 0.0, std::f64::consts::PI * 2.0).ok();
                                    ctx.fill();
                                    ctx.set_stroke_style_str("#1f6feb");
                                    ctx.set_line_width((1.2 / scale_px).max(0.001));
                                    ctx.stroke();
                                }
                                model::TileKind::Direction { dir, role } => {
                                    // Uniform path background + directional arrow overlay
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#082235");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                    let color = match role {
                                        model::DirRole::Entrance => "#2ea043",
                                        model::DirRole::Exit => "#f0883e",
                                    };
                                    ctx.set_fill_style_str(color);
                                    ctx.begin_path();
                                    match dir {
                                        model::ArrowDir::Right => {
                                            ctx.move_to(rx + 0.25, ry + 0.20);
                                            ctx.line_to(rx + 0.25, ry + 0.80);
                                            ctx.line_to(rx + 0.80, ry + 0.50);
                                        }
                                        model::ArrowDir::Left => {
                                            ctx.move_to(rx + 0.75, ry + 0.20);
                                            ctx.line_to(rx + 0.75, ry + 0.80);
                                            ctx.line_to(rx + 0.20, ry + 0.50);
                                        }
                                        model::ArrowDir::Up => {
                                            ctx.move_to(rx + 0.20, ry + 0.75);
                                            ctx.line_to(rx + 0.80, ry + 0.75);
                                            ctx.line_to(rx + 0.50, ry + 0.20);
                                        }
                                        model::ArrowDir::Down => {
                                            ctx.move_to(rx + 0.20, ry + 0.25);
                                            ctx.line_to(rx + 0.80, ry + 0.25);
                                            ctx.line_to(rx + 0.50, ry + 0.80);
                                        }
                                    }
                                    ctx.close_path();
                                    ctx.fill();
                                }
                                model::TileKind::Indestructible => {
                                    let rx = x as f64 + margin;
                                    let ry = y as f64 + margin;
                                    let rw = 1.0 - 2.0 * margin;
                                    let rh = rw;
                                    ctx.set_fill_style_str("#3c4454");
                                    ctx.fill_rect(rx, ry, rw, rh);
                                    ctx.set_stroke_style_str("#596273");
                                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Empty => {
                                    // Use a slightly lighter tone to differentiate mined tiles clearly
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#082235"); // higher contrast empty
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                }
                                _ => {}
                            }
                            // Fog-of-war overlay for non-interactable tiles
                            if !interact_mask[idx] {
                                ctx.set_fill_style_str("rgba(0,0,0,0.35)");
                                ctx.fill_rect(x as f64, y as f64, 1.0, 1.0);
                            }
                        }
                    }
                    // enemies simple circles
                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                    for e in &rs.enemies {
                        let radius = 0.28 * e.radius_scale;
                        ctx.begin_path();
                        ctx.set_fill_style_str("#00eaff");
                        ctx.arc(e.x, e.y, radius, 0.0, std::f64::consts::PI * 2.0)
                            .ok();
                        ctx.fill();
                        ctx.set_stroke_style_str("#a80032");
                        ctx.stroke();
                    }
                    // draw towers (after enemies so bodies overlay)
                    for tw in &rs.towers {
                        let cx = tw.x as f64 + 0.5;
                        let cy = tw.y as f64 + 0.5;
                        ctx.begin_path();
                        let color = match tw.kind {
                            TowerKind::Basic => "#ffd700",
                            TowerKind::Slow => "#2ea043",
                            TowerKind::Damage => "#f85149",
                        };
                        ctx.set_fill_style_str(color);
                        ctx.arc(cx, cy, 0.30, 0.0, std::f64::consts::PI * 2.0).ok();
                        ctx.fill();
                        ctx.set_stroke_style_str("#111821");
                        ctx.stroke();
                    }
                    // projectiles
                    if !rs.projectiles.is_empty() {
                        ctx.set_fill_style_str("#fffb");
                        for p in &rs.projectiles {
                            ctx.begin_path();
                            ctx.arc(p.x, p.y, 0.08, 0.0, std::f64::consts::PI * 2.0)
                                .ok();
                            ctx.fill();
                        }
                    }
                    let m = mining.borrow();
                    if m.active && m.mouse_down {
                        if m.tile_x >= 0
                            && m.tile_y >= 0
                            && (m.tile_x as u32) < gs.width
                            && (m.tile_y as u32) < gs.height
                        {
                            let idx = (m.tile_y as u32 * gs.width + m.tile_x as u32) as usize;
                            if matches!(
                                rs.tiles[idx].kind,
                                model::TileKind::Rock { .. } | model::TileKind::Wall
                            ) {
                                let rx = m.tile_x as f64 + margin;
                                let ry = m.tile_y as f64
                                    + margin
                                    + (1.0 - 2.0 * margin) * (1.0 - m.progress.clamp(0.0, 1.0));
                                let rw = 1.0 - 2.0 * margin;
                                let rh = (1.0 - 2.0 * margin) * m.progress.clamp(0.0, 1.0);
                                ctx.set_fill_style_str("rgba(46,160,67,0.7)");
                                ctx.fill_rect(rx, ry, rw, rh);
                            }
                        }
                    }
                    // Optional path visualization: simple polyline only
                    if show_path_on {
                        let path_for_draw: Vec<model::Position> = if !rs.path_loop.is_empty() {
                            rs.path_loop.clone()
                        } else {
                            rs.path.clone()
                        };
                        if path_for_draw.is_empty() {
                            // Optional small notice (can be removed if not desired)
                            ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok();
                            ctx.set_fill_style_str("rgba(255,80,80,0.9)");
                            ctx.set_font("12px sans-serif");
                            ctx.fill_text("No path", 10.0, 40.0).ok();
                            ctx.set_transform(
                                scale_px,
                                0.0,
                                0.0,
                                scale_px,
                                cam.offset_x,
                                cam.offset_y,
                            )
                            .ok();
                        } else if path_for_draw.len() >= 2 {
                            ctx.set_stroke_style_str("#ff66ff");
                            ctx.set_line_width((2.5 / scale_px).max(0.002));
                            ctx.begin_path();
                            for (i, node) in path_for_draw.iter().enumerate() {
                                let cx = node.x as f64 + 0.5;
                                let cy = node.y as f64 + 0.5;
                                if i == 0 {
                                    ctx.move_to(cx, cy);
                                } else {
                                    ctx.line_to(cx, cy);
                                }
                            }
                            ctx.stroke();
                        }
                    }
                    // Tower placement hover highlight (always active for feedback)
                    let (hx, hy) = *hover_tile_draw.borrow();
                    if hx >= 0 && hy >= 0 {
                        let gs = rs.grid_size;
                        if (hx as u32) < gs.width && (hy as u32) < gs.height {
                            let idx = (hy as u32 * gs.width + hx as u32) as usize;
                            let interact_ok = interact_mask[idx];
                            // Build tuple (color_opt, msg, show_range)
                            let (color_opt, msg, show_range) = if !interact_ok {
                                (Some("rgba(90,90,90,0.35)"), "Out of reach".to_string(), false)
                            } else if rs.is_paused || rs.game_over {
                                (Some("rgba(110,118,129,0.35)"), "Paused".to_string(), false)
                            } else if !matches!(rs.tiles[idx].kind, model::TileKind::Rock { .. }) {
                                (Some("rgba(248,81,73,0.45)"), "Need Rock".to_string(), false)
                            } else if rs.towers.iter().any(|t| t.x == hx as u32 && t.y == hy as u32) {
                                (Some("rgba(219,109,40,0.55)"), "T: remove tower".to_string(), true)
                            } else if rs.currencies.gold < rs.tower_cost {
                                (Some("rgba(248,81,73,0.45)"), format!("Need {} gold", rs.tower_cost), false)
                            } else {
                                (Some("rgba(46,160,67,0.45)"), format!("T: place ({}g)", rs.tower_cost), true)
                            };
                            if let Some(c) = color_opt { ctx.set_fill_style_str(c); ctx.fill_rect(hx as f64, hy as f64, 1.0, 1.0); }
                            if show_range {
                                ctx.begin_path();
                                ctx.set_line_width((1.0 / scale_px).max(0.001));
                                ctx.set_stroke_style_str("rgba(56,139,253,0.5)");
                                ctx.arc(hx as f64 + 0.5, hy as f64 + 0.5, rs.tower_base_range, 0.0, std::f64::consts::PI * 2.0).ok();
                                ctx.stroke();
                            }
                            if *tower_feedback_draw != msg { tower_feedback_draw.set(msg); }
                        }
                    }
                })
            };
            *draw_ref_setup.borrow_mut() = Some(draw_closure.clone());

            // Initial draw
            (draw_closure)();

            // Animation frame loop to ensure redraws even when run not started (e.g., for path toggle)
            let raf_id = Rc::new(RefCell::new(None));
            {
                let raf_id_clone = raf_id.clone();
                let draw_ref_loop = draw_ref_setup.clone();
                let window_loop = window.clone();
                let closure_cell: Rc<RefCell<Option<Closure<dyn FnMut()>>>> =
                    Rc::new(RefCell::new(None));
                let closure_cell_clone = closure_cell.clone();
                *closure_cell.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                    if let Some(f) = &*draw_ref_loop.borrow() {
                        f();
                    }
                    // schedule next frame
                    if let Ok(id) = window_loop.request_animation_frame(
                        closure_cell_clone
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unchecked_ref(),
                    ) {
                        *raf_id_clone.borrow_mut() = Some(id);
                    }
                })
                    as Box<dyn FnMut()>));
                // kick off
                if let Ok(id) = window.request_animation_frame(
                    closure_cell
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                ) {
                    *raf_id.borrow_mut() = Some(id);
                }
                // store closure_cell & raf_id in canvas dataset? not needed; captured by cleanup
                // Add to cleanup below
            }

            // Mining tick
            let mining_tick = {
                // CHANGED: use run_state_ref for fresh state each tick
                let run_state_ref_ct = run_state_ref.clone();
                let mining = mining_setup.clone();
                Closure::wrap(Box::new(move || {
                    let mut m = mining.borrow_mut();
                    if !m.active || !m.mouse_down {
                        return;
                    }
                    let handle = run_state_ref_ct.borrow().clone();
                    let rs_snap = (*handle).clone();
                    if rs_snap.is_paused {
                        return;
                    }
                    let gs = rs_snap.grid_size;
                    if m.tile_x < 0
                        || m.tile_y < 0
                        || (m.tile_x as u32) >= gs.width
                        || (m.tile_y as u32) >= gs.height
                    {
                        m.active = false;
                        return;
                    }
                    let idx = (m.tile_y as u32 * gs.width + m.tile_x as u32) as usize;
                    if matches!(
                        rs_snap.tiles[idx].kind,
                        model::TileKind::Rock { .. } | model::TileKind::Wall
                    ) {
                        m.elapsed_secs += 0.016;
                        m.progress = (m.elapsed_secs / m.required_secs).min(1.0);
                        if m.progress >= 1.0 {
                            clog(&format!(
                                "MiningComplete at idx={} kind(before)={:?}",
                                idx, rs_snap.tiles[idx].kind
                            ));
                            // drop borrow before dispatch
                            drop(m);
                            handle.dispatch(RunAction::MiningComplete { idx });
                            let mut m2 = mining.borrow_mut();
                            m2.active = false;
                            m2.mouse_down = false;
                            m2.progress = 0.0;
                            m2.elapsed_secs = 0.0;
                        } else if !rs_snap.started {
                            drop(m);
                            handle.dispatch(RunAction::StartRun);
                        }
                    } else {
                        m.active = false;
                        m.mouse_down = false;
                    }
                }) as Box<dyn FnMut()>)
            };
            let mining_tick_id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    mining_tick.as_ref().unchecked_ref(),
                    16,
                )
                .unwrap();

            // Simulation tick (enemy movement & spawning)
            let sim_tick = {
                // CHANGED: use run_state_ref
                let run_state_ref_ct = run_state_ref.clone();
                Closure::wrap(Box::new(move || {
                    let handle = run_state_ref_ct.borrow().clone();
                    handle.dispatch(RunAction::SimTick { dt: 0.016 });
                }) as Box<dyn FnMut()>)
            };
            let sim_tick_id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    sim_tick.as_ref().unchecked_ref(),
                    16,
                )
                .unwrap();

            // Wheel
            let wheel_cb = {
                let camera = camera.clone();
                let draw_ref = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
                    e.prevent_default();
                    let mut cam = camera.borrow_mut();
                    let tile_px = 32.0;
                    let canvas_x = e.offset_x() as f64;
                    let canvas_y = e.offset_y() as f64;
                    let old_scale = cam.zoom * tile_px;
                    let world_x = (canvas_x - cam.offset_x) / old_scale;
                    let world_y = (canvas_y - cam.offset_y) / old_scale;
                    let delta = e.delta_y();
                    let zoom_change = (-delta * 0.001).exp();
                    cam.zoom = (cam.zoom * zoom_change).clamp(0.2, 5.0);
                    let new_scale = cam.zoom * tile_px;
                    cam.offset_x = canvas_x - world_x * new_scale;
                    cam.offset_y = canvas_y - world_y * new_scale;
                    drop(cam);
                    if let Some(f) = &*draw_ref.borrow() {
                        f();
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback("wheel", wheel_cb.as_ref().unchecked_ref())
                .unwrap();

            // Keydown listener for tower hotkey 'T'
            let keydown_cb = {
                let run_state_ref_ct = run_state_ref.clone();
                let hover_ref = hover_tile.clone();
                let tower_feedback_hotkey = tower_feedback_handle.clone();
                let draw_ref_k = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                    if e.key() == "t" || e.key() == "T" {
                        e.prevent_default();
                        let (hx, hy) = *hover_ref.borrow();
                        if hx < 0 || hy < 0 { return; }
                        let handle = run_state_ref_ct.borrow().clone();
                        let rs = (*handle).clone();
                        let gs = rs.grid_size;
                        if (hx as u32) >= gs.width || (hy as u32) >= gs.height { return; }
                        let interact_mask = compute_interactable_mask(&rs);
                        let idx = (hy as u32 * gs.width + hx as u32) as usize;
                        if !interact_mask[idx] { tower_feedback_hotkey.set("Out of reach".into()); return; }
                        if rs.is_paused || rs.game_over { tower_feedback_hotkey.set("Paused".into()); return; }
                        let idx2 = idx; // reuse
                        use web_sys::console;
                        if let model::TileKind::Rock { .. } = rs.tiles[idx2].kind {
                            let has_t = rs.towers.iter().any(|t| t.x == hx as u32 && t.y == hy as u32);
                            if has_t {
                                console::log_1(&format!("Hotkey: removing tower at ({},{})", hx, hy).into());
                                handle.dispatch(RunAction::RemoveTower { x: hx as u32, y: hy as u32 });
                                tower_feedback_hotkey.set("Tower removed".into());
                            } else if rs.currencies.gold < rs.tower_cost {
                                console::log_1(&format!("Hotkey: insufficient gold (have {}, need {})", rs.currencies.gold, rs.tower_cost).into());
                                tower_feedback_hotkey.set(format!("Need {} gold", rs.tower_cost));
                            } else {
                                console::log_1(&format!("Hotkey: placing tower at ({},{}) cost {}", hx, hy, rs.tower_cost).into());
                                handle.dispatch(RunAction::PlaceTower { x: hx as u32, y: hy as u32 });
                                tower_feedback_hotkey.set("Tower placed".into());
                            }
                        } else {
                            console::log_1(&format!("Hotkey: invalid tile kind for tower at ({},{})", hx, hy).into());
                            tower_feedback_hotkey.set("Need Rock".into());
                        }
                        if let Some(f) = &*draw_ref_k.borrow() { f(); }
                    }
                }) as Box<dyn FnMut(_)> )
            };
            window
                .add_event_listener_with_callback("keydown", keydown_cb.as_ref().unchecked_ref())
                .ok();

            // Mouse down (removed tower_mode logic, only mining & wall now)
            let mousedown_cb = {
                let camera = camera.clone();
                let mining = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let draw_ref = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    let button = e.button();
                    if button == 0 {
                        let cam = camera.borrow_mut();
                        let tile_px = 32.0; let scale_px = cam.zoom * tile_px;
                        let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                        let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                        drop(cam);
                        let handle = run_state_ref_ct.borrow().clone();
                        let rs = (*handle).clone();
                        if rs.is_paused { return; }
                        let gs = rs.grid_size;
                        let tx = world_x.floor() as i32; let ty = world_y.floor() as i32;
                        if tx >= 0 && ty >= 0 && (tx as u32) < gs.width && (ty as u32) < gs.height {
                            let idx = (ty as u32 * gs.width + tx as u32) as usize;
                            let interact_mask = compute_interactable_mask(&rs);
                            if !interact_mask[idx] { return; }
                            match rs.tiles[idx].kind {
                                model::TileKind::Rock { .. } | model::TileKind::Wall => {
                                    let has_tower_here = rs.towers.iter().any(|t| t.x == tx as u32 && t.y == ty as u32);
                                    if !has_tower_here {
                                        if !rs.started { handle.dispatch(RunAction::StartRun); }
                                        let mut m = mining.borrow_mut();
                                        m.tile_x = tx; m.tile_y = ty;
                                        let hardness = rs.tiles[idx].hardness.max(1) as f64;
                                        let spd = rs.mining_speed.max(0.0001);
                                        m.required_secs = hardness / spd;
                                        m.elapsed_secs = 0.0; m.progress = 0.0; m.active = true; m.mouse_down = true;
                                    }
                                }
                                model::TileKind::Empty => {
                                    // allow placing wall only if interactable (already true)
                                    let mut m = mining.borrow_mut();
                                    m.active = false; m.mouse_down = false; m.progress = 0.0; m.elapsed_secs = 0.0;
                                    handle.dispatch(RunAction::PlaceWall { x: tx as u32, y: ty as u32 });
                                }
                                _ => {}
                            }
                        }
                    } else {
                        let mut cam = camera.borrow_mut();
                        cam.panning = true; cam.last_x = e.client_x() as f64; cam.last_y = e.client_y() as f64;
                    }
                    if let Some(f) = &*draw_ref.borrow() { f(); }
                }) as Box<dyn FnMut(_)> )
            };
            canvas
                .add_event_listener_with_callback(
                    "mousedown",
                    mousedown_cb.as_ref().unchecked_ref(),
                )
                .unwrap();

            // Mouse move (updates hover tile, handles panning & mining retarget)
            let mousemove_cb = {
                let camera = camera.clone();
                let mining = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let draw_ref = draw_ref_setup.clone();
                let hover_tile_move = hover_tile.clone();
                Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    let mut cam = camera.borrow_mut();
                    if cam.panning {
                        let x = e.client_x() as f64;
                        let y = e.client_y() as f64;
                        let dx = x - cam.last_x;
                        let dy = y - cam.last_y;
                        cam.last_x = x;
                        cam.last_y = y;
                        cam.offset_x += dx;
                        cam.offset_y += dy;
                        drop(cam);
                        if let Some(f) = &*draw_ref.borrow() {
                            f();
                        }
                        return;
                    }
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                    let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                    drop(cam);
                    let tx = world_x.floor() as i32;
                    let ty = world_y.floor() as i32;
                    *hover_tile_move.borrow_mut() = (tx, ty);
                    {
                        let mut m = mining.borrow_mut();
                        if m.mouse_down && m.active {
                            let handle = run_state_ref_ct.borrow().clone();
                            let rs = (*handle).clone();
                            if rs.is_paused {
                                m.active = false;
                                m.mouse_down = false;
                            } else {
                                let gs = rs.grid_size;
                                if tx >= 0
                                    && ty >= 0
                                    && (tx as u32) < gs.width
                                    && (ty as u32) < gs.height
                                {
                                    let idx = (ty as u32 * gs.width + tx as u32) as usize;
                                    match rs.tiles[idx].kind {
                                        model::TileKind::Rock { .. } | model::TileKind::Wall => {
                                            if tx != m.tile_x || ty != m.tile_y {
                                                m.tile_x = tx;
                                                m.tile_y = ty;
                                                let hardness = rs.tiles[idx].hardness.max(1) as f64;
                                                let spd = rs.mining_speed.max(0.0001);
                                                m.required_secs = hardness / spd;
                                                m.elapsed_secs = 0.0;
                                                m.progress = 0.0;
                                            }
                                        }
                                        _ => {
                                            m.active = false;
                                            m.mouse_down = false;
                                        }
                                    }
                                } else {
                                    m.active = false;
                                    m.mouse_down = false;
                                }
                            }
                        }
                    }
                    if let Some(f) = &*draw_ref.borrow() {
                        f();
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback(
                    "mousemove",
                    mousemove_cb.as_ref().unchecked_ref(),
                )
                .unwrap();

            // Mouse up
            let mouseup_cb = {
                let camera = camera.clone();
                let mining = mining_setup.clone();
                let draw_ref = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                    let mut cam = camera.borrow_mut();
                    cam.panning = false;
                    drop(cam);
                    let mut m = mining.borrow_mut();
                    m.mouse_down = false;
                    m.active = false;
                    m.progress = 0.0;
                    m.elapsed_secs = 0.0;
                    drop(m);
                    if let Some(f) = &*draw_ref.borrow() {
                        f();
                    }
                }) as Box<dyn FnMut(_)>)
            };
            window
                .add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref())
                .unwrap();

            // Context menu disable
            let contextmenu_cb = {
                Closure::wrap(Box::new(move |e: web_sys::Event| {
                    e.prevent_default();
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback(
                    "contextmenu",
                    contextmenu_cb.as_ref().unchecked_ref(),
                )
                .unwrap();

            // Resize handler
            let resize_cb = {
                let compute_and_apply_canvas_size = compute_and_apply_canvas_size.clone();
                let draw_ref = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    compute_and_apply_canvas_size();
                    if let Some(f) = &*draw_ref.borrow() {
                        f();
                    }
                }) as Box<dyn FnMut(_)>)
            };
            window
                .add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref())
                .unwrap();

            // Touch events (retain previous mobile support)
            let touch_start_cb = {
                let canvas_tc = canvas.clone();
                let camera_tc = camera.clone();
                let mining_tc = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let touch_state_tc = touch_state.clone();
                Closure::wrap(Box::new(move |e: TouchEvent| {
                    if let Some(t0) = e.touches().item(0) {
                        let rect = canvas_tc.get_bounding_client_rect();
                        let cx = t0.client_x() as f64 - rect.left();
                        let cy = t0.client_y() as f64 - rect.top();
                        let mut cam = camera_tc.borrow_mut();
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = (cx - cam.offset_x) / scale_px;
                        let world_y = (cy - cam.offset_y) / scale_px;
                        let mut ts = touch_state_tc.borrow_mut();
                        ts.last_touch_x = cx;
                        ts.last_touch_y = cy;
                        ts.single_active = true;
                        ts.pinch = false;
                        drop(ts);
                        let handle = run_state_ref_ct.borrow().clone();
                        let rs_snap = (*handle).clone();
                        if !rs_snap.is_paused && e.touches().length() == 1 {
                            let gs = rs_snap.grid_size;
                            let tx = world_x.floor() as i32;
                            let ty = world_y.floor() as i32;
                            if tx >= 0
                                && ty >= 0
                                && (tx as u32) < gs.width
                                && (ty as u32) < gs.height
                            {
                                let idx = (ty as u32 * gs.width + tx as u32) as usize;
                                match rs_snap.tiles[idx].kind {
                                    model::TileKind::Rock { .. } | model::TileKind::Wall => {
                                        if !rs_snap.started {
                                            handle.dispatch(RunAction::StartRun);
                                        }
                                        let mut m = mining_tc.borrow_mut();
                                        let hardness = rs_snap.tiles[idx].hardness.max(1) as f64;
                                        let spd = rs_snap.mining_speed.max(0.0001);
                                        m.tile_x = tx;
                                        m.tile_y = ty;
                                        m.required_secs = hardness / spd;
                                        m.elapsed_secs = 0.0;
                                        m.progress = 0.0;
                                        m.active = true;
                                        m.mouse_down = true;
                                    }
                                    model::TileKind::Empty => {
                                        handle.dispatch(RunAction::PlaceWall {
                                            x: tx as u32,
                                            y: ty as u32,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback(
                    "touchstart",
                    touch_start_cb.as_ref().unchecked_ref(),
                )
                .ok();

            let touch_move_cb = {
                let canvas_tc = canvas.clone();
                let camera_tc = camera.clone();
                let mining_tc = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let touch_state_tc = touch_state.clone();
                Closure::wrap(Box::new(move |e: TouchEvent| {
                    let touches = e.touches();
                    if touches.length() == 0 {
                        e.prevent_default();
                        return;
                    }
                    let rect = canvas_tc.get_bounding_client_rect();
                    let tile_px = 32.0;
                    if touches.length() == 1 {
                        if let Some(t0) = touches.item(0) {
                            let cx = t0.client_x() as f64 - rect.left();
                            let cy = t0.client_y() as f64 - rect.top();
                            let handle = run_state_ref_ct.borrow().clone();
                            let rs_snap = (*handle).clone();
                            if rs_snap.is_paused {
                                e.prevent_default();
                                return;
                            }
                            let mut cam = camera_tc.borrow_mut();
                            let scale_px = cam.zoom * tile_px;
                            let world_x = (cx - cam.offset_x) / scale_px;
                            let world_y = (cy - cam.offset_y) / scale_px;
                            drop(cam);
                            let tx = world_x.floor() as i32;
                            let ty = world_y.floor() as i32;
                            let mut m = mining_tc.borrow_mut();
                            if m.active && m.mouse_down {
                                let gs = rs_snap.grid_size;
                                if tx >= 0
                                    && ty >= 0
                                    && (tx as u32) < gs.width
                                    && (ty as u32) < gs.height
                                {
                                    let idx = (ty as u32 * gs.width + tx as u32) as usize;
                                    match rs_snap.tiles[idx].kind {
                                        model::TileKind::Rock { .. } | model::TileKind::Wall => {
                                            if tx != m.tile_x || ty != m.tile_y {
                                                m.tile_x = tx;
                                                m.tile_y = ty;
                                                let hardness =
                                                    rs_snap.tiles[idx].hardness.max(1) as f64;
                                                let spd = rs_snap.mining_speed.max(0.0001);
                                                m.required_secs = hardness / spd;
                                                m.elapsed_secs = 0.0;
                                                m.progress = 0.0;
                                            }
                                        }
                                        _ => {
                                            m.active = false;
                                            m.mouse_down = false;
                                        }
                                    }
                                } else {
                                    m.active = false;
                                    m.mouse_down = false;
                                }
                            } else {
                                let mut cam2 = camera_tc.borrow_mut();
                                let mut ts = touch_state_tc.borrow_mut();
                                if ts.single_active {
                                    let dx = cx - ts.last_touch_x;
                                    let dy = cy - ts.last_touch_y;
                                    cam2.offset_x += dx;
                                    cam2.offset_y += dy;
                                    ts.last_touch_x = cx;
                                    ts.last_touch_y = cy;
                                }
                            }
                        }
                    }
                    // pinch zoom omitted for brevity (can add later)
                    e.prevent_default();
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback(
                    "touchmove",
                    touch_move_cb.as_ref().unchecked_ref(),
                )
                .ok();

            let touch_end_cb = {
                let camera_tc = camera.clone();
                let mining_tc = mining_setup.clone();
                let touch_state_tc = touch_state.clone();
                Closure::wrap(Box::new(move |e: TouchEvent| {
                    if e.touches().length() == 0 {
                        {
                            let mut ts = touch_state_tc.borrow_mut();
                            ts.single_active = false;
                            ts.pinch = false;
                        }
                        {
                            let mut cam = camera_tc.borrow_mut();
                            cam.panning = false;
                        }
                        {
                            let mut m = mining_tc.borrow_mut();
                            m.active = false;
                            m.mouse_down = false;
                            m.progress = 0.0;
                            m.elapsed_secs = 0.0;
                        }
                    }
                    e.prevent_default();
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback("touchend", touch_end_cb.as_ref().unchecked_ref())
                .ok();
            canvas
                .add_event_listener_with_callback(
                    "touchcancel",
                    touch_end_cb.as_ref().unchecked_ref(),
                )
                .ok();
            // Provide cleanup for all listeners & intervals
            let window_clone = window.clone();
            move || {
                let _ = canvas.remove_event_listener_with_callback(
                    "wheel",
                    wheel_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "mousedown",
                    mousedown_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "mousemove",
                    mousemove_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "contextmenu",
                    contextmenu_cb.as_ref().unchecked_ref(),
                );
                let _ = window_clone.remove_event_listener_with_callback(
                    "mouseup",
                    mouseup_cb.as_ref().unchecked_ref(),
                );
                let _ = window_clone.remove_event_listener_with_callback(
                    "resize",
                    resize_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "touchstart",
                    touch_start_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "touchmove",
                    touch_move_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "touchend",
                    touch_end_cb.as_ref().unchecked_ref(),
                );
                let _ = canvas.remove_event_listener_with_callback(
                    "touchcancel",
                    touch_end_cb.as_ref().unchecked_ref(),
                );
                let _ = window_clone.remove_event_listener_with_callback(
                    "keydown",
                    keydown_cb.as_ref().unchecked_ref(),
                );
                window_clone.clear_interval_with_handle(mining_tick_id);
                window_clone.clear_interval_with_handle(sim_tick_id);
                if let Some(id) = *raf_id.borrow() {
                    let _ = window_clone.cancel_animation_frame(id);
                }
                // Keep closures (mining_tick, sim_tick, etc.) in scope until here so they aren't dropped early.
                let _keep_alive = (
                    &mining_tick,
                    &sim_tick,
                    &wheel_cb,
                    &mousedown_cb,
                    &mousemove_cb,
                    &mouseup_cb,
                    &touch_start_cb,
                    &touch_move_cb,
                    &touch_end_cb,
                );
            }
        });
    }
    // Recenter automatically when run_id changes (after a reset)
    {
        let camera_ref = camera.clone();
        let run_state_handle = props.run_state.clone();
        let canvas_ref_local = canvas_ref.clone();
        let run_id_dependency = props.run_state.run_id;
        use_effect_with(run_id_dependency, move |_| {
            let rs = (*run_state_handle).clone();
            // find new Start
            let mut sx = (rs.grid_size.width / 2) as u32;
            let mut sy = (rs.grid_size.height / 2) as u32;
            for (i, t) in rs.tiles.iter().enumerate() {
                if let model::TileKind::Start = t.kind { sx = (i as u32)%rs.grid_size.width; sy = (i as u32)/rs.grid_size.width; break; }
            }
            if let Some(canvas) = canvas_ref_local.cast::<HtmlCanvasElement>() {
                let w = canvas.width() as f64; let h = canvas.height() as f64;
                let mut cam = camera_ref.borrow_mut();
                let tile_px = 32.0; let scale_px = cam.zoom * tile_px;
                cam.offset_x = w * 0.5 - scale_px * (sx as f64 + 0.5);
                cam.offset_y = h * 0.5 - scale_px * (sy as f64 + 0.5);
                cam.initialized = true;
            } else {
                // log_dbg(&format!("[run-id-center] canvas not ready run_id={}", rs.run_id));
            }
            || ()
        });
    }
    // Recenter & reset zoom when game over triggers
    {
        let camera_ref = camera.clone();
        let run_state_handle = props.run_state.clone();
        let canvas_ref_local = canvas_ref.clone();
        let game_over_dep = props.run_state.game_over;
        use_effect_with(game_over_dep, move |go| {
            if *go {
                let rs = (*run_state_handle).clone();
                let mut sx = (rs.grid_size.width / 2) as u32; let mut sy = (rs.grid_size.height / 2) as u32;
                for (i, t) in rs.tiles.iter().enumerate() { if let model::TileKind::Start = t.kind { sx = (i as u32)%rs.grid_size.width; sy=(i as u32)/rs.grid_size.width; break; } }
                if let Some(canvas) = canvas_ref_local.cast::<HtmlCanvasElement>() {
                    let w = canvas.width() as f64; let h = canvas.height() as f64;
                    let mut cam = camera_ref.borrow_mut();
                    cam.zoom = 2.5; let tile_px=32.0; let scale_px = cam.zoom * tile_px;
                    cam.offset_x = w*0.5 - scale_px*(sx as f64 + 0.5);
                    cam.offset_y = h*0.5 - scale_px*(sy as f64 + 0.5);
                    cam.initialized = true;
                }
            }
            || ()
        });
    }

    // Overlay controls & legend
    let rs_snapshot = (*props.run_state).clone();
    let mut has_basic = false;
    let mut has_gold = false;
    let mut has_empty = false;
    let mut has_start = false;
    let mut has_entrance = false;
    let mut has_exit = false;
    let mut has_indestructible = false;
    for t in &rs_snapshot.tiles {
        match &t.kind {
            model::TileKind::Rock { has_gold: hg, .. } => {
                if *hg {
                    has_gold = true;
                } else {
                    has_basic = true;
                }
            }
            model::TileKind::Empty => has_empty = true,
            model::TileKind::Start => has_start = true,
            model::TileKind::Direction { role, .. } => match role {
                model::DirRole::Entrance => has_entrance = true,
                model::DirRole::Exit => has_exit = true,
            },
            model::TileKind::Indestructible => has_indestructible = true,
            _ => {}
        }
    }

    // HUD overlay values and callbacks
    let rs_overlay = (*props.run_state).clone();
    let gold_ov = rs_overlay.currencies.gold;
    let research_ov = rs_overlay.currencies.research;
    let life_ov = rs_overlay.life;
    let time_ov = rs_overlay.stats.time_survived_secs;
    let paused_ov = rs_overlay.is_paused;
    let game_over = rs_overlay.game_over; // new
    let enemy_count = rs_overlay.enemies.len(); // debug
    let path_len = if !rs_overlay.path_loop.is_empty() {
        rs_overlay.path_loop.len()
    } else {
        rs_overlay.path.len()
    }; // debug loop length
    let pause_label_rv = if paused_ov {
        if game_over {
            "Game Over"
        } else {
            "Resume (Space)"
        }
    } else {
        "Pause (Space)"
    };
    let toggle_pause_rv = {
        let run_state = props.run_state.clone();
        Callback::from(move |_: yew::events::MouseEvent| {
            if !run_state.game_over {
                run_state.dispatch(RunAction::TogglePause);
            }
        })
    };
    let restart_cb = {
        let run_state = props.run_state.clone();
        Callback::from(move |_: yew::events::MouseEvent| {
            run_state.dispatch(RunAction::ResetRun);
        })
    };
    let to_upgrades_click = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_: yew::events::MouseEvent| cb.emit(()))
    };

    // Camera control callbacks
    let zoom_in = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        Callback::from(move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let mut cam = camera.borrow_mut();
                let tile_px = 32.0;
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                let cx = w * 0.5;
                let cy = h * 0.5;
                let old_scale = cam.zoom * tile_px;
                let world_x = (cx - cam.offset_x) / old_scale;
                let world_y = (cy - cam.offset_y) / old_scale;
                cam.zoom = (cam.zoom * 1.25).clamp(0.2, 5.0);
                let new_scale = cam.zoom * tile_px;
                cam.offset_x = cx - world_x * new_scale;
                cam.offset_y = cy - world_y * new_scale;
            }
            let _ = web_sys::window()
                .unwrap()
                .dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };
    let zoom_out = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        Callback::from(move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let mut cam = camera.borrow_mut();
                let tile_px = 32.0;
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                let cx = w * 0.5;
                let cy = h * 0.5;
                let old_scale = cam.zoom * tile_px;
                let world_x = (cx - cam.offset_x) / old_scale;
                let world_y = (cy - cam.offset_y) / old_scale;
                cam.zoom = (cam.zoom * 0.8).clamp(0.2, 5.0);
                let new_scale = cam.zoom * tile_px;
                cam.offset_x = cx - world_x * new_scale;
                cam.offset_y = cy - world_y * new_scale;
            }
            let _ = web_sys::window()
                .unwrap()
                .dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };
    let pan_by = |dx: f64, dy: f64| {
        let camera = camera.clone();
        Callback::from(move |_| {
            let mut cam = camera.borrow_mut();
            cam.offset_x += dx;
            cam.offset_y += dy;
            drop(cam);
            let _ = web_sys::window()
                .unwrap()
                .dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };
    let center_on_start = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        let run_state = props.run_state.clone();
        Callback::from(move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                let rs = (*run_state).clone();
                let gs = rs.grid_size;
                let mut cam = camera.borrow_mut();
                let tile_px = 32.0;
                let scale_px = cam.zoom * tile_px;
                // find Start tile; fallback to grid center
                let mut sx = (gs.width / 2) as u32;
                let mut sy = (gs.height / 2) as u32;
                for (i, t) in rs.tiles.iter().enumerate() {
                    if let model::TileKind::Start = t.kind {
                        sx = (i as u32) % gs.width;
                        sy = (i as u32) / gs.width;
                        break;
                    }
                }
                let cx = sx as f64 + 0.5;
                let cy = sy as f64 + 0.5;
                cam.offset_x = w * 0.5 - scale_px * cx;
                cam.offset_y = h * 0.5 - scale_px * cy;
                drop(cam);
                let _ = web_sys::window()
                    .unwrap()
                    .dispatch_event(&web_sys::Event::new("resize").unwrap());
            }
        })
    };

    let path_debug_text = if *show_path {
        let rsd = (*props.run_state).clone();
        let source = if !rsd.path_loop.is_empty() {
            &rsd.path_loop
        } else {
            &rsd.path
        };
        if source.is_empty() {
            "(empty)".to_string()
        } else {
            let mut s = String::new();
            for (i, p) in source.iter().enumerate() {
                if i > 0 {
                    s.push_str(" -> ");
                }
                s.push_str(&format!("({},{})", p.x, p.y));
                if i > 14 {
                    s.push_str(" ...");
                    break;
                }
            }
            s
        }
    } else {
        String::new()
    };
    let path_nodes_style = if *show_path {
        "font-size:11px; opacity:0.7;"
    } else {
        "font-size:11px; opacity:0.7; display:none;"
    };

    html! {
        <div style="position:relative; width:100vw; height:100vh;">
            <canvas ref={canvas_ref.clone()} id="game-canvas" style="display:block; width:100%; height:100%;"></canvas>
            <div style="position:absolute; top:12px; left:50%; transform:translateX(-50%); font-size:20px; font-weight:600;">
                { format_time(time_ov) }
            </div>
            <div style="position:absolute; top:12px; left:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:180px; display:flex; flex-direction:column; gap:6px;">
                <div>{ format!("Gold: {}", gold_ov) }</div>
                <div>{ format!("Life: {}", life_ov) }</div>
                <div>{ format!("Research: {}", research_ov) }</div>
                <div style="font-size:11px; opacity:0.7;">{ format!("Run: {}", rs_overlay.run_id) }</div>
                <div style="font-size:11px; opacity:0.7;">{ format!("Enemies: {}", enemy_count) }</div>
                <div style="font-size:11px; opacity:0.7;">{ format!("Path: {}", path_len) }</div>
                <div style={path_nodes_style.to_string()}>{ format!("PathNodes: {}", path_debug_text) }</div>
            </div>
            <div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:200px; display:flex; flex-direction:column; gap:6px;">
                <button onclick={toggle_pause_rv.clone()}>{ pause_label_rv }</button>
                <button onclick={ {
                    let show_path = show_path.clone();
                    Callback::from(move |_| show_path.set(!*show_path))
                } }>{ if *show_path { "Hide Path" } else { "Show Path" } }</button>
                <button onclick={to_upgrades_click.clone()}>{"Upgrades"}</button>
                <div style="font-size:11px; opacity:0.7;">{"Hotkey: 'T' place/remove tower"}</div>
                { if !tower_feedback.is_empty() { html!{ <div style="font-size:11px; line-height:1.2; background:#1c2128; border:1px solid #30363d; padding:4px 6px; border-radius:6px;">{ (*tower_feedback).clone() }</div> } } else { html!{} } }
            </div>
            <div style="position:absolute; left:12px; bottom:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; display:flex; gap:6px; align-items:center;">
                <button onclick={zoom_out.clone()}>{"-"}</button>
                <button onclick={zoom_in.clone()}>{"+"}</button>
                <span style="width:8px;"></span>
                <button onclick={pan_by(-64.0, 0.0)}>{"←"}</button>
                <button onclick={pan_by(0.0, -64.0)}>{"↑"}</button>
                <button onclick={pan_by(0.0, 64.0)}>{"↓"}</button>
                <button onclick={pan_by(64.0, 0.0)}>{"→"}</button>
                <span style="width:8px;"></span>
                <button onclick={center_on_start.clone()}>{"Center"}</button>
            </div>
            <div style="position:absolute; right:12px; bottom:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:160px;">
                <div style="font-weight:600; margin-bottom:6px;">{"Legend"}</div>
                { if has_start { html!{ <LegendRow color="#58a6ff" label="Start" /> } } else { html!{} } }
                { if has_entrance { html!{ <LegendRow color="#2ea043" label="Entrance" /> } } else { html!{} } }
                { if has_exit { html!{ <LegendRow color="#f0883e" label="Exit" /> } } else { html!{} } }
                { if has_indestructible { html!{ <LegendRow color="#3c4454" label="Indestructible" /> } } else { html!{} } }
                { if has_basic { html!{ <LegendRow color="#1d2430" label="Rock" /> } } else { html!{} } }
                { if has_gold { html!{ <LegendRow color="#4d3b1f" label="Gold Rock" /> } } else { html!{} } }
                { if has_empty { html!{ <LegendRow color="#082235" label="Path" /> } } else { html!{} } }
            </div>
            { if game_over {
                html! { <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); background:rgba(0,0,0,0.85); border:2px solid #f85149; padding:24px 32px; border-radius:12px; text-align:center; min-width:320px;">
                    <h2 style="margin:0 0 12px 0; color:#f85149;">{"Game Over"}</h2>
                    <p style="margin:4px 0;">{ format!("Time Survived: {}", format_time(time_ov)) }</p>
                    <p style="margin:4px 0;">{ format!("Loops Completed: {}", rs_overlay.stats.loops_completed) }</p>
                    <p style="margin:4px 0;">{ format!("Blocks Mined: {}", rs_overlay.stats.blocks_mined) }</p>
                    <div style="margin-top:16px; display:flex; gap:12px; justify-content:center;">
                        <button onclick={restart_cb.clone()}> {"Restart Run"} </button>
                        <button onclick={to_upgrades_click.clone()}> {"Upgrades"} </button>
                    </div>
                </div> }
            } else { html! {} } }
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct LegendRowProps {
    pub color: &'static str,
    pub label: &'static str,
}

#[function_component(LegendRow)]
fn legend_row(props: &LegendRowProps) -> Html {
    html! {
        <div style="display:flex; align-items:center; gap:8px; margin:3px 0;">
            <span style={format!("display:inline-block; width:12px; height:12px; background:{}; border:1px solid #30363d; border-radius:2px;", props.color)}></span>
            <span>{ props.label }</span>
        </div>
    }
}

struct Camera {
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    panning: bool,
    last_x: f64,
    last_y: f64,
    initialized: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 2.5,
            offset_x: 0.0,
            offset_y: 0.0,
            panning: false,
            last_x: 0.0,
            last_y: 0.0,
            initialized: false,
        }
    }
}

#[derive(Default)]
struct Mining {
    tile_x: i32,
    tile_y: i32,
    required_secs: f64,
    elapsed_secs: f64,
    progress: f64,
    active: bool,
    mouse_down: bool,
}

#[derive(Default)]
struct TouchState {
    single_active: bool,
    pinch: bool,
    _start_pinch_dist: f64,
    _start_zoom: f64,
    _world_center_x: f64,
    _world_center_y: f64,
    last_touch_x: f64,
    last_touch_y: f64,
}

#[function_component(App)]
fn app() -> Html {
    let view = use_state(|| View::Run);
    let run_state = use_reducer(|| {
        RunState::new_basic(GridSize {
            width: 25,
            height: 25,
        })
    });
    let _upgrade_state = use_state(|| UpgradeState {
        tower_refund_rate_percent: 100,
        ..Default::default()
    });
    let last_resources = use_mut_ref(|| (0u64, 0u64, 0u32));

    {
        // Ticker for run time
        let run_state = run_state.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let run_state2 = run_state.clone();
            let tick = Closure::wrap(Box::new(move || {
                run_state2.dispatch(RunAction::TickSecond);
            }) as Box<dyn FnMut()>);
            let id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    tick.as_ref().unchecked_ref(),
                    1000,
                )
                .unwrap();
            let key_cb = {
                let run_state = run_state.clone();
                Closure::wrap(Box::new(move |e: KeyboardEvent| {
                    if e.code() == "Space" {
                        e.prevent_default();
                        run_state.dispatch(RunAction::TogglePause);
                    }
                }) as Box<dyn FnMut(_)>)
            };
            window
                .add_event_listener_with_callback("keydown", key_cb.as_ref().unchecked_ref())
                .unwrap();
            move || {
                let _ = window.clear_interval_with_handle(id);
                let _ = window.remove_event_listener_with_callback(
                    "keydown",
                    key_cb.as_ref().unchecked_ref(),
                );
                drop(key_cb);
                drop(tick);
            }
        });
    }

    {
        // Log resource changes
        let run_state = run_state.clone();
        let last_resources = last_resources.clone();
        use_effect_with(
            (
                (*run_state).currencies.gold,
                (*run_state).currencies.research,
                (*run_state).life,
            ),
            move |deps| {
                let (g, r, l) = *deps;
                let mut prev = last_resources.borrow_mut();
                if prev.0 != g {
                    clog(&format!("gold: {} -> {}", prev.0, g));
                }
                if prev.1 != r {
                    clog(&format!("research: {} -> {}", prev.1, r));
                }
                if prev.2 != l {
                    clog(&format!("life: {} -> {}", prev.2, l));
                }
                *prev = (g, r, l);
                || ()
            },
        );
    }

    let to_run = {
        let view = view.clone();
        Callback::from(move |_| view.set(View::Run))
    };
    let to_upgrades = {
        let view = view.clone();
        Callback::from(move |_| view.set(View::Upgrades))
    };

    html! {
        <div id="root">
            {
                match (*view).clone() {
                    View::Run => html! { <RunView run_state={run_state.clone()} to_upgrades={to_upgrades.clone()} /> },
                    View::Upgrades => html! {
                        <div style="position:relative; width:100vw; height:100vh;">
                            <div id="upgrades-view" style="padding: 12px;">
                                <h2>{"Upgrades"}</h2>
                                <p>{"Spend research to improve mining speed, starting gold, tower stats, etc. (coming soon)"}</p>
                            </div>
                            <div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px;">
                                <button onclick={to_run.clone()}>{"Back to Run"}</button>
                            </div>
                        </div>
                    }
                }
            }
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

// New: compute interactable (frontier) mask: path tiles + their 4-neighbors
fn compute_interactable_mask(rs: &RunState) -> Vec<bool> {
    use std::collections::VecDeque;
    let gs = rs.grid_size;
    let tile_count = rs.tiles.len();
    let mut mask = vec![false; tile_count]; // final interactable tiles (empties + frontier rocks/walls/start/direction)
    let mut reachable = vec![false; tile_count]; // reachable empty/start/direction tiles via flood fill

    // Helper: index from (x,y)
    let idx_of = |x: u32, y: u32| -> usize { (y * gs.width + x) as usize };
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;

    // Seed queue with path tiles (rs.path_loop if available else rs.path) that are Empty or Start/Direction.
    let mut q: VecDeque<(u32,u32)> = VecDeque::new();
    let mut enqueue = |x: u32, y: u32, reachable: &mut Vec<bool>, q: &mut VecDeque<(u32,u32)>| {
        let i = idx_of(x,y);
        if !reachable[i] { reachable[i] = true; q.push_back((x,y)); }
    };
    let seeds: Vec<model::Position> = if !rs.path_loop.is_empty() { rs.path_loop.clone() } else { rs.path.clone() };
    for p in &seeds {
        if p.x < gs.width && p.y < gs.height {
            let i = idx_of(p.x, p.y);
            match rs.tiles[i].kind {
                model::TileKind::Empty | model::TileKind::Start | model::TileKind::Direction { .. } => enqueue(p.x, p.y, &mut reachable, &mut q),
                _ => {}
            }
        }
    }
    // Fallback: if seeds empty, try Start tile
    if q.is_empty() {
        for (i,t) in rs.tiles.iter().enumerate() { if matches!(t.kind, model::TileKind::Start) { let x = (i as u32)%gs.width; let y = (i as u32)/gs.width; enqueue(x,y,&mut reachable,&mut q); break; } }
    }

    // Flood-fill through Empty tiles only (and keep Start/Direction reachable but do not traverse through walls/rocks)
    let dirs = [(1i32,0i32),(-1,0),(0,1),(0,-1)];
    while let Some((x,y)) = q.pop_front() {
        let i = idx_of(x,y);
        // Mark as interactable (reachable floor / start / direction tiles)
        mask[i] = true;
        for (dx,dy) in dirs { let nx = x as i32 + dx; let ny = y as i32 + dy; if !in_bounds(nx,ny) { continue; } let ux = nx as u32; let uy = ny as u32; let ni = idx_of(ux,uy); match rs.tiles[ni].kind { model::TileKind::Empty => { if !reachable[ni] { reachable[ni] = true; q.push_back((ux,uy)); } }, model::TileKind::Start | model::TileKind::Direction { .. } => { if !reachable[ni] { reachable[ni] = true; q.push_back((ux,uy)); } }, _ => {} } }
    }

    // Frontier: any Rock or Wall adjacent to a reachable tile becomes interactable
    for y in 0..gs.height { for x in 0..gs.width { let i = idx_of(x,y); match rs.tiles[i].kind { model::TileKind::Rock { .. } | model::TileKind::Wall => { // check neighbors
                let mut adj_reachable = false; for (dx,dy) in dirs { let nx = x as i32 + dx; let ny = y as i32 + dy; if in_bounds(nx,ny) { let ni = idx_of(nx as u32, ny as u32); if reachable[ni] { adj_reachable = true; break; } } }
                if adj_reachable { mask[i] = true; }
            }, _ => {} } } }

    mask
}
