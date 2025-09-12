use crate::model::{RunAction, RunState, UpgradeId, UpgradeState, UPGRADE_DEFS};
use std::collections::{HashMap, HashSet, VecDeque};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct UpgradesViewProps {
    pub run_state: UseReducerHandle<RunState>,
    pub upgrade_state: UseStateHandle<UpgradeState>,
    pub to_run: Callback<()>,
    pub purchase: Callback<UpgradeId>,
}

#[function_component(UpgradesView)]
pub fn upgrades_view(props: &UpgradesViewProps) -> Html {
    // Pan / zoom state
    let tree_zoom = use_state(|| 1.0_f64);
    let tree_offset = use_state(|| (0.0_f64, 0.0_f64));
    let dragging = use_state(|| false);
    let drag_last = use_state(|| (0.0_f64, 0.0_f64));
    let container_ref = use_node_ref();
    // New: reset confirmation state
    let show_reset_confirm = use_state(|| false);

    // Handlers
    let wheel_tree = {
        let tree_zoom = tree_zoom.clone();
        let tree_offset = tree_offset.clone();
        let container_ref = container_ref.clone();
        Callback::from(move |e: yew::events::WheelEvent| {
            e.prevent_default();
            let delta = e.delta_y();
            let factor = (-delta * 0.001).exp();
            let old = *tree_zoom;
            let new = (old * factor).clamp(0.25, 3.0);
            if let Some(el) = container_ref.cast::<web_sys::HtmlElement>() {
                let r = el.get_bounding_client_rect();
                let cx = e.client_x() as f64 - r.left();
                let cy = e.client_y() as f64 - r.top();
                let (ox, oy) = *tree_offset;
                let wx = (cx - ox) / old;
                let wy = (cy - oy) / old;
                tree_offset.set((cx - wx * new, cy - wy * new));
            }
            tree_zoom.set(new);
        })
    };
    let mousedown_tree = {
        let dragging = dragging.clone();
        let drag_last = drag_last.clone();
        Callback::from(move |e: yew::events::MouseEvent| {
            if e.button() == 0 {
                dragging.set(true);
                drag_last.set((e.client_x() as f64, e.client_y() as f64));
            }
        })
    };
    let mousemove_tree = {
        let dragging = dragging.clone();
        let drag_last = drag_last.clone();
        let tree_offset = tree_offset.clone();
        Callback::from(move |e: yew::events::MouseEvent| {
            if *dragging {
                let (lx, ly) = *drag_last;
                let dx = e.client_x() as f64 - lx;
                let dy = e.client_y() as f64 - ly;
                let (ox, oy) = *tree_offset;
                tree_offset.set((ox + dx, oy + dy));
                drag_last.set((e.client_x() as f64, e.client_y() as f64));
            }
        })
    };
    let mouseup_tree = {
        let dragging = dragging.clone();
        Callback::from(move |_| dragging.set(false))
    };

    // Snapshot
    let research = props.run_state.currencies.research;
    let upgrade_state_snapshot = (*props.upgrade_state).clone();

    // Build dependency edges strictly from prerequisites
    let mut raw_edges: Vec<(UpgradeId, UpgradeId)> = Vec::new();
    for def in UPGRADE_DEFS.iter() {
        for prereq in def.prerequisites {
            raw_edges.push((prereq.id, def.id));
        }
    }
    // Choose a root: first upgrade with no prerequisites (TowerDamage1 exists)
    let root = UpgradeId::TowerDamage1;

    // Remove duplicate edges
    let mut seen = HashSet::new();
    let mut edges: Vec<(UpgradeId, UpgradeId)> = Vec::new();
    for (a, b) in raw_edges.into_iter() {
        if seen.insert((a as usize, b as usize)) {
            edges.push((a, b));
        }
    }
    // Adjacency + depth BFS
    let mut adj: HashMap<UpgradeId, Vec<UpgradeId>> = HashMap::new();
    for (a, b) in &edges {
        adj.entry(*a).or_default().push(*b);
    }
    let mut depth: HashMap<UpgradeId, usize> = HashMap::new();
    depth.insert(root, 0);
    let mut q = VecDeque::new();
    q.push_back(root);
    while let Some(u) = q.pop_front() {
        let d = depth[&u];
        if let Some(list) = adj.get(&u) {
            for v in list {
                if !depth.contains_key(v) {
                    depth.insert(*v, d + 1);
                    q.push_back(*v);
                }
            }
        }
    }
    // Any node not reached (should not happen unless disconnected) -> place after max depth
    let maxd = depth.values().copied().max().unwrap_or(0);
    for def in UPGRADE_DEFS.iter() {
        depth.entry(def.id).or_insert(maxd + 1);
    }
    let mut by_depth: HashMap<usize, Vec<UpgradeId>> = HashMap::new();
    for def in UPGRADE_DEFS.iter() {
        by_depth.entry(depth[&def.id]).or_default().push(def.id);
    }
    let mut depths: Vec<usize> = by_depth.keys().copied().collect();
    depths.sort();

    // Layout parameters
    let node_w = 190.0;
    let node_h = 140.0;
    let h_gap = 260.0;
    let v_gap = 220.0;
    #[derive(Clone)]
    struct Layout {
        id: UpgradeId,
        x: f64,
        y: f64,
    }
    let mut layouts: Vec<Layout> = Vec::new();
    for d in &depths {
        let list = &by_depth[d];
        if list.is_empty() {
            continue;
        }
        let total_w = (list.len().saturating_sub(1)) as f64 * h_gap;
        let start_x = -total_w / 2.0;
        for (i, id) in list.iter().enumerate() {
            let x = start_x + i as f64 * h_gap;
            let y = *d as f64 * v_gap;
            layouts.push(Layout { id: *id, x, y });
        }
    }
    let layout_of = |id: UpgradeId| layouts.iter().find(|l| l.id == id).cloned();

    // Auto-center on mount
    {
        let tree_offset = tree_offset.clone();
        let tree_zoom = tree_zoom.clone();
        let container_ref = container_ref.clone();
        let layouts_clone = layouts.clone();
        use_effect_with((), move |_| {
            if let Some(el) = container_ref.cast::<web_sys::HtmlElement>() {
                if let Some(root_layout) = layouts_clone.iter().find(|l| l.id == root) {
                    let rect = el.get_bounding_client_rect();
                    let zoom = *tree_zoom;
                    let root_cx = root_layout.x + node_w * 0.5;
                    let root_cy = root_layout.y + node_h * 0.5;
                    let new_ox = rect.width() / 2.0 - root_cx * zoom;
                    let new_oy = rect.height() / 2.0 - root_cy * zoom;
                    tree_offset.set((new_ox, new_oy));
                }
            }
            || ()
        });
    }

    // Edge SVG lines
    let edge_paths: Vec<Html> = edges
        .iter()
        .filter_map(|(p, c)| {
            let pl = layout_of(*p)?;
            let cl = layout_of(*c)?;
            let x1 = pl.x + node_w * 0.5;
            let y1 = pl.y + node_h;
            let x2 = cl.x + node_w * 0.5;
            let y2 = cl.y;
            Some(html! {
                <line
                    x1={format!("{:.1}", x1)}
                    y1={format!("{:.1}", y1 + 4.0)}
                    x2={format!("{:.1}", x2)}
                    y2={format!("{:.1}", y2 - 4.0)}
                    stroke="#374151"
                    stroke-width="3"
                    marker-end="url(#arrowhead)"
                />
            })
        })
        .collect();

    let zoom = *tree_zoom;
    let (off_x, off_y) = *tree_offset;
    let transform = format!(
        "transform:translate({}px, {}px) scale({}); transform-origin:0 0;",
        off_x, off_y, zoom
    );

    // Node cards
    let nodes_html: Vec<Html> = layouts
        .iter()
        .map(|lay| {
            let def = &UPGRADE_DEFS[lay.id as usize];
            let ups = &upgrade_state_snapshot;
            let lvl = ups.level(lay.id);
            let max = def.max_level;
            let unlocked = ups.is_unlocked(lay.id);
            let at_max = lvl >= max;
            let cost = ups.next_cost(lay.id);
            let affordable = cost.map(|c| c <= research).unwrap_or(false);
            let name = def.id.key();
            let desc = def.effect_per_level;
            // Build tooltip
            let mut tip = format!("{}\n{}\nLevel: {}/{}", name, desc, lvl, max);
            if let Some(c) = cost {
                tip.push_str(&format!("\nNext: {} RP", c));
            } else {
                tip.push_str("\nMaxed");
            }
            if !unlocked {
                if !def.prerequisites.is_empty() {
                    tip.push_str("\nPrerequisites:");
                    for p in def.prerequisites {
                        let cur = ups.level(p.id);
                        tip.push_str(&format!("\n- {} {} (you: {})", p.id.key(), p.level, cur));
                    }
                }
            }
            let bar = if max > 0 {
                (lvl as f64 / max as f64) * 100.0
            } else {
                0.0
            };
            let disabled = !unlocked || at_max || !affordable;
            let btn_label = if at_max {
                "MAX".into()
            } else {
                cost.map(|c| format!("Buy ({})", c)).unwrap_or("MAX".into())
            };
            let idc = lay.id;
            let onclick_cb = {
                let purchase = props.purchase.clone();
                Callback::from(move |_| purchase.emit(idc))
            };
            html! {
                <div style={format!("position:absolute; width:{}px; height:{}px; transform:translate({}px, {}px);", node_w, node_h, lay.x, lay.y)}>
                    <div
                        style="position:absolute; inset:0; border:2px solid #374151; border-radius:14px; padding:8px 10px 42px 10px; background:#111821;"
                        title={tip}
                    >
                        <div style="font-weight:700; font-size:14px; letter-spacing:.5px;">{ name }</div>
                        <div style="font-size:12px; line-height:1.2; opacity:0.85; white-space:pre-line;">{ desc }</div>
                        <div style="font-size:11px; opacity:0.7;">{ format!("{}/{}", lvl, max) }</div>
                        <button
                            disabled={disabled}
                            style="position:absolute; left:10px; right:10px; bottom:10px; height:26px; font-size:12px; border-radius:8px; border:1px solid #30363d; background:#1c2128; color:#fff;"
                            onclick={onclick_cb}
                        >
                            { btn_label }
                        </button>
                        <div style="position:absolute; left:0; bottom:0; height:6px; width:100%; background:#161b22; border-radius:0 0 14px 14px; overflow:hidden;">
                            <div style={format!("height:100%; width:{:.1}%; background:#3fb950;", bar)}></div>
                        </div>
                    </div>
                </div>
            }
        })
        .collect();

    let svg_w = 4000;
    let svg_h = 4000;

    let zoom_in_btn = {
        let tree_zoom = tree_zoom.clone();
        let tree_offset = tree_offset.clone();
        let container_ref = container_ref.clone();
        Callback::from(move |_| {
            if let Some(el) = container_ref.cast::<web_sys::HtmlElement>() {
                let rect = el.get_bounding_client_rect();
                let cx = rect.width() / 2.0;
                let cy = rect.height() / 2.0;
                let old = *tree_zoom;
                let new = (old * 1.25).clamp(0.25, 3.0);
                let (ox, oy) = *tree_offset;
                let wx = (cx - ox) / old;
                let wy = (cy - oy) / old;
                tree_offset.set((cx - wx * new, cy - wy * new));
                tree_zoom.set(new);
            }
        })
    };
    let zoom_out_btn = {
        let tree_zoom = tree_zoom.clone();
        let tree_offset = tree_offset.clone();
        let container_ref = container_ref.clone();
        Callback::from(move |_| {
            if let Some(el) = container_ref.cast::<web_sys::HtmlElement>() {
                let rect = el.get_bounding_client_rect();
                let cx = rect.width() / 2.0;
                let cy = rect.height() / 2.0;
                let old = *tree_zoom;
                let new = (old * 0.8).clamp(0.25, 3.0);
                let (ox, oy) = *tree_offset;
                let wx = (cx - ox) / old;
                let wy = (cy - oy) / old;
                tree_offset.set((cx - wx * new, cy - wy * new));
                tree_zoom.set(new);
            }
        })
    };
    let center_btn = {
        let tree_offset = tree_offset.clone();
        let tree_zoom = tree_zoom.clone();
        let container_ref = container_ref.clone();
        let layouts_clone = layouts.clone();
        Callback::from(move |_| {
            if let Some(el) = container_ref.cast::<web_sys::HtmlElement>() {
                let rect = el.get_bounding_client_rect(); // center on root node
                if let Some(root_layout) = layouts_clone.iter().find(|l| l.id == root) {
                    let zoom = *tree_zoom;
                    let root_cx = root_layout.x + node_w * 0.5;
                    let root_cy = root_layout.y + node_h * 0.5;
                    let new_ox = rect.width() / 2.0 - root_cx * zoom;
                    let new_oy = rect.height() / 2.0 - root_cy * zoom;
                    tree_offset.set((new_ox, new_oy));
                }
            }
        })
    };

    // Prevent dragging when pressing inside control panels
    let stop_mouse_down = Callback::from(|e: yew::events::MouseEvent| {
        e.stop_propagation();
    });

    // Reset button callback (open modal)
    let open_reset = {
        let show_reset_confirm = show_reset_confirm.clone();
        Callback::from(move |_| show_reset_confirm.set(true))
    };
    // Cancel reset
    let cancel_reset = {
        let show_reset_confirm = show_reset_confirm.clone();
        Callback::from(move |_| show_reset_confirm.set(false))
    };
    // Confirm reset logic
    let confirm_reset = {
        let show_reset_confirm = show_reset_confirm.clone();
        let upgrade_state_handle = props.upgrade_state.clone();
        let run_state_handle = props.run_state.clone();
        Callback::from(move |_| {
            // Clear relevant localStorage keys
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    let _ = store.remove_item("md_upgrade_state");
                    let _ = store.remove_item("md_research");
                    let _ = store.remove_item("md_intro_seen"); // also reset intro/help
                }
            }
            // Reset upgrade state (preserve custom initial values as in App)
            let new_ups = UpgradeState {
                tower_refund_rate_percent: 100,
                ..Default::default()
            };
            // (If future: ensure any always-on baseline adjustments here)
            upgrade_state_handle.set(new_ups.clone());
            // Reset research in run_state
            run_state_handle.dispatch(RunAction::SetResearch { amount: 0 });
            // Apply fresh upgrades to run state so it reflects base modifiers
            run_state_handle.dispatch(RunAction::ApplyUpgrades { ups: new_ups });
            show_reset_confirm.set(false);
        })
    };

    // Confirmation modal HTML
    let reset_modal = if *show_reset_confirm {
        html! {
            <div style="position:absolute; inset:0; background:rgba(0,0,0,0.55); backdrop-filter:blur(2px); display:flex; align-items:center; justify-content:center; z-index:200;">
                <div style="width:360px; max-width:90%; background:#161b22; border:1px solid #30363d; border-radius:12px; padding:18px 20px 16px 20px; display:flex; flex-direction:column; gap:14px;" onmousedown={stop_mouse_down.clone()}>
                    <div style="font-size:16px; font-weight:600;">{"Reset Progress"}</div>
                    <div style="font-size:13px; line-height:1.4; opacity:0.85;">
                        {"This will erase all upgrades, research points, and show the intro again. This cannot be undone. Are you sure you want to reset?"}
                    </div>
                    <div style="display:flex; gap:10px; justify-content:flex-end;">
                        <button onclick={cancel_reset.clone()} style="min-width:90px;">{"Cancel"}</button>
                        <button onclick={confirm_reset} style="min-width:110px; background:#b62324; border:1px solid #da3633;">{"Confirm Reset"}</button>
                    </div>
                </div>
            </div>
        }
    } else {
        html! {}
    };

    html! {
        <div style="position:relative; width:100vw; height:100vh; background:#0d1117; overflow:hidden;" ref={container_ref} onwheel={wheel_tree} onmousedown={mousedown_tree} onmousemove={mousemove_tree} onmouseup={mouseup_tree.clone()} onmouseleave={mouseup_tree}>
            <div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.95); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:180px; display:flex; flex-direction:column; gap:6px; z-index:20;" onmousedown={stop_mouse_down.clone()}>
                <div style="font-weight:600;">{ format!("Research: {}", research) }</div>
                <button onclick={{ let cb=props.to_run.clone(); Callback::from(move |_| cb.emit(())) }}>{"Back"}</button>
                <button onclick={open_reset} style="background:#3b1d1d; border:1px solid #5d2d2d;">{"Reset Progress"}</button>
            </div>
            <div style="position:absolute; left:12px; bottom:12px; background:rgba(22,27,34,0.95); border:1px solid #30363d; border-radius:8px; padding:8px; display:flex; gap:6px; align-items:center; z-index:20;" onmousedown={stop_mouse_down}>
                <button onclick={zoom_out_btn}> {"-"} </button>
                <button onclick={zoom_in_btn}> {"+"} </button>
                <span style="width:8px;"></span>
                <button onclick={center_btn}> {"Center"} </button>
            </div>
            <div style={format!("position:absolute; inset:0; cursor:{}; z-index:0;", if *dragging {"grabbing"} else {"grab"})}>
                <div style={transform}>
                    <svg style="position:absolute; inset:0; overflow:visible; pointer-events:none;" width={svg_w.to_string()} height={svg_h.to_string()}><defs><marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"><polygon points="0 0, 10 3.5, 0 7" fill="#374151" /></marker></defs>{ for edge_paths }</svg>
                    { for nodes_html }
                </div>
            </div>
            { reset_modal }
        </div>
    }
}
