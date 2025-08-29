use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, KeyboardEvent};
use yew::prelude::*;
use std::collections::VecDeque;

mod model;
use model::{GridSize, RunState, UpgradeState, Enemy};

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
    web_sys::console::log_1(&JsValue::from_str(msg));
}

fn dir_to_delta(dir: model::ArrowDir) -> (i32, i32) {
    match dir {
        model::ArrowDir::Up => (0, -1),
        model::ArrowDir::Down => (0, 1),
        model::ArrowDir::Left => (-1, 0),
        model::ArrowDir::Right => (1, 0),
    }
}

fn find_entrance_exit(rs: &RunState) -> Option<((i32, i32, model::ArrowDir), (i32, i32, model::ArrowDir))> {
    let gs = rs.grid_size;
    let mut ent: Option<(i32, i32, model::ArrowDir)> = None;
    let mut exit: Option<(i32, i32, model::ArrowDir)> = None;
    for y in 0..gs.height {
        for x in 0..gs.width {
            let idx = (y * gs.width + x) as usize;
            if let model::TileKind::Direction { dir, role } = rs.tiles[idx].kind {
                match role {
                    model::DirRole::Entrance => ent = Some((x as i32, y as i32, dir)),
                    model::DirRole::Exit => exit = Some((x as i32, y as i32, dir)),
                }
            }
        }
    }
    match (ent, exit) {
        (Some(e), Some(x)) => Some((e, x)),
        _ => None,
    }
}

fn compute_path(rs: &RunState) -> Vec<model::Position> {
    use model::TileKind;
    let gs = rs.grid_size;
    let Some(((ex, ey, edir), (xx, xy, xdir))) = find_entrance_exit(rs) else {
        return Vec::new();
    };
    let (edx, edy) = dir_to_delta(edir);
    let (xdx, xdy) = dir_to_delta(xdir);
    let sx = ex + edx; // start from cell beyond entrance arrow
    let sy = ey + edy;
    let tx = xx - xdx; // end at cell before exit arrow (neighbor Empty ensured)
    let ty = xy - xdy;
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    if !in_bounds(sx, sy) || !in_bounds(tx, ty) { return Vec::new(); }
    let sidx = (sy as u32 * gs.width + sx as u32) as usize;
    let tidx = (ty as u32 * gs.width + tx as u32) as usize;
    let is_empty = |idx: usize| matches!(rs.tiles[idx].kind, TileKind::Empty);
    if !is_empty(sidx) || !is_empty(tidx) { return Vec::new(); }

    let mut q: VecDeque<usize> = VecDeque::new();
    let mut visited = vec![false; (gs.width * gs.height) as usize];
    let mut parent: Vec<Option<usize>> = vec![None; (gs.width * gs.height) as usize];
    visited[sidx] = true;
    q.push_back(sidx);
    let dirs = [(1i32,0i32),(-1,0),(0,1),(0,-1)];
    while let Some(idx) = q.pop_front() {
        if idx == tidx { break; }
        let x = (idx as u32 % gs.width) as i32;
        let y = (idx as u32 / gs.width) as i32;
        for (dx,dy) in dirs {
            let nx = x + dx; let ny = y + dy;
            if !in_bounds(nx, ny) { continue; }
            let nidx = (ny as u32 * gs.width + nx as u32) as usize;
            if visited[nidx] { continue; }
            if !is_empty(nidx) { continue; }
            visited[nidx] = true;
            parent[nidx] = Some(idx);
            q.push_back(nidx);
        }
    }
    if !visited[tidx] { return Vec::new(); }
    // reconstruct
    let mut path_rev: Vec<usize> = Vec::new();
    let mut cur = Some(tidx);
    while let Some(ci) = cur {
        path_rev.push(ci);
        cur = parent[ci];
    }
    path_rev.reverse();
    path_rev.into_iter().map(|i| {
        let x = (i as u32 % gs.width) as u32;
        let y = (i as u32 / gs.width) as u32;
        model::Position { x, y }
    }).collect()
}

#[derive(PartialEq, Clone)]
enum View {
    Run,
    Upgrades,
}

