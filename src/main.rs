use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};
use yew::prelude::*;

mod model;
use model::{GridSize, RunState, UpgradeState};

#[derive(PartialEq, Clone)]
enum View {
    Run,
    Upgrades,
}

#[derive(Properties, PartialEq, Clone)]
struct RunViewProps {
    pub run_state: UseStateHandle<RunState>,
}

#[function_component(RunView)]
fn run_view(props: &RunViewProps) -> Html {
    let canvas_ref = use_node_ref();
    let camera = use_mut_ref(|| Camera::default());
    let mining = use_mut_ref(|| Mining::default());

    {
        let canvas_ref = canvas_ref.clone();
        let camera = camera.clone();
        let run_state = props.run_state.clone();

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
                    ctx.set_fill_style(&JsValue::from_str("#0e1116"));
                    ctx.fill_rect(0.0, 0.0, w, h);

                    let cam = camera.borrow();
                    let tile_px = 32.0;
                    let scale_px = cam.zoom * tile_px;
                    ctx.set_transform(scale_px, 0.0, 0.0, scale_px, cam.offset_x, cam.offset_y)
                        .ok();

                    let rs = (*run_state).clone();
                    let gs = rs.grid_size;

                    // Grid background
                    ctx.set_fill_style(&JsValue::from_str("#161b22"));
                    ctx.fill_rect(0.0, 0.0, gs.width as f64, gs.height as f64);

                    // Draw tiles
                    let rock_fill = JsValue::from_str("#1d2430");
                    let rock_border = JsValue::from_str("#3a4455");
                    let margin = 0.1f64;
                    for y in 0..gs.height {
                        for x in 0..gs.width {
                            let idx = (y * gs.width + x) as usize;
                            if let model::TileKind::Rock { .. } = rs.tiles[idx].kind {
                                let rx = x as f64 + margin;
                                let ry = y as f64 + margin;
                                let rw = 1.0 - 2.0 * margin;
                                let rh = 1.0 - 2.0 * margin;
                                ctx.set_fill_style(&rock_fill);
                                ctx.fill_rect(rx, ry, rw, rh);
                                ctx.set_stroke_style(&rock_border);
                                ctx.set_line_width((1.0 / scale_px).max(0.001));
                                ctx.stroke_rect(rx, ry, rw, rh);
                            }
                        }
                    }

                    // Grid lines
                    ctx.set_stroke_style(&JsValue::from_str("#2f3641"));
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

                    // Mining progress overlay
                    let m = mining.borrow();
                    if m.active && m.mouse_down {
                        if m.tile_x >= 0 && m.tile_y >= 0 && (m.tile_x as u32) < gs.width && (m.tile_y as u32) < gs.height {
                            let rx = m.tile_x as f64 + margin;
                            let ry = m.tile_y as f64 + margin + (1.0 - 2.0*margin) * (1.0 - m.progress.clamp(0.0, 1.0));
                            let rw = 1.0 - 2.0 * margin;
                            let rh = (1.0 - 2.0 * margin) * m.progress.clamp(0.0, 1.0);
                            ctx.set_fill_style(&JsValue::from_str("rgba(46,160,67,0.7)"));
                            ctx.fill_rect(rx, ry, rw, rh);
                        }
                    }
                }
            };

            draw();

            // Mining tick (~60 FPS)
            let mining_tick = {
                let run_state = run_state.clone();
                let mining = mining.clone();
                let draw = draw.clone();
                Closure::wrap(Box::new(move || {
                    let mut m = mining.borrow_mut();
                    if !m.active || !m.mouse_down { return; }
                    // Bounds check and tile kind check
                    let rs = (*run_state).clone();
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
                            // complete mining
                            let mut rs2 = rs.clone();
                            if let model::TileKind::Rock { has_gold, .. } = rs2.tiles[idx].kind.clone() {
                                rs2.tiles[idx].kind = model::TileKind::Empty;
                                rs2.stats.blocks_mined = rs2.stats.blocks_mined.saturating_add(1);
                                rs2.currencies.tile_credits = rs2.currencies.tile_credits.saturating_add(1);
                                if has_gold { rs2.currencies.gold = rs2.currencies.gold.saturating_add(1); }
                            }
                            run_state.set(rs2);
                            let mut m2 = mining.borrow_mut();
                            m2.active = false;
                            m2.mouse_down = false;
                            m2.progress = 0.0;
                            m2.elapsed_secs = 0.0;
                            drop(m2);
                            draw();
                            return;
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

            // Wheel (zoom)
            let wheel_cb = {
                let camera = camera.clone();
                let draw = draw.clone();
                Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
                    e.prevent_default();
                    let mut cam = camera.borrow_mut();
                    let delta = e.delta_y();
                    let zoom_change = (-delta * 0.001).exp();
                    cam.zoom = (cam.zoom * zoom_change).clamp(0.2, 5.0);
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
                let draw = draw.clone();
                Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    let button = e.button();
                    if button == 0 {
                        // Left click: start mining on tile under cursor
                        let mut cam = camera.borrow_mut();
                        let tile_px = 32.0;
                        let scale_px = cam.zoom * tile_px;
                        let world_x = ((e.offset_x() as f64) - cam.offset_x) / scale_px;
                        let world_y = ((e.offset_y() as f64) - cam.offset_y) / scale_px;
                        drop(cam);
                        let rs = (*run_state).clone();
                        let gs = rs.grid_size;
                        let tx = world_x.floor() as i32;
                        let ty = world_y.floor() as i32;
                        if tx >= 0 && ty >= 0 && (tx as u32) < gs.width && (ty as u32) < gs.height {
                            let idx = (ty as u32 * gs.width + tx as u32) as usize;
                            if let model::TileKind::Rock { .. } = rs.tiles[idx].kind {
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
                drop(mining_tick);
                drop(wheel_cb);
                drop(mousedown_cb);
                drop(mousemove_cb);
                drop(mouseup_cb);
                drop(contextmenu_cb);
                drop(resize_cb);
            }
        });
    }

    html! { <canvas ref={canvas_ref} id="game-canvas" style="display:block; width:100vw; height:calc(100vh - 48px);"></canvas> }
}

struct Camera {
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    panning: bool,
    last_x: f64,
    last_y: f64,
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

    // Ticker for run time
    {
        let run_state = run_state.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let tick = Closure::wrap(Box::new(move || {
                let mut rs = (*run_state).clone();
                rs.stats.time_survived_secs = rs.stats.time_survived_secs.saturating_add(1);
                run_state.set(rs);
            }) as Box<dyn FnMut()>);
            let id = window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    tick.as_ref().unchecked_ref(),
                    1000,
                )
                .unwrap();
            move || {
                let _ = window.clear_interval_with_handle(id);
                drop(tick);
            }
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

    let gold = (*run_state).currencies.gold;
    let research = (*run_state).currencies.research;
    let life = (*run_state).life;
    let time = (*run_state).stats.time_survived_secs;

    html! {
        <div id="root">
            <div id="top-bar" class="nav" style="padding: 8px 12px; display: flex; align-items: center; gap: 8px;">
                <button onclick={to_run.clone()}>{"Run"}</button>
                <button onclick={to_upgrades.clone()}>{"Upgrades"}</button>
                <span style="margin-left: 16px;">{format!("Gold: {}", gold)}</span>
                <span style="margin-left: 12px;">{format!("Life: {}", life)}</span>
                <span style="margin-left: 12px;">{format!("Research: {}", research)}</span>
                <span style="margin-left: 12px;">{format!("Time: {}s", time)}</span>
            </div>
            {
                match (*view).clone() {
                    View::Run => html! { <RunView run_state={run_state.clone()} /> },
                    View::Upgrades => html! {
                        <div id="upgrades-view" style="padding: 12px;">
                            <h2>{"Upgrades"}</h2>
                            <p>{"Spend research to improve mining speed, starting gold, tower stats, etc. (coming soon)"}</p>
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
