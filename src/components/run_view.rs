use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, TouchEvent};
use yew::prelude::*;

use crate::model::{self, RunAction, RunState, TowerKind};
use crate::state::{compute_interactable_mask, Camera, Mining, TouchState};
use crate::util::clog;
// Replace direct legend row usage with modular components
use super::{
    camera_controls::CameraControls, controls_panel::ControlsPanel,
    game_over_overlay::GameOverOverlay, intro_overlay::IntroOverlay, legend_panel::LegendPanel,
    settings_modal::SettingsModal, stats_panel::StatsPanel, time_display::TimeDisplay,
};

#[derive(Properties, PartialEq, Clone)]
pub struct RunViewProps {
    pub run_state: UseReducerHandle<RunState>,
    pub to_upgrades: Callback<()>,
    pub restart_run: Callback<()>,
}

#[function_component(RunView)]
pub fn run_view(props: &RunViewProps) -> Html {
    let canvas_ref = use_node_ref();
    let camera = use_mut_ref(|| Camera::default());
    let mining = use_mut_ref(|| Mining::default());
    let draw_ref = use_mut_ref(|| None::<Rc<dyn Fn()>>);
    let run_state_ref = use_mut_ref(|| props.run_state.clone());
    let show_path = use_state(|| {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(store)) = win.local_storage() {
                if let Ok(Some(v)) = store.get_item("md_setting_show_path") {
                    return v == "1" || v == "true";
                }
            }
        }
        false
    });
    let show_path_flag = use_mut_ref(|| false);
    let show_damage_numbers = use_state(|| {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(store)) = win.local_storage() {
                if let Ok(Some(v)) = store.get_item("md_setting_show_damage_numbers") {
                    return !(v == "0" || v == "false");
                }
            }
        }
        true // default ON
    });
    let show_damage_numbers_flag = use_mut_ref(|| true);
    let open_settings = use_state(|| false);
    let touch_state = use_mut_ref(|| TouchState::default());
    let tower_feedback = use_state(|| String::new());
    let hover_tile = use_mut_ref(|| (-1_i32, -1_i32));
    let hover_tile_effect = hover_tile.clone(); // clone for effects to avoid moving original
    let tower_feedback_for_effect = tower_feedback.clone();
    // NEW: intro overlay visibility (persist across sessions)
    let show_intro = {
        let initial = {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    // Show only if key absent
                    store.get_item("md_intro_seen").ok().flatten().is_none()
                } else {
                    true
                }
            } else {
                true
            }
        };
        use_state(|| initial)
    };

    // Effect: toggle path
    {
        let draw_ref = draw_ref.clone();
        let flag = *show_path;
        let show_path_flag_ref = show_path_flag.clone();
        use_effect_with(flag, move |_| {
            *show_path_flag_ref.borrow_mut() = flag;
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    let _ = store.set_item("md_setting_show_path", if flag { "1" } else { "0" });
                }
            }
            if let Some(f) = &*draw_ref.borrow() {
                f();
            }
            || ()
        });
    }
    // Effect: toggle damage numbers
    {
        let draw_ref = draw_ref.clone();
        let flag = *show_damage_numbers;
        let show_damage_numbers_flag_ref = show_damage_numbers_flag.clone();
        use_effect_with(flag, move |_| {
            *show_damage_numbers_flag_ref.borrow_mut() = flag;
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    let _ = store.set_item(
                        "md_setting_show_damage_numbers",
                        if flag { "1" } else { "0" },
                    );
                }
            }
            if let Some(f) = &*draw_ref.borrow() {
                f();
            }
            || ()
        });
    }
    // Effect: update run handle each version
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
    // Main mount effect (events, loops)
    {
        let canvas_ref = canvas_ref.clone();
        let camera = camera.clone();
        let run_state = props.run_state.clone();
        let draw_ref_setup = draw_ref.clone();
        let mining_setup = mining.clone();
        let hover_tile_effect_local = hover_tile_effect.clone();
        // Clone state handles so the originals remain usable in render scope
        let tower_feedback_clone = tower_feedback_for_effect.clone();
        let show_intro_clone = show_intro.clone();
        use_effect_with((), move |_| {
            // Use cloned handles inside effect
            let tower_feedback_handle = tower_feedback_clone.clone();
            let show_intro_handle = show_intro_clone.clone();
            let window = web_sys::window().expect("window");
            let document = window.document().expect("document");
            let canvas: HtmlCanvasElement = canvas_ref.cast::<HtmlCanvasElement>().expect("canvas");
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
            // Initial center
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
            // Draw closure
            let draw_closure: Rc<dyn Fn()> = {
                let canvas = canvas.clone();
                let camera = camera.clone();
                let run_state_ref = run_state_ref.clone();
                let mining = mining_setup.clone();
                let show_path_flag = show_path_flag.clone();
                let show_damage_numbers_flag = show_damage_numbers_flag.clone();
                let hover_tile_draw = hover_tile_effect_local.clone();
                let tower_feedback_draw = tower_feedback_handle.clone();
                Rc::new(move || {
                    if !canvas.is_connected() {
                        return;
                    }
                    let ctx = match canvas.get_context("2d").ok().flatten() {
                        Some(c) => c.dyn_into::<CanvasRenderingContext2d>().unwrap(),
                        None => return,
                    };
                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;
                    let cam = camera.borrow();
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    let rs_handle = run_state_ref.borrow();
                    let rs = (**rs_handle).clone();
                    let show_path_on = *show_path_flag.borrow();
                    let show_damage_nums_on = *show_damage_numbers_flag.borrow();
                    let interact_mask = compute_interactable_mask(&rs);
                    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok();
                    ctx.set_fill_style_str("#0e1116");
                    ctx.fill_rect(0.0, 0.0, w, h);
                    ctx.set_transform(scale_px, 0.0, 0.0, scale_px, cam.offset_x, cam.offset_y)
                        .ok();
                    let gs = rs.grid_size;
                    ctx.set_fill_style_str("#161b22");
                    ctx.fill_rect(0.0, 0.0, gs.width as f64, gs.height as f64);
                    ctx.set_stroke_style_str("#2f3641");
                    let line_w = (1.0f64 / scale_px).max(0.001f64);
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
                                    ctx.set_line_width((1.0f64 / scale_px).max(0.001f64));
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
                                    ctx.set_line_width((1.0f64 / scale_px).max(0.001f64));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Start => {
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#082235");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                    let cx = rx + 0.5;
                                    let cy = ry + 0.5;
                                    ctx.begin_path();
                                    ctx.set_fill_style_str("#58a6ff");
                                    ctx.arc(cx, cy, 0.30, 0.0, std::f64::consts::PI * 2.0).ok();
                                    ctx.fill();
                                    ctx.set_stroke_style_str("#1f6feb");
                                    ctx.set_line_width((1.2f64 / scale_px).max(0.001f64));
                                    ctx.stroke();
                                }
                                model::TileKind::Direction { dir, role } => {
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
                                    ctx.set_line_width((1.0f64 / scale_px).max(0.001f64));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Empty => {
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#082235");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                }
                                _ => {}
                            }
                            if !interact_mask[idx] {
                                ctx.set_fill_style_str("rgba(0,0,0,0.35)");
                                ctx.fill_rect(x as f64, y as f64, 1.0, 1.0);
                            }
                        }
                    }
                    ctx.set_line_width((1.0f64 / scale_px).max(0.001f64));
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
                    if !rs.projectiles.is_empty() {
                        ctx.set_fill_style_str("#fffb");
                        for p in &rs.projectiles {
                            ctx.begin_path();
                            ctx.arc(p.x, p.y, 0.08, 0.0, std::f64::consts::PI * 2.0)
                                .ok();
                            ctx.fill();
                        }
                    }
                    // Damage numbers (floating text)
                    if show_damage_nums_on && !rs.damage_numbers.is_empty() {
                        ctx.set_font(&format!("{}px sans-serif", (0.2 / scale_px).max(0.5))); // 10% of prior 6.0 baseline
                        ctx.set_text_align("center");
                        for dn in &rs.damage_numbers {
                            let life_ratio = (dn.ttl / 0.8).clamp(0.0, 1.0);
                            let rise = (0.8 - dn.ttl).max(0.0) * 0.30; // slightly reduced rise for very small text
                            let alpha = life_ratio;
                            ctx.set_fill_style_str(&format!("rgba(255,50,50,{:.3})", alpha)); // red color
                            ctx.fill_text(&dn.amount.to_string(), dn.x, dn.y - rise)
                                .ok();
                        }
                        ctx.set_text_align("start");
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
                    if show_path_on {
                        let path_for_draw: Vec<model::Position> = if !rs.path_loop.is_empty() {
                            rs.path_loop.clone()
                        } else {
                            rs.path.clone()
                        };
                        if path_for_draw.is_empty() {
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
                            ctx.set_line_width((2.5f64 / scale_px).max(0.002f64));
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
                    let (hx, hy) = *hover_tile_draw.borrow();
                    if hx >= 0 && hy >= 0 {
                        if (hx as u32) < gs.width && (hy as u32) < gs.height {
                            let idx = (hy as u32 * gs.width + hx as u32) as usize;
                            let interact_ok = interact_mask[idx];
                            let (color_opt, msg, show_range) = if !interact_ok {
                                (
                                    Some("rgba(90,90,90,0.35)"),
                                    "Out of reach".to_string(),
                                    false,
                                )
                            } else if rs.game_over {
                                // removed rs.is_paused here to allow placement while paused
                                (
                                    Some("rgba(110,118,129,0.35)"),
                                    "Game Over".to_string(),
                                    false,
                                )
                            } else if !matches!(
                                rs.tiles[idx].kind,
                                model::TileKind::Rock { .. } | model::TileKind::Wall
                            ) {
                                (
                                    Some("rgba(248,81,73,0.45)"),
                                    "Need Rock/Wall".to_string(),
                                    false,
                                )
                            } else if rs
                                .towers
                                .iter()
                                .any(|t| t.x == hx as u32 && t.y == hy as u32)
                            {
                                (
                                    Some("rgba(219,109,40,0.55)"),
                                    "T: remove tower".to_string(),
                                    true,
                                )
                            } else if rs.currencies.gold < rs.tower_cost {
                                (
                                    Some("rgba(248,81,73,0.45)"),
                                    format!("Need {} gold", rs.tower_cost),
                                    false,
                                )
                            } else {
                                (
                                    Some("rgba(46,160,67,0.45)"),
                                    format!("T: place ({}g)", rs.tower_cost),
                                    true,
                                )
                            };
                            if let Some(c) = color_opt {
                                ctx.set_fill_style_str(c);
                                ctx.fill_rect(hx as f64, hy as f64, 1.0, 1.0);
                            }
                            if show_range {
                                ctx.begin_path();
                                ctx.set_line_width((1.0f64 / scale_px).max(0.001f64));
                                ctx.set_stroke_style_str("rgba(56,139,253,0.5)");
                                ctx.arc(
                                    hx as f64 + 0.5,
                                    hy as f64 + 0.5,
                                    rs.tower_base_range,
                                    0.0,
                                    std::f64::consts::PI * 2.0,
                                )
                                .ok();
                                ctx.stroke();
                            }
                            if *tower_feedback_draw != msg {
                                tower_feedback_draw.set(msg);
                            }
                        }
                    }
                })
            };
            *draw_ref_setup.borrow_mut() = Some(draw_closure.clone());
            (draw_closure)();
            // RAF loop
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
            }
            // Mining interval
            let mining_tick = {
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
                            clog(&format!("MiningComplete idx={}", idx));
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
            // Sim interval
            let sim_tick = {
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
            // Seconds interval
            let second_tick = {
                let run_state_ref_ct = run_state_ref.clone();
                Closure::wrap(Box::new(move || {
                    let handle = run_state_ref_ct.borrow().clone();
                    handle.dispatch(RunAction::TickSecond);
                }) as Box<dyn FnMut()>)
            };
            let second_tick_id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    second_tick.as_ref().unchecked_ref(),
                    1000,
                )
                .unwrap();
            // Wheel zoom
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
            // Keydown + tower hotkey (Space + T)
            let keydown_cb = {
                let run_state_ref_ct = run_state_ref.clone();
                let hover_ref = hover_tile_effect_local.clone();
                let tower_feedback_hotkey = tower_feedback_handle.clone();
                let draw_ref_k = draw_ref_setup.clone();
                let show_intro_handle_k = show_intro_handle.clone();
                Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                    // Spacebar: dismiss intro if showing, else toggle pause
                    let key = e.key();
                    let code = e.code();
                    if code == "Space" || key == " " || key == "Space" || key == "Spacebar" {
                        e.prevent_default();
                        if *show_intro_handle_k {
                            show_intro_handle_k.set(false);
                            if let Some(win) = web_sys::window() {
                                if let Ok(Some(store)) = win.local_storage() {
                                    let _ = store.set_item("md_intro_seen", "1");
                                }
                            }
                            return;
                        }
                        let handle = run_state_ref_ct.borrow().clone();
                        if !handle.game_over {
                            handle.dispatch(RunAction::TogglePause);
                        }
                        return;
                    }
                    // T: place/remove tower
                    if key == "t" || key == "T" {
                        e.prevent_default();
                        let (hx, hy) = *hover_ref.borrow();
                        if hx < 0 || hy < 0 {
                            return;
                        }
                        let handle = run_state_ref_ct.borrow().clone();
                        let rs = (*handle).clone();
                        if rs.game_over {
                            return;
                        }
                        let was_paused = rs.is_paused; // remember paused state
                        let gs = rs.grid_size;
                        if (hx as u32) >= gs.width || (hy as u32) >= gs.height {
                            return;
                        }
                        let interact_mask = compute_interactable_mask(&rs);
                        let idx = (hy as u32 * gs.width + hx as u32) as usize;
                        if !interact_mask[idx] {
                            tower_feedback_hotkey.set("Out of reach".into());
                            return;
                        }
                        if let model::TileKind::Rock { .. } = rs.tiles[idx].kind {
                            let has_t = rs
                                .towers
                                .iter()
                                .any(|t| t.x == hx as u32 && t.y == hy as u32);
                            if has_t {
                                handle.dispatch(RunAction::RemoveTower {
                                    x: hx as u32,
                                    y: hy as u32,
                                });
                                tower_feedback_hotkey.set("Tower removed".into());
                                // Do NOT auto-unpause on removal (spec only asks for placement)
                            } else if rs.currencies.gold < rs.tower_cost {
                                tower_feedback_hotkey.set(format!("Need {} gold", rs.tower_cost));
                            } else {
                                if !rs.started {
                                    handle.dispatch(RunAction::StartRun);
                                }
                                handle.dispatch(RunAction::PlaceTower {
                                    x: hx as u32,
                                    y: hy as u32,
                                });
                                tower_feedback_hotkey.set("Tower placed".into());
                                if was_paused {
                                    handle.dispatch(RunAction::TogglePause);
                                }
                            }
                        } else if let model::TileKind::Wall = rs.tiles[idx].kind {
                            let has_t = rs
                                .towers
                                .iter()
                                .any(|t| t.x == hx as u32 && t.y == hy as u32);
                            if has_t {
                                handle.dispatch(RunAction::RemoveTower {
                                    x: hx as u32,
                                    y: hy as u32,
                                });
                                tower_feedback_hotkey.set("Tower removed".into());
                            } else if rs.currencies.gold < rs.tower_cost {
                                tower_feedback_hotkey.set(format!("Need {} gold", rs.tower_cost));
                            } else {
                                if !rs.started {
                                    handle.dispatch(RunAction::StartRun);
                                }
                                handle.dispatch(RunAction::PlaceTower {
                                    x: hx as u32,
                                    y: hy as u32,
                                });
                                tower_feedback_hotkey.set("Tower placed".into());
                                if was_paused {
                                    handle.dispatch(RunAction::TogglePause);
                                }
                            }
                        } else {
                            tower_feedback_hotkey.set("Need Rock/Wall".into());
                        }
                        if let Some(f) = &*draw_ref_k.borrow() {
                            f();
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };
            window
                .add_event_listener_with_callback("keydown", keydown_cb.as_ref().unchecked_ref())
                .ok();
            // Mouse events
            let mousedown_cb = {
                let camera = camera.clone();
                let mining = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let draw_ref = draw_ref_setup.clone();
                Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    if e.button() == 0 {
                        let cam = camera.borrow_mut();
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                        let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                        drop(cam);
                        let handle = run_state_ref_ct.borrow().clone();
                        let rs = (*handle).clone();
                        if rs.is_paused {
                            return;
                        }
                        let gs = rs.grid_size;
                        let tx = world_x.floor() as i32;
                        let ty = world_y.floor() as i32;
                        if tx >= 0 && ty >= 0 && (tx as u32) < gs.width && (ty as u32) < gs.height {
                            let idx = (ty as u32 * gs.width + tx as u32) as usize;
                            let interact_mask = compute_interactable_mask(&rs);
                            if !interact_mask[idx] {
                                return;
                            }
                            match rs.tiles[idx].kind {
                                model::TileKind::Rock { .. } | model::TileKind::Wall => {
                                    if !rs
                                        .towers
                                        .iter()
                                        .any(|t| t.x == tx as u32 && t.y == ty as u32)
                                    {
                                        if !rs.started {
                                            handle.dispatch(RunAction::StartRun);
                                        }
                                        let mut m = mining.borrow_mut();
                                        m.tile_x = tx;
                                        m.tile_y = ty;
                                        let hardness = rs.tiles[idx].hardness.max(1) as f64;
                                        let spd = rs.mining_speed.max(0.0001);
                                        m.required_secs = hardness / spd;
                                        m.elapsed_secs = 0.0;
                                        m.progress = 0.0;
                                        m.active = true;
                                        m.mouse_down = true;
                                    }
                                }
                                model::TileKind::Empty => {
                                    let mut m = mining.borrow_mut();
                                    m.active = false;
                                    m.mouse_down = false;
                                    m.progress = 0.0;
                                    m.elapsed_secs = 0.0;
                                    handle.dispatch(RunAction::PlaceWall {
                                        x: tx as u32,
                                        y: ty as u32,
                                    });
                                }
                                _ => {}
                            }
                        }
                    } else {
                        let mut cam = camera.borrow_mut();
                        cam.panning = true;
                        cam.last_x = e.client_x() as f64;
                        cam.last_y = e.client_y() as f64;
                    }
                    if let Some(f) = &*draw_ref.borrow() {
                        f();
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas
                .add_event_listener_with_callback(
                    "mousedown",
                    mousedown_cb.as_ref().unchecked_ref(),
                )
                .unwrap();
            let mousemove_cb = {
                let camera = camera.clone();
                let mining = mining_setup.clone();
                let run_state_ref_ct = run_state_ref.clone();
                let draw_ref = draw_ref_setup.clone();
                let hover_tile_move = hover_tile_effect_local.clone();
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
            // Touch
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
                        // Compute world coords (were missing causing compile error)
                        let cam = camera_tc.borrow_mut();
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = (cx - cam.offset_x) / scale_px;
                        let world_y = (cy - cam.offset_y) / scale_px;
                        drop(cam);
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
                            let cam = camera_tc.borrow_mut();
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
            // Cleanup
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
                window_clone.clear_interval_with_handle(second_tick_id);
                if let Some(id) = *raf_id.borrow() {
                    let _ = window_clone.cancel_animation_frame(id);
                }
                let _keep_alive = (
                    &mining_tick,
                    &sim_tick,
                    &second_tick,
                    &wheel_cb,
                    &mousedown_cb,
                    &mousemove_cb,
                    &mouseup_cb,
                    &touch_start_cb,
                    &touch_move_cb,
                    &touch_end_cb,
                    &keydown_cb,
                );
            }
        });
    }
    // reset center on run id change
    {
        let camera_ref = camera.clone();
        let run_state_handle = props.run_state.clone();
        let canvas_ref_local = canvas_ref.clone();
        let run_id_dependency = props.run_state.run_id;
        use_effect_with(run_id_dependency, move |_| {
            let rs = (*run_state_handle).clone();
            let mut sx = (rs.grid_size.width / 2) as u32;
            let mut sy = (rs.grid_size.height / 2) as u32;
            for (i, t) in rs.tiles.iter().enumerate() {
                if let model::TileKind::Start = t.kind {
                    sx = (i as u32) % rs.grid_size.width;
                    sy = (i as u32) / rs.grid_size.width;
                    break;
                }
            }
            if let Some(canvas) = canvas_ref_local.cast::<HtmlCanvasElement>() {
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                let mut cam = camera_ref.borrow_mut();
                let tile_px = 32.0;
                let scale_px = cam.zoom * tile_px;
                cam.offset_x = w * 0.5 - scale_px * (sx as f64 + 0.5);
                cam.offset_y = h * 0.5 - scale_px * (sy as f64 + 0.5);
                cam.initialized = true;
            }
            || ()
        });
    }
    // game over recenter
    {
        let camera_ref = camera.clone();
        let run_state_handle = props.run_state.clone();
        let canvas_ref_local = canvas_ref.clone();
        let game_over_dep = props.run_state.game_over;
        use_effect_with(game_over_dep, move |go| {
            if *go {
                let rs = (*run_state_handle).clone();
                let mut sx = (rs.grid_size.width / 2) as u32;
                let mut sy = (rs.grid_size.height / 2) as u32;
                for (i, t) in rs.tiles.iter().enumerate() {
                    if let model::TileKind::Start = t.kind {
                        sx = (i as u32) % rs.grid_size.width;
                        sy = (i as u32) / rs.grid_size.width;
                        break;
                    }
                }
                if let Some(canvas) = canvas_ref_local.cast::<HtmlCanvasElement>() {
                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;
                    let mut cam = camera_ref.borrow_mut();
                    cam.zoom = 2.5;
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    cam.offset_x = w * 0.5 - scale_px * (sx as f64 + 0.5);
                    cam.offset_y = h * 0.5 - scale_px * (sy as f64 + 0.5);
                    cam.initialized = true;
                }
            }
            || ()
        });
    }

    // snapshot for legend
    let rs_snapshot = (*props.run_state).clone();
    let mut has_basic = false;
    let mut has_gold = false;
    let mut has_empty = false;
    let mut has_start = false;
    let mut has_entrance = false;
    let mut has_exit = false;
    let mut has_indestructible = false;
    let mut has_wall = false;
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
            model::TileKind::Wall => has_wall = true,
            _ => {}
        }
    }

    // Hover tile legend highlight mapping
    let (
        hover_text,
        hl_start,
        hl_entrance,
        hl_exit,
        hl_indestructible,
        hl_basic,
        hl_gold,
        hl_empty,
        hl_wall,
    ) = {
        let (hx, hy) = *hover_tile.borrow();
        if hx >= 0 && hy >= 0 {
            let hx_u = hx as u32;
            let hy_u = hy as u32;
            let gs = rs_snapshot.grid_size;
            if hx_u < gs.width && hy_u < gs.height {
                let idx = (hy_u * gs.width + hx_u) as usize;
                let tile = &rs_snapshot.tiles[idx];
                match &tile.kind {
                    model::TileKind::Start => (
                        Some(format!("({},{}) Start", hx_u, hy_u)),
                        true,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                    ),
                    model::TileKind::Direction { role, .. } => match role {
                        model::DirRole::Entrance => (
                            Some(format!("({},{}) Entrance", hx_u, hy_u)),
                            false,
                            true,
                            false,
                            false,
                            false,
                            false,
                            false,
                            false,
                        ),
                        model::DirRole::Exit => (
                            Some(format!("({},{}) Exit", hx_u, hy_u)),
                            false,
                            false,
                            true,
                            false,
                            false,
                            false,
                            false,
                            false,
                        ),
                    },
                    model::TileKind::Indestructible => (
                        Some(format!("({},{}) Indestructible", hx_u, hy_u)),
                        false,
                        false,
                        false,
                        true,
                        false,
                        false,
                        false,
                        false,
                    ),
                    model::TileKind::Rock { has_gold: hg, .. } => {
                        if *hg {
                            (
                                Some(format!("({},{}) Gold Rock", hx_u, hy_u)),
                                false,
                                false,
                                false,
                                false,
                                false,
                                true,
                                false,
                                false,
                            )
                        } else {
                            (
                                Some(format!("({},{}) Rock", hx_u, hy_u)),
                                false,
                                false,
                                false,
                                false,
                                true,
                                false,
                                false,
                                false,
                            )
                        }
                    }
                    model::TileKind::Empty => (
                        Some(format!("({},{}) Path", hx_u, hy_u)),
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        true,
                        false,
                    ),
                    model::TileKind::Wall => (
                        Some(format!("({},{}) Wall", hx_u, hy_u)),
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        true,
                    ),
                    model::TileKind::End => (
                        Some(format!("({},{}) End", hx_u, hy_u)),
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                    ),
                }
            } else {
                (None, false, false, false, false, false, false, false, false)
            }
        } else {
            (None, false, false, false, false, false, false, false, false)
        }
    };

    let rs_overlay = (*props.run_state).clone();
    let gold_ov = rs_overlay.currencies.gold;
    let research_ov = rs_overlay.currencies.research;
    let life_ov = rs_overlay.life;
    let time_ov = rs_overlay.stats.time_survived_secs;
    let paused_ov = rs_overlay.is_paused;
    let game_over = rs_overlay.game_over;
    let enemy_count = rs_overlay.enemies.len();
    let path_len = if !rs_overlay.path_loop.is_empty() {
        rs_overlay.path_loop.len()
    } else {
        rs_overlay.path.len()
    };
    let pause_label_rv = if paused_ov {
        if game_over {
            "Game Over"
        } else {
            "Resume (Space)"
        }
    } else {
        "Pause (Space)"
    };

    // camera control buttons
    // (refactored to produce Callback<()> for new CameraControls component)
    let zoom_in_cb: Callback<()> = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        Callback::from(move |()| {
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
    let zoom_out_cb: Callback<()> = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        Callback::from(move |()| {
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
    let pan_cb = |dx: f64, dy: f64| {
        let camera = camera.clone();
        Callback::from(move |()| {
            let mut cam = camera.borrow_mut();
            cam.offset_x += dx;
            cam.offset_y += dy;
            drop(cam);
            let _ = web_sys::window()
                .unwrap()
                .dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };
    let center_cb: Callback<()> = {
        let camera = camera.clone();
        let canvas_ref = canvas_ref.clone();
        let run_state = props.run_state.clone();
        Callback::from(move |()| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                let rs = (*run_state).clone();
                let gs = rs.grid_size;
                let mut cam = camera.borrow_mut();
                let tile_px = 32.0;
                let scale_px = cam.zoom * tile_px;
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
            }
            let _ = web_sys::window()
                .unwrap()
                .dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };

    // Pause & path toggle callbacks adapted to unit callbacks for new components
    let toggle_pause_cb: Callback<()> = {
        let run_state = props.run_state.clone();
        Callback::from(move |()| {
            if !run_state.game_over {
                run_state.dispatch(RunAction::TogglePause);
            }
        })
    };
    let toggle_path_cb: Callback<()> = {
        let show_path = show_path.clone();
        Callback::from(move |()| show_path.set(!*show_path))
    };
    let toggle_damage_numbers_cb: Callback<()> = {
        let show_damage_numbers = show_damage_numbers.clone();
        Callback::from(move |()| show_damage_numbers.set(!*show_damage_numbers))
    };
    let open_settings_cb: Callback<()> = {
        let open_settings = open_settings.clone();
        Callback::from(move |()| open_settings.set(true))
    };
    let close_settings_cb: Callback<()> = {
        let open_settings = open_settings.clone();
        Callback::from(move |()| open_settings.set(false))
    };
    // restart & upgrades already callbacks with ()
    let restart_cb_unit: Callback<()> = {
        let restart = props.restart_run.clone();
        Callback::from(move |()| restart.emit(()))
    };
    let to_upgrades_unit: Callback<()> = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |()| cb.emit(()))
    };

    // Path nodes debug string
    let path_debug_text = {
        if !*show_path {
            String::new()
        } else {
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
        }
    };
    // Path nodes text optional
    let path_nodes_text_opt = if *show_path {
        Some(path_debug_text.clone())
    } else {
        None
    };

    // Intro overlay hide callback
    let hide_intro_cb: Callback<()> = {
        let show_intro = show_intro.clone();
        Callback::from(move |()| {
            show_intro.set(false);
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    let _ = store.set_item("md_intro_seen", "1");
                }
            }
        })
    };

    // Help button callback (re-show intro without clearing seen flag)
    let show_help_cb: Callback<()> = {
        let show_intro = show_intro.clone();
        Callback::from(move |()| show_intro.set(true))
    };

    // Tower feedback option
    let tower_feedback_opt = if tower_feedback.is_empty() {
        None
    } else {
        Some((*tower_feedback).clone())
    };

    // Legend component boolean flags already computed

    html! {<div style="position:relative; width:100vw; height:100vh;">
        <canvas ref={canvas_ref.clone()} id="game-canvas" style="display:block; width:100%; height:100%;"></canvas>
        <TimeDisplay time_survived={time_ov} />
        <IntroOverlay show={*show_intro} game_over={game_over} hide_intro={hide_intro_cb} to_upgrades={to_upgrades_unit.clone()} />
        <StatsPanel gold={gold_ov} life={life_ov} research={research_ov} run_id={rs_overlay.run_id} enemy_count={enemy_count} path_len={path_len} path_nodes_text={path_nodes_text_opt} />
        <ControlsPanel pause_label={pause_label_rv.to_string()} on_toggle_pause={toggle_pause_cb} to_upgrades={to_upgrades_unit.clone()} tower_feedback={tower_feedback_opt} on_show_help={show_help_cb} on_open_settings={open_settings_cb} />
        <CameraControls on_zoom_in={zoom_in_cb} on_zoom_out={zoom_out_cb} on_pan_left={pan_cb(-64.0,0.0)} on_pan_right={pan_cb(64.0,0.0)} on_pan_up={pan_cb(0.0,-64.0)} on_pan_down={pan_cb(0.0,64.0)} on_center={center_cb} />
        <LegendPanel has_start={has_start} has_entrance={has_entrance} has_exit={has_exit} has_indestructible={has_indestructible} has_basic={has_basic} has_gold={has_gold} has_empty={has_empty} has_wall={has_wall}
            hover_text={hover_text}
            highlight_start={hl_start}
            highlight_entrance={hl_entrance}
            highlight_exit={hl_exit}
            highlight_indestructible={hl_indestructible}
            highlight_basic={hl_basic}
            highlight_gold={hl_gold}
            highlight_empty={hl_empty}
            highlight_wall={hl_wall}
        />
        <SettingsModal show={*open_settings} on_close={close_settings_cb.clone()} show_path={*show_path} on_toggle_path={toggle_path_cb} show_damage_numbers={*show_damage_numbers} on_toggle_damage_numbers={toggle_damage_numbers_cb} on_restart_run={restart_cb_unit.clone()} />
        <GameOverOverlay show={game_over} time_survived={time_ov} loops_completed={rs_overlay.stats.loops_completed} blocks_mined={rs_overlay.stats.blocks_mined} restart={restart_cb_unit} to_upgrades={to_upgrades_unit} />
    </div> }
}