#[derive(Properties, PartialEq, Clone)]
struct RunViewProps {
    pub run_state: UseStateHandle<RunState>,
    pub to_upgrades: Callback<()>,
}

#[function_component(RunView)]
fn run_view(props: &RunViewProps) -> Html {
    let canvas_ref = use_node_ref();
    let camera = use_mut_ref(|| Camera::default());
    let mining = use_mut_ref(|| Mining::default());
    // Shared latest RunState snapshot for safe updates across async/event closures
    let rs_ref = use_mut_ref(|| (*props.run_state).clone());
    // Keep rs_ref synchronized with the latest committed state each render
    {
        let mut s = rs_ref.borrow_mut();
        *s = (*props.run_state).clone();
    }

    {
        let canvas_ref = canvas_ref.clone();
        let camera = camera.clone();
        let run_state = props.run_state.clone();
        let rs_ref = rs_ref.clone();

        use_effect_with((), move |_| {
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

            // Initial centering to match the Center button behavior
            {
                let mut cam = camera.borrow_mut();
                if !cam.initialized {
                    let rs = (*run_state).clone();
                    let gs = rs.grid_size;
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;
                    // Find Start tile position; fallback to grid center if not found
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

            let draw = {
                let canvas = canvas.clone();
                let camera = camera.clone();
                let run_state = run_state.clone();
                let mining = mining.clone();
                move || {
                    let ctx = canvas
                        .get_context("2d")
                        .unwrap()
                        .unwrap()
                        .dyn_into::<CanvasRenderingContext2d>()
                        .unwrap();

                    let w = canvas.width() as f64;
                    let h = canvas.height() as f64;
                    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok();
                    ctx.set_fill_style_str("#0e1116");
                    ctx.fill_rect(0.0, 0.0, w, h);

                    let cam = camera.borrow();
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    ctx.set_transform(scale_px, 0.0, 0.0, scale_px, cam.offset_x, cam.offset_y)
                        .ok();

                    let rs = (*run_state).clone();
                    let gs = rs.grid_size;

                    // Grid background
                    ctx.set_fill_style_str("#161b22");
                    ctx.fill_rect(0.0, 0.0, gs.width as f64, gs.height as f64);

                    // Grid lines
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

                    // Draw tiles and special markers
                    let margin = 0.1f64;
                    for y in 0..gs.height {
                        for x in 0..gs.width {
                            let idx = (y * gs.width + x) as usize;
                            match rs.tiles[idx].kind {
                                model::TileKind::Rock { has_gold, boost } => {
                                    let rx = x as f64 + margin;
                                    let ry = y as f64 + margin;
                                    let rw = 1.0 - 2.0 * margin;
                                    let rh = 1.0 - 2.0 * margin;
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
                                model::TileKind::Start => {
                                    // Fill as Path to keep uniform look
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#121721");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                    // Draw a blue circle at center
                                    let cx = x as f64 + 0.5;
                                    let cy = y as f64 + 0.5;
                                    let r = 0.35;
                                    ctx.begin_path();
                                    ctx.set_fill_style_str("#58a6ff");
                                    ctx.arc(cx, cy, r, 0.0, std::f64::consts::PI * 2.0).ok();
                                    ctx.fill();
                                    ctx.set_stroke_style_str("#1f6feb");
                                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                                    ctx.stroke();
                                }
                                model::TileKind::Direction { dir, role } => {
                                    // Fill as Path to keep uniform look
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    ctx.set_fill_style_str("#121721");
                                    ctx.fill_rect(rx, ry, 1.0, 1.0);
                                    // Draw an oriented arrow triangle on top
                                    let color = match role {
                                        model::DirRole::Entrance => "#2ea043", // green
                                        model::DirRole::Exit => "#f0883e",     // orange
                                    };
                                    ctx.set_fill_style_str(color);
                                    ctx.begin_path();
                                    match dir {
                                        model::ArrowDir::Right => {
                                            ctx.move_to(x as f64 + 0.2, y as f64 + 0.2);
                                            ctx.line_to(x as f64 + 0.2, y as f64 + 0.8);
                                            ctx.line_to(x as f64 + 0.8, y as f64 + 0.5);
                                        }
                                        model::ArrowDir::Left => {
                                            ctx.move_to(x as f64 + 0.8, y as f64 + 0.2);
                                            ctx.line_to(x as f64 + 0.8, y as f64 + 0.8);
                                            ctx.line_to(x as f64 + 0.2, y as f64 + 0.5);
                                        }
                                        model::ArrowDir::Up => {
                                            ctx.move_to(x as f64 + 0.2, y as f64 + 0.8);
                                            ctx.line_to(x as f64 + 0.8, y as f64 + 0.8);
                                            ctx.line_to(x as f64 + 0.5, y as f64 + 0.2);
                                        }
                                        model::ArrowDir::Down => {
                                            ctx.move_to(x as f64 + 0.2, y as f64 + 0.2);
                                            ctx.line_to(x as f64 + 0.8, y as f64 + 0.2);
                                            ctx.line_to(x as f64 + 0.5, y as f64 + 0.8);
                                        }
                                    }
                                    ctx.close_path();
                                    ctx.fill();
                                }
                                model::TileKind::Indestructible => {
                                    let rx = x as f64 + margin;
                                    let ry = y as f64 + margin;
                                    let rw = 1.0 - 2.0 * margin;
                                    let rh = 1.0 - 2.0 * margin;
                                    ctx.set_fill_style_str("#3c4454");
                                    ctx.fill_rect(rx, ry, rw, rh);
                                    ctx.set_stroke_style_str("#596273");
                                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                                    ctx.stroke_rect(rx, ry, rw, rh);
                                }
                                model::TileKind::Empty => {
                                    let rx = x as f64;
                                    let ry = y as f64;
                                    let rw = 1.0;
                                    let rh = 1.0;
                                    ctx.set_fill_style_str("#121721");
                                    ctx.fill_rect(rx, ry, rw, rh);
                                }
                                _ => {}
                            }
                        }
                    }

                    // Draw enemies
                    ctx.set_line_width((1.0 / scale_px).max(0.001));
                    for e in &rs.enemies {
                        ctx.begin_path();
                        ctx.set_fill_style_str("#d73a49");
                        ctx.arc(e.x, e.y, 0.22, 0.0, std::f64::consts::PI * 2.0).ok();
                        ctx.fill();
                        ctx.set_stroke_style_str("#b62324");
                        ctx.stroke();
                    }

                    let m = mining.borrow();
                    if m.active && m.mouse_down {
                        if m.tile_x >= 0 && m.tile_y >= 0 && (m.tile_x as u32) < gs.width && (m.tile_y as u32) < gs.height {
                            let rx = m.tile_x as f64 + margin;
                            let ry = m.tile_y as f64 + margin + (1.0 - 2.0*margin) * (1.0 - m.progress.clamp(0.0, 1.0));
                            let rw = 1.0 - 2.0 * margin;
                            let rh = (1.0 - 2.0 * margin) * m.progress.clamp(0.0, 1.0);
                            ctx.set_fill_style_str("rgba(46,160,67,0.7)");
                            ctx.fill_rect(rx, ry, rw, rh);
                        }
                    }
                }
            };

            // Seed initial path before first draw
            {
                let mut s = rs_ref.borrow_mut();
                if s.path.is_empty() {
                    let path = compute_path(&s);
                    s.path = path;
                    run_state.set((*s).clone());
                }
                drop(s);
            }

            draw();

            // Mining tick (~60 FPS)
            let mining_tick = {
                let run_state = run_state.clone();
                let mining = mining.clone();
                let rs_ref = rs_ref.clone();
                let draw = draw.clone();
                Closure::wrap(Box::new(move || {
                    let mut m = mining.borrow_mut();
                    if !m.active || !m.mouse_down { return; }
                    // Bounds check and tile kind check
                    let rs = (*run_state).clone();
                    if rs.is_paused { return; }
                    let gs = rs.grid_size;
                    if m.tile_x < 0 || m.tile_y < 0 || (m.tile_x as u32) >= gs.width || (m.tile_y as u32) >= gs.height {
                        m.active = false;
                        return;
                    }
                    let idx = (m.tile_y as u32 * gs.width + m.tile_x as u32) as usize;
                    let tile = &rs.tiles[idx];
                    if let model::TileKind::Rock { .. } = tile.kind {
                        m.elapsed_secs += 0.016;
                        m.progress = (m.elapsed_secs / m.required_secs).min(1.0);
                        if m.progress >= 1.0 {
                            drop(m);
                            // complete mining using rs_ref to avoid lost updates
                            {
                                let mut s = rs_ref.borrow_mut();
                                let mut grant_gold = false;
                                if let model::TileKind::Rock { has_gold, .. } = s.tiles[idx].kind.clone() {
                                    s.tiles[idx].kind = model::TileKind::Empty;
                                    s.stats.blocks_mined = s.stats.blocks_mined.saturating_add(1);
                                    s.currencies.tile_credits = s.currencies.tile_credits.saturating_add(1);
                                    grant_gold = has_gold;
                                }
                                if grant_gold {
                                    s.currencies.gold = s.currencies.gold.saturating_add(1);
                                    let gs2 = s.grid_size;
                                    let tx = (idx as u32) % gs2.width;
                                    let ty = (idx as u32) / gs2.width;
                                    clog(&format!("gold +1 -> {} (mined @{}, {})", s.currencies.gold, tx, ty));
                                }
                                // Recompute path after terrain change
                                s.path = compute_path(&s);
                                run_state.set((*s).clone());
                            }
                            let mut m2 = mining.borrow_mut();
                            m2.active = false;
                            m2.mouse_down = false;
                            m2.progress = 0.0;
                            m2.elapsed_secs = 0.0;
                            drop(m2);
                            draw();
                            return;
                        } else {
                            // ensure timer starts on first mining progress using rs_ref
                            {
                                let mut s = rs_ref.borrow_mut();
                                if !s.started {
                                    s.started = true;
                                    run_state.set((*s).clone());
                                }
                            }
                        }
                        drop(m);
                        draw();
                    } else {
                        m.active = false;
                    }
                }) as Box<dyn FnMut()> )
            };
            let mining_tick_id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(mining_tick.as_ref().unchecked_ref(), 16)
                .unwrap();

            // Simulation tick (~60 FPS) for enemies
            let sim_tick = {
                let run_state = run_state.clone();
                let rs_ref = rs_ref.clone();
                let draw = draw.clone();
                Closure::wrap(Box::new(move || {
                    let rs_snapshot = (*run_state).clone();
                    if rs_snapshot.started && !rs_snapshot.is_paused {
                        {
                            let mut s = rs_ref.borrow_mut();
                            if s.path.is_empty() {
                                s.path = compute_path(&s);
                            }
                            // Spawn cadence: every 2 seconds if path is ready
                            let t = s.stats.time_survived_secs;
                            let path = s.path.clone();
                            if path.len() >= 2 && t.saturating_sub(s.last_enemy_spawn_time_secs) >= 2 {
                                let first = path[0];
                                let speed = 1.0 + 0.002 * (t as f64);
                                let hp = 5 + (t / 10) as u32;
                                let e = Enemy {
                                    x: first.x as f64 + 0.5,
                                    y: first.y as f64 + 0.5,
                                    speed_tps: speed,
                                    hp,
                                    spawned_at: t,
                                    path_index: 1,
                                };
                                s.enemies.push(e);
                                clog(&format!("enemy spawned at t={} speed={:.3} hp={} start=({}, {})", t, speed, hp, first.x, first.y));
                                s.last_enemy_spawn_time_secs = t;
                            }
                            // Advance enemies along path
                            if !path.is_empty() {
                                let enemies_vec = std::mem::take(&mut s.enemies);
                                let mut survivors: Vec<Enemy> = Vec::with_capacity(enemies_vec.len());
                                for mut e in enemies_vec.into_iter() {
                                    let mut remaining = 0.016 * e.speed_tps; // tiles per frame
                                    while remaining > 0.0 && e.path_index < path.len() {
                                        let wp = path[e.path_index];
                                        let tx = wp.x as f64 + 0.5;
                                        let ty = wp.y as f64 + 0.5;
                                        let dx = tx - e.x;
                                        let dy = ty - e.y;
                                        let dist = (dx*dx + dy*dy).sqrt();
                                        if dist < 1e-6 {
                                            e.path_index += 1;
                                            continue;
                                        }
                                        if remaining >= dist {
                                            e.x = tx; e.y = ty;
                                            remaining -= dist;
                                            e.path_index += 1;
                                        } else {
                                            let ratio = remaining / dist;
                                            e.x += dx * ratio;
                                            e.y += dy * ratio;
                                            remaining = 0.0;
                                        }
                                    }
                                    if e.path_index >= path.len() {
                                        // Reached Exit
                                        let old_life = s.life;
                                        if s.life > 0 { s.life -= 1; }
                                        clog(&format!("life: {} -> {} (enemy exited)", old_life, s.life));
                                        s.stats.loops_completed = s.stats.loops_completed.saturating_add(1);
                                    } else {
                                        survivors.push(e);
                                    }
                                }
                                s.enemies = survivors;
                            }
                            run_state.set((*s).clone());
                        }
                        draw();
                    }
                }) as Box<dyn FnMut()>)
            };
            let sim_tick_id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(sim_tick.as_ref().unchecked_ref(), 16)
                .unwrap();

            // Wheel (zoom)
            let wheel_cb = {
                let camera = camera.clone();
                let draw = draw.clone();
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
                    draw();
                }) as Box<dyn FnMut(_)> )
            };
            canvas
                .add_event_listener_with_callback("wheel", wheel_cb.as_ref().unchecked_ref())
                .unwrap();

            // Mouse down: left = mine; middle/right = pan
            let mousedown_cb = {
                let camera = camera.clone();
                let mining = mining.clone();
                let run_state = run_state.clone();
                let rs_ref = rs_ref.clone();
                let draw = draw.clone();
                Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    let button = e.button();
                    if button == 0 {
                        // Left click: start mining on tile under cursor
                        let cam = camera.borrow_mut();
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                        let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                        drop(cam);
                        let rs = (*run_state).clone();
                        if rs.is_paused { return; }
                        let gs = rs.grid_size;
                        let tx = world_x.floor() as i32;
                        let ty = world_y.floor() as i32;
                        if tx >= 0 && ty >= 0 && (tx as u32) < gs.width && (ty as u32) < gs.height {
                            let idx = (ty as u32 * gs.width + tx as u32) as usize;
                            if let model::TileKind::Rock { .. } = rs.tiles[idx].kind {
                                // Start the run timer immediately on initiating mining (if not already started)
                                {
                                    let mut s = rs_ref.borrow_mut();
                                    if !s.started {
                                        s.started = true;
                                        run_state.set((*s).clone());
                                    }
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
                                drop(m);
                                draw();
                            }
                        }
                    } else {
                        // Middle/right: pan
                        let mut cam = camera.borrow_mut();
                        cam.panning = true;
                        cam.last_x = e.client_x() as f64;
                        cam.last_y = e.client_y() as f64;
                    }
                }) as Box<dyn FnMut(_)> )
            };
            canvas
                .add_event_listener_with_callback("mousedown", mousedown_cb.as_ref().unchecked_ref())
                .unwrap();

            // Mouse move: pan if panning; else handle mining hover/reset
            let mousemove_cb = {
                let camera = camera.clone();
                let mining = mining.clone();
                let run_state = run_state.clone();
                let draw = draw.clone();
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
                        draw();
                    } else {
                        // mining hover/reset
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                        let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                        drop(cam);
                        let mut m = mining.borrow_mut();
                        if m.mouse_down {
                            let rs = (*run_state).clone();
                            if rs.is_paused {
                                m.active = false;
                                m.progress = 0.0;
                                m.elapsed_secs = 0.0;
                                drop(m);
                                draw();
                                return;
                            }
                            let gs = rs.grid_size;
                            let tx = world_x.floor() as i32;
                            let ty = world_y.floor() as i32;
                            if tx >= 0 && ty >= 0 && (tx as u32) < gs.width && (ty as u32) < gs.height {
                                let idx = (ty as u32 * gs.width + tx as u32) as usize;
                                match rs.tiles[idx].kind {
                                    model::TileKind::Rock { .. } => {
                                        if tx != m.tile_x || ty != m.tile_y {
                                            // reset and start on new tile
                                            m.tile_x = tx;
                                            m.tile_y = ty;
                                            let hardness = rs.tiles[idx].hardness.max(1) as f64;
                                            let spd = rs.mining_speed.max(0.0001);
                                            m.required_secs = hardness / spd;
                                            m.elapsed_secs = 0.0;
                                            m.progress = 0.0;
                                            m.active = true;
                                        }
                                    }
                                    _ => {
                                        // moved off a mineable tile: stop and reset
                                        m.active = false;
                                        m.progress = 0.0;
                                        m.elapsed_secs = 0.0;
                                    }
                                }
                            } else {
                                m.active = false;
                                m.progress = 0.0;
                                m.elapsed_secs = 0.0;
                            }
                            drop(m);
                            draw();
                        }
                    }
                }) as Box<dyn FnMut(_)> )
            };
            canvas
                .add_event_listener_with_callback("mousemove", mousemove_cb.as_ref().unchecked_ref())
                .unwrap();

            // Mouse up (stop panning or mining)
            let mouseup_cb = {
                let camera = camera.clone();
                let mining = mining.clone();
                let draw = draw.clone();
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
                    draw();
                }) as Box<dyn FnMut(_)> )
            };
            window
                .add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref())
                .unwrap();

            // Prevent context menu for right-click panning on canvas
            let contextmenu_cb = {
                Closure::wrap(Box::new(move |e: web_sys::Event| {
                    e.prevent_default();
                }) as Box<dyn FnMut(_)> )
            };
            canvas
                .add_event_listener_with_callback("contextmenu", contextmenu_cb.as_ref().unchecked_ref())
                .unwrap();

            // Resize
            let resize_cb = {
                let draw = draw.clone();
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    compute_and_apply_canvas_size();
                    draw();
                }) as Box<dyn FnMut(_)> )
            };
            window
                .add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref())
                .unwrap();

            // Cleanup
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
                let _ = window.remove_event_listener_with_callback(
                    "mouseup",
                    mouseup_cb.as_ref().unchecked_ref(),
                );
                let _ = window.remove_event_listener_with_callback(
                    "resize",
                    resize_cb.as_ref().unchecked_ref(),
                );
                let _ = window.clear_interval_with_handle(mining_tick_id);
                let _ = window.clear_interval_with_handle(sim_tick_id);
                drop(mining_tick);
                drop(sim_tick);
                drop(wheel_cb);
                drop(mousedown_cb);
                drop(mousemove_cb);
                drop(mouseup_cb);
                drop(contextmenu_cb);
                drop(resize_cb);
            }
        });
    }

    // Overlay controls (camera) and legend panels
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
            let _ = web_sys::window().unwrap().dispatch_event(&web_sys::Event::new("resize").unwrap());
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
            let _ = web_sys::window().unwrap().dispatch_event(&web_sys::Event::new("resize").unwrap());
        })
    };
    let pan_by = |dx: f64, dy: f64| {
        let camera = camera.clone();
        Callback::from(move |_| {
            let mut cam = camera.borrow_mut();
            cam.offset_x += dx;
            cam.offset_y += dy;
            drop(cam);
            let _ = web_sys::window().unwrap().dispatch_event(&web_sys::Event::new("resize").unwrap());
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
                let _ = web_sys::window().unwrap().dispatch_event(&web_sys::Event::new("resize").unwrap());
            }
        })
    };

    // Legend presence computation
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
                if *hg { has_gold = true; } else { has_basic = true; }
            }
            model::TileKind::Empty => has_empty = true,
            model::TileKind::Start => has_start = true,
            model::TileKind::Direction { role, .. } => {
                match role {
                    model::DirRole::Entrance => has_entrance = true,
                    model::DirRole::Exit => has_exit = true,
                }
            }
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
    let pause_label_rv = if paused_ov { "Resume (Space)" } else { "Pause (Space)" };
    let toggle_pause_rv = {
        let run_state = props.run_state.clone();
        let rs_ref = rs_ref.clone();
        Callback::from(move |_: yew::events::MouseEvent| {
            let mut s = rs_ref.borrow_mut();
            s.is_paused = !s.is_paused;
            run_state.set((*s).clone());
        })
    };
    let to_upgrades_click = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_: yew::events::MouseEvent| cb.emit(()))
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
            </div>
            <div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:180px; display:flex; flex-direction:column; gap:6px;">
                <button onclick={toggle_pause_rv.clone()}>{ pause_label_rv }</button>
                <button onclick={to_upgrades_click.clone()}>{"Upgrades"}</button>
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
                { if has_empty { html!{ <LegendRow color="#121721" label="Path" /> } } else { html!{} } }
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct LegendRowProps { pub color: &'static str, pub label: &'static str }

