use crate::model::{RunAction, RunState, UpgradeId, UpgradeState, UPGRADE_DEFS, play_area_size_for_level};
use std::collections::{HashMap, HashSet};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct UpgradesViewProps {
    pub run_state: UseReducerHandle<RunState>,
    pub upgrade_state: UseStateHandle<UpgradeState>,
    pub to_run: Callback<()>,
    pub purchase: Callback<UpgradeId>,
}

fn cat_symbol(cat: &str) -> &'static str {
    match cat {
        "Damage" => "⚔",
        "Health" => "❤",
        "Economy" => "💰",
        "Boost" => "✦",
        "PlayArea" => "⛶",
        _ => "●",
    }
}

fn compute_depths() -> HashMap<UpgradeId, usize> {
    let mut depth: HashMap<UpgradeId, usize> = HashMap::new();
    depth.insert(UpgradeId::TowerDamage1, 0);
    let mut changed = true;
    while changed {
        changed = false;
        for def in UPGRADE_DEFS {
            if def.id == UpgradeId::TowerDamage1 {
                continue;
            }
            let d = if def.prerequisites.is_empty() {
                Some(1)
            } else {
                let mut ok = true;
                let mut maxd = 0usize;
                for p in def.prerequisites {
                    if let Some(pd) = depth.get(&p.id) {
                        maxd = maxd.max(*pd);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok { Some(maxd + 1) } else { None }
            };
            if let Some(v) = d {
                if depth.insert(def.id, v) != Some(v) {
                    changed = true;
                }
            }
        }
    }
    for def in UPGRADE_DEFS {
        depth.entry(def.id).or_insert(2);
    }
    depth
}

#[function_component(UpgradesView)]
pub fn upgrades_view(props: &UpgradesViewProps) -> Html {
    // --- State ---
    let zoom = use_state(|| 1.0_f64);
    let offset = use_state(|| (0.0_f64, 0.0_f64));
    let dragging = use_state(|| false);
    let drag_last = use_state(|| (0.0_f64, 0.0_f64));
    let container_ref = use_node_ref();
    let hover_id = use_state(|| Option::<UpgradeId>::None);

    let research = props.run_state.currencies.research;
    let ups = (*props.upgrade_state).clone();

    // Visibility: only show upgrades whose prerequisites are fully met
    let visible_ids: HashSet<UpgradeId> = UPGRADE_DEFS.iter()
        .filter(|d| d.prerequisites.iter().all(|p| ups.level(p.id) >= p.level))
        .map(|d| d.id)
        .collect();

    // Auto-center on first mount
    {
        let offset = offset.clone();
        let container_ref = container_ref.clone();
        use_effect_with((), move |_| {
            if let Some(el) = container_ref.cast::<web_sys::Element>() {
                let rect = el.get_bounding_client_rect();
                offset.set((rect.width() / 2.0, rect.height() / 2.0));
            }
            || ()
        });
    }

    // --- Layout prep ---
    let depths = compute_depths();
    let mut rings: HashMap<usize, Vec<UpgradeId>> = HashMap::new();
    let mut max_depth = 0usize;
    for def in UPGRADE_DEFS { // group by depth
        let d = *depths.get(&def.id).unwrap_or(&1);
        if d > 0 {
            rings.entry(d).or_default().push(def.id);
            max_depth = max_depth.max(d);
        }
    }

    // Position map (root at origin)
    let mut pos: HashMap<UpgradeId, (f64, f64)> = HashMap::new();
    pos.insert(UpgradeId::TowerDamage1, (0.0, 0.0));

    // Precompute parent & child lists per node for quick lookup
    let mut parents: HashMap<UpgradeId, Vec<UpgradeId>> = HashMap::new();
    let mut children: HashMap<UpgradeId, Vec<UpgradeId>> = HashMap::new();
    for def in UPGRADE_DEFS {
        for p in def.prerequisites {
            parents.entry(def.id).or_default().push(p.id);
            children.entry(p.id).or_default().push(def.id);
        }
    }

    let base_ring = 150.0_f64; // radius of depth 1 circle
    let ring_gap = 170.0_f64;  // slightly tighter
    let node_diam = 48.0_f64;
    let node_padding = 28.0_f64; // a bit more padding

    // Improved ring placement: distribute entire ring using parent centroid angles
    for depth_idx in 1..=max_depth {
        if let Some(list) = rings.get_mut(&depth_idx) {
            if list.is_empty() { continue; }
            let r = base_ring + (depth_idx as f64 - 1.0) * ring_gap;
            // Compute base angles
            let mut items: Vec<(UpgradeId, f64)> = Vec::with_capacity(list.len());
            for id in list.iter().copied() {
                let ang = if let Some(ps) = parents.get(&id) {
                    let mut sx = 0.0;
                    let mut sy = 0.0;
                    let mut cnt = 0.0;
                    for pid in ps {
                        if let Some(&(px, py)) = pos.get(&pid) {
                            sx += px;
                            sy += py;
                            cnt += 1.0;
                        }
                    }
                    if cnt > 0.0 { sy.atan2(sx) } else { // fallback deterministic
                        let h = id as u32 as f64;
                        (h * 2.399963229728653).rem_euclid(std::f64::consts::TAU)
                    }
                } else {
                    let h = id as u32 as f64;
                    (h * 2.399963229728653).rem_euclid(std::f64::consts::TAU)
                };
                items.push((id, ang));
            }
            // Sort by angle
            items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            // Enforce minimum separation
            let min_sep_angle = (node_diam + node_padding) / r; // approximate
            let n = items.len();
            // Forward pass
            let mut prev = None;
            for (_, a) in items.iter_mut() {
                while *a < 0.0 { *a += std::f64::consts::TAU; }
                while *a >= std::f64::consts::TAU { *a -= std::f64::consts::TAU; }
                if let Some(p) = prev { if *a < p + min_sep_angle { *a = p + min_sep_angle; } }
                prev = Some(*a);
            }
            // Overflow handling
            if let Some(last) = prev {
                if last >= std::f64::consts::TAU { // compress into full circle
                    let span = last - items[0].1;
                    if span > 1e-6 { // scale angles into [first, first+TAU)
                        let first = items[0].1;
                        for (_, a) in items.iter_mut() { *a = first + (*a - first) / span * (std::f64::consts::TAU - min_sep_angle); }
                    } else { // all equal -> equal spacing
                        for (i, (_, a)) in items.iter_mut().enumerate() { *a = i as f64 * (std::f64::consts::TAU / n as f64); }
                    }
                }
            }
            // Second pass ensure separation after compression
            let mut last = items[0].1;
            for i in 1..n {
                if items[i].1 < last + min_sep_angle { items[i].1 = last + min_sep_angle; }
                last = items[i].1;
            }
            // Wrap again if exceeded
            if items[n - 1].1 >= std::f64::consts::TAU {
                let excess = items[n - 1].1 - std::f64::consts::TAU + min_sep_angle;
                for (i, (_, a)) in items.iter_mut().enumerate() {
                    let t = i as f64 / ((n - 1).max(1) as f64);
                    *a -= excess * t;
                }
            }
            // Local swap optimization (reduce total parent edge length)
            let cost = |id: UpgradeId, ang: f64| -> f64 {
                if let Some(ps) = parents.get(&id) {
                    let mut c = 0.0;
                    for pid in ps {
                        if let Some(&(px, py)) = pos.get(pid) {
                            let dx = ang.cos() * r - px;
                            let dy = ang.sin() * r - py;
                            c += dx * dx + dy * dy;
                        }
                    }
                    c
                } else { 0.0 }
            };
            let mut improved = true;
            let mut passes = 0;
            while improved && passes < 4 {
                improved = false;
                passes += 1;
                for i in 0..n.saturating_sub(1) {
                    let (id_a, ang_a) = items[i];
                    let (id_b, ang_b) = items[i + 1];
                    let before = cost(id_a, ang_a) + cost(id_b, ang_b);
                    let after = cost(id_a, ang_b) + cost(id_b, ang_a);
                    if after + 1e-6 < before {
                        items[i].1 = ang_b;
                        items[i + 1].1 = ang_a;
                        improved = true;
                    }
                }
            }
            // Commit positions
            for (id, ang) in items { pos.insert(id, (r * ang.cos(), r * ang.sin())); }
        }
    }

    // --- SVG edges (lines to prerequisites) ---
    let hovered_opt = *hover_id; // capture early
    // Build ancestor set (full chain to root) & descendant set (full subtree) for hovered node
    let mut ancestor_set: HashSet<UpgradeId> = HashSet::new();
    let mut descendant_set: HashSet<UpgradeId> = HashSet::new();
    if let Some(h) = hovered_opt {
        // ancestors (including self)
        ancestor_set.insert(h);
        let mut stack = vec![h];
        while let Some(cur) = stack.pop() {
            if let Some(ps) = parents.get(&cur) {
                for pid in ps { if ancestor_set.insert(*pid) { stack.push(*pid); } }
            }
        }
        // descendants (excluding self at first then add later if needed)
        let mut dstack = vec![h];
        while let Some(cur) = dstack.pop() {
            if let Some(cs) = children.get(&cur) {
                for cid in cs { if descendant_set.insert(*cid) { dstack.push(*cid); } }
            }
        }
        descendant_set.insert(h); // unify logic (hovered part of both for mixed edge coloring simplicity)
    }
    let chain_set: HashSet<UpgradeId> = if hovered_opt.is_some() {
        ancestor_set.union(&descendant_set).copied().collect()
    } else { HashSet::new() };
    let mut edge_svg: Vec<Html> = Vec::new();
    for def in UPGRADE_DEFS {
        if !visible_ids.contains(&def.id) { continue; } // hide edges for hidden future nodes
        if let Some(&(x2, y2)) = pos.get(&def.id) {
            for p in def.prerequisites {
                if let Some(&(x1, y1)) = pos.get(&p.id) {
                    // parent should always be visible if child is visible (prereqs met), but guard anyway
                    if !visible_ids.contains(&p.id) { continue; }
                    let locked = !ups.is_unlocked(def.id);
                    let parent_len = (x1 * x1 + y1 * y1).sqrt();
                    let child_len = (x2 * x2 + y2 * y2).sqrt();
                    let a1 = y1.atan2(x1);
                    let a2 = y2.atan2(x2);
                    let ac = (a1 + a2) * 0.5;
                    let rc = (parent_len + child_len) * 0.5 + 40.0;
                    let cx = rc * ac.cos();
                    let cy = rc * ac.sin();
                    let both_in = chain_set.contains(&def.id) && chain_set.contains(&p.id);
                    let ancestor_edge = both_in && ancestor_set.contains(&def.id) && ancestor_set.contains(&p.id);
                    let descendant_edge = both_in && descendant_set.contains(&def.id) && descendant_set.contains(&p.id) && !ancestor_edge;
                    let stroke = if both_in {
                        if ancestor_edge { if locked { "#555" } else { "#58a6ff" } } else if descendant_edge { if locked { "#3a5c3a" } else { "#2ea043" } } else { if locked { "#555" } else { "#58a6ff" } }
                    } else if locked { "#262b31" } else { "#30363d" };
                    let width = if both_in { if ancestor_edge { 5 } else if descendant_edge { 4 } else { 5 } } else { 3 };
                    let opacity = if hovered_opt.is_some() && !both_in { 0.12 } else { 1.0 };
                    edge_svg.push(html! {
                        <path d={format!("M{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}", x1, y1, cx, cy, x2, y2)}
                              stroke={stroke}
                              stroke-width={width.to_string()}
                              stroke-linecap="round"
                              fill="none"
                              opacity={format!("{:.2}", opacity)} />
                    });
                }
            }
        }
    }

    // --- Node HTML ---
    let mut node_html: Vec<Html> = Vec::new();
    let purchase_cb = props.purchase.clone();
    for def in UPGRADE_DEFS {
        if !visible_ids.contains(&def.id) { continue; }
        if let Some(&(x, y)) = pos.get(&def.id) {
            let lvl = ups.level(def.id);
            let max = def.max_level;
            let cost = ups.next_cost(def.id);
            let unlocked = ups.is_unlocked(def.id);
            let affordable = cost.map(|c| c <= research).unwrap_or(false);
            let can_buy = unlocked && affordable && lvl < max;
            let base_dim = if !unlocked { 0.30 } else if can_buy { 1.0 } else { 0.75 };
            let is_hovered = Some(def.id) == hovered_opt;
            let is_ancestor = ancestor_set.contains(&def.id) && !is_hovered;
            let is_descendant = descendant_set.contains(&def.id) && !is_hovered && !is_ancestor;
            let in_chain = is_hovered || is_ancestor || is_descendant;
            let dim = if hovered_opt.is_some() && !in_chain { base_dim * 0.18 } else if is_hovered { 1.0 } else { base_dim };
            let is_max = lvl >= max;
            let symbol = if def.id == UpgradeId::TowerDamage1 { "★" } else { cat_symbol(def.category) };
            let border = if is_hovered { "#58a6ff" } else if is_ancestor { "#3c6fa3" } else if is_descendant { "#2ea043" } else if is_max { "#d29922" } else if can_buy { "#2ea043" } else { "#30363d" };
            let bg = if is_hovered { "#1b2733" } else if is_ancestor { "#15222e" } else if is_descendant { "#142818" } else if can_buy { "#1d2b1d" } else { "#111821" };
            let glow = if is_hovered { "0 0 14px #58a6ff" } else if is_ancestor { "0 0 9px #244a68" } else if is_descendant { "0 0 9px #245b2e" } else if can_buy { "0 0 10px #2ea043" } else { "none" };
            let size = if is_hovered { 56.0 } else { 48.0 };
            let ring = depths.get(&def.id).copied().unwrap_or(0);
            let mut tip = format!(
                "{}\nCategory: {}\nEffect: {}\nLevel: {}/{}",
                def.id.key(), def.category, def.effect_per_level, lvl, max
            );
            if def.id == UpgradeId::PlayAreaSize {
                let cur_sz = play_area_size_for_level(lvl as u8);
                tip.push_str(&format!("\nCurrent size: {0}x{0}", cur_sz));
                if lvl < max {
                    let next_sz = play_area_size_for_level(lvl as u8 + 1);
                    tip.push_str(&format!("\nNext size: {0}x{0}", next_sz));
                } else {
                    tip.push_str("\nMax size reached");
                }
            }
            if let Some(c) = cost { if lvl < max { tip.push_str(&format!("\nCost: {} RP", c)); } } else { tip.push_str("\nMaxed"); }
            if !def.prerequisites.is_empty() {
                tip.push_str("\nPrerequisites:");
                for p in def.prerequisites { tip.push_str(&format!("\n- {} {} (you:{})", p.id.key(), p.level, ups.level(p.id))); }
            }
            let hid = hover_id.clone();
            let idc = def.id;
            let on_enter = Callback::from(move |_| hid.set(Some(idc)));
            let hid2 = hover_id.clone();
            let on_leave = Callback::from(move |_| hid2.set(None));
            let purchase2 = purchase_cb.clone();
            let onclick = Callback::from(move |_| purchase2.emit(idc));
            node_html.push(html! {
                <div key={def.id.key()}
                     onmouseenter={on_enter}
                     onmouseleave={on_leave}
                     onclick={onclick}
                     aria-label={tip.clone()}
                     style={format!("position:absolute; left:{:.1}px; top:{:.1}px; width:{:.1}px; height:{:.1}px; margin-left:-{:.1}px; margin-top:-{:.1}px; display:flex; align-items:center; justify-content:center; font-size:{:.0}px; cursor:pointer; user-select:none; border:3px solid {}; background:{}; color:#fff; border-radius:50%; opacity:{:.2}; box-shadow:{}; transition:all 120ms ease;",
                                    x, y, size, size, size / 2.0, size / 2.0, if is_hovered { 26.0 } else { 22.0 }, border, bg, dim, glow)}
                >
                    { symbol }
                    <div style="position:absolute; bottom:-4px; right:-4px; font-size:11px; background:#161b22; padding:2px 4px; border-radius:6px; border:1px solid #30363d;">
                        { format!("{}/{}", lvl, max) }
                    </div>
                    { if ring == 0 { html!{
                        <div style="position:absolute; top:-6px; left:-6px; font-size:10px; background:#2e3138; padding:2px 4px; border-radius:6px; border:1px solid #30363d;">{"ROOT"}</div>
                    }} else { html!{} } }
                </div>
            });
        }
    }

    // --- Tooltip overlay (independent to avoid layout shifts) ---
    let tooltip = if let Some(hid) = *hover_id {
        if let Some(def) = UPGRADE_DEFS.iter().find(|d| d.id == hid) {
            if let Some((x, y)) = pos.get(&hid) {
                let lvl = ups.level(hid);
                let max = def.max_level;
                let cost = ups.next_cost(hid);
                let unlocked = ups.is_unlocked(hid);
                let affordable = cost.map(|c| c <= research).unwrap_or(false);
                let mut lines = vec![
                    format!("{}", def.id.key()),
                    format!("Category: {}", def.category),
                    format!("Effect: {}", def.effect_per_level),
                    format!("Level: {}/{}", lvl, max),
                ];
                if let Some(c) = cost { if lvl < max { lines.push(format!("Cost: {} RP", c)); } } else { lines.push("Maxed".into()); }
                if !def.prerequisites.is_empty() {
                    lines.push("Prereqs:".into());
                    for p in def.prerequisites { lines.push(format!("- {} {} (you:{})", p.id.key(), p.level, ups.level(p.id))); }
                }
                if !unlocked { lines.push("LOCKED".into()); } else if lvl < max && !affordable { lines.push("Need more RP".into()); }
                if def.id == UpgradeId::PlayAreaSize {
                    let cur_sz = play_area_size_for_level(lvl as u8);
                    lines.push(format!("Current size: {0}x{0}", cur_sz));
                    if lvl < max { let next_sz = play_area_size_for_level(lvl as u8 + 1); lines.push(format!("Next size: {0}x{0}", next_sz)); } else { lines.push("Max size reached".into()); }
                }
                let content = lines.join("\n");
                html! { <div style={format!("position:absolute; left:{:.1}px; top:{:.1}px; transform:translate(14px,-14px); background:#161b22; border:1px solid #30363d; padding:8px 10px; white-space:pre; font-size:12px; line-height:1.2; border-radius:8px; max-width:240px; pointer-events:none; z-index:50;", x, y)}>{ content }</div> }
            } else { html! {} }
        } else { html! {} }
    } else { html! {} };

    // --- Respec (refund all invested research points back to pool) ---
    let respec_cb = {
        let run_state = props.run_state.clone();
        let upgrade_state = props.upgrade_state.clone();
        Callback::from(move |_| {
            let current = (*upgrade_state).clone();
            let refund = current.total_spent();
            let mut new_ups = UpgradeState { tower_refund_rate_percent: current.tower_refund_rate_percent, ..Default::default() };
            // preserve any future meta fields if added (only tower_refund_rate_percent now)
            upgrade_state.set(new_ups.clone());
            let new_amount = run_state.currencies.research.saturating_add(refund);
            run_state.dispatch(RunAction::SetResearch { amount: new_amount });
            run_state.dispatch(RunAction::ApplyUpgrades { ups: new_ups });
        })
    };

    // --- Interaction handlers ---
    let mousedown = {
        let dragging = dragging.clone();
        let drag_last = drag_last.clone();
        Callback::from(move |e: yew::events::MouseEvent| {
            dragging.set(true);
            drag_last.set((e.client_x() as f64, e.client_y() as f64));
        })
    };
    let mouseup = { let dragging = dragging.clone(); Callback::from(move |_| dragging.set(false)) };
    let mousemove = {
        let dragging = dragging.clone();
        let drag_last = drag_last.clone();
        let offset = offset.clone();
        Callback::from(move |e: yew::events::MouseEvent| {
            if *dragging {
                let (lx, ly) = *drag_last;
                let nx = e.client_x() as f64;
                let ny = e.client_y() as f64;
                let dx = nx - lx;
                let dy = ny - ly;
                drag_last.set((nx, ny));
                let (ox, oy) = *offset;
                // With transform scale(s) translate(ox,oy), translation is post-scale (screen pixels)
                offset.set((ox + dx, oy + dy));
            }
        })
    };
    let wheel_cb = {
        let zoom = zoom.clone();
        let offset = offset.clone();
        let container_ref = container_ref.clone();
        Callback::from(move |e: yew::events::WheelEvent| {
            e.prevent_default();
            e.stop_propagation();
            let old_zoom = *zoom;
            if let Some(el) = container_ref.cast::<web_sys::Element>() {
                let rect = el.get_bounding_client_rect();
                let mut dy = e.delta_y();
                // Normalize delta based on deltaMode (0=pixel,1=line,2=page)
                match e.delta_mode() { 1 => dy *= 16.0, 2 => dy *= rect.height(), _ => {} }
                let factor = (-dy * 0.001).exp();
                let new_zoom = (old_zoom * factor).clamp(0.3, 3.5);
                if (new_zoom - old_zoom).abs() < 1e-6 { return; }
                let bx = e.client_x() as f64 - rect.left();
                let by = e.client_y() as f64 - rect.top();
                let (ox, oy) = *offset;
                // Transform: scale(s) translate(ox,oy) => screen = world*s + (ox,oy)
                let world_x = (bx - ox) / old_zoom;
                let world_y = (by - oy) / old_zoom;
                let new_ox = bx - world_x * new_zoom;
                let new_oy = by - world_y * new_zoom;
                offset.set((new_ox, new_oy));
                zoom.set(new_zoom);
            }
        })
    };

    let stop_mouse_down = Callback::from(|e: yew::events::MouseEvent| e.stop_propagation());

    // Recenter (Origin): root world (0,0) -> screen = offset, so set offset to viewport center
    let recenter_root = {
        let offset = offset.clone();
        let container_ref = container_ref.clone();
        Callback::from(move |_| {
            if let Some(el) = container_ref.cast::<web_sys::Element>() {
                let rect = el.get_bounding_client_rect();
                offset.set((rect.width()/2.0, rect.height()/2.0));
            } else { offset.set((0.0,0.0)); }
        })
    };

    // --- Viewport / transform ---
    let (ox, oy) = *offset;
    let scale = *zoom;
    let svg_edges = html! {<svg style="position:absolute; inset:0; overflow:visible; pointer-events:none;" width="100%" height="100%">{ for edge_svg }</svg>};

    html! {
        <div ref={container_ref}
             style="position:relative; width:100vw; height:100vh; background:#0d1117; overflow:hidden; overscroll-behavior:contain; touch-action:none;"
             onwheel={wheel_cb}
             onmousedown={mousedown}
             onmousemove={mousemove}
             onmouseup={mouseup.clone()}
             onmouseleave={mouseup}
        >
            <div style="position:absolute; top:12px; right:12px; background:#161b22dd; border:1px solid #30363d; border-radius:8px; padding:8px; display:flex; flex-direction:column; gap:6px; z-index:20;" onmousedown={stop_mouse_down.clone()}>
                <div style="font-weight:600; font-size:14px;">{ format!("Research: {}", research) }</div>
                <div style="display:flex; gap:6px;">
                    <button onclick={{ let cb=props.to_run.clone(); Callback::from(move |_| cb.emit(())) }}> {"Back"} </button>
                    <button onclick={respec_cb} style="background:#1b2733; border:1px solid #244a68;">{"Respec"}</button>
                </div>
                <div style="display:flex; gap:4px;">
                    <button onclick={recenter_root.clone()}> {"Origin"} </button>
                    <button onclick={{ let zoom=zoom.clone(); Callback::from(move |_| zoom.set((*zoom*1.25).clamp(0.3,3.5))) }}> {"+"} </button>
                    <button onclick={{ let zoom=zoom.clone(); Callback::from(move |_| zoom.set((*zoom*0.8).clamp(0.3,3.5))) }}> {"-"} </button>
                </div>
            </div>
            <div style={format!("position:absolute; inset:0; cursor:{};", if *dragging {"grabbing"} else {"grab"})}></div>
            <div style={format!("position:absolute; inset:0; transform:translate({}px, {}px) scale({}); transform-origin:0 0;", ox, oy, scale)}>
                { svg_edges }
                { for node_html }
                { tooltip }
            </div>
            { html!{} }
        </div>
    }
}