#[function_component(LegendRow)]
fn legend_row(props: &LegendRowProps) -> Html {
    html!{
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
            zoom: 1.0,
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

#[function_component(App)]
fn app() -> Html {
    let view = use_state(|| View::Run);

    // Game/meta state
    let run_state = use_state(|| RunState::new_basic(GridSize { width: 25, height: 25 }));
    let _upgrade_state = use_state(|| UpgradeState { tower_refund_rate_percent: 100, ..Default::default() });
        let last_resources = use_mut_ref(|| (0u64, 0u64, 0u32)); // (gold, research, life)

    // Ticker for run time
    {
        let run_state = run_state.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let run_state2 = run_state.clone();
            let tick = Closure::wrap(Box::new(move || {
                let mut latest = (*run_state2).clone();
                if latest.started && !latest.is_paused {
                    latest.stats.time_survived_secs = latest.stats.time_survived_secs.saturating_add(1);
                    clog(&format!("time tick: {}", latest.stats.time_survived_secs));
                    run_state2.set(latest);
                } else if latest.started && latest.is_paused {
                    clog(&format!("time paused at {}", latest.stats.time_survived_secs));
                } else {
                    clog("time idle (not started)");
                }
            }) as Box<dyn FnMut()>);
            let id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    tick.as_ref().unchecked_ref(),
                    1000,
                )
                .unwrap();
            // Spacebar pause/resume hotkey
            let key_cb = {
                let run_state = run_state.clone();
                Closure::wrap(Box::new(move |e: KeyboardEvent| {
                    if e.code() == "Space" {
                        e.prevent_default();
                        let mut rs = (*run_state).clone();
                        rs.is_paused = !rs.is_paused;
                        run_state.set(rs);
                    }
                }) as Box<dyn FnMut(_)> )
            };
            window
                .add_event_listener_with_callback("keydown", key_cb.as_ref().unchecked_ref())
                .unwrap();
            move || {
                let _ = window.clear_interval_with_handle(id);
                let _ = window.remove_event_listener_with_callback("keydown", key_cb.as_ref().unchecked_ref());
                drop(key_cb);
                drop(tick);
            }
        });
    }

    // Log resource changes: gold, research, life
    {
        let run_state = run_state.clone();
        let last_resources = last_resources.clone();
        use_effect_with(((*run_state).currencies.gold, (*run_state).currencies.research, (*run_state).life), move |deps| {
            let (g, r, l) = *deps;
            let mut prev = last_resources.borrow_mut();
            if prev.0 != g { clog(&format!("gold: {} -> {}", prev.0, g)); }
            if prev.1 != r { clog(&format!("research: {} -> {}", prev.1, r)); }
            if prev.2 != l { clog(&format!("life: {} -> {}", prev.2, l)); }
            *prev = (g, r, l);
            || ()
        });
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
