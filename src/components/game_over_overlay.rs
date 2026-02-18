use crate::model::{MetaRecords, RunStats};
use crate::util::format_time;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct GameOverOverlayProps {
    pub show: bool,
    pub time_survived: u64,
    pub loops_completed: u32,
    pub blocks_mined: u32,
    pub restart: Callback<()>,
    pub to_upgrades: Callback<()>,
}

fn load_records() -> MetaRecords {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(store)) = win.local_storage() {
            if let Ok(Some(raw)) = store.get_item("md_records") {
                if let Ok(r) = serde_json::from_str::<MetaRecords>(&raw) {
                    return r;
                }
            }
        }
    }
    MetaRecords::default()
}

fn save_records(records: &MetaRecords) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(store)) = win.local_storage() {
            if let Ok(s) = serde_json::to_string(records) {
                let _ = store.set_item("md_records", &s);
            }
        }
    }
}

#[function_component]
pub fn GameOverOverlay(props: &GameOverOverlayProps) -> Html {
    if !props.show {
        return html! {};
    }

    let records_updated = use_state(|| false);
    let new_records = use_state(|| Vec::<String>::new());
    let records = use_state(MetaRecords::default);

    {
        let show = props.show;
        let time = props.time_survived;
        let loops = props.loops_completed;
        let blocks = props.blocks_mined;
        let records_updated = records_updated.clone();
        let new_records = new_records.clone();
        let records = records.clone();
        use_effect_with(show, move |_| {
            if show && !*records_updated {
                let mut r = load_records();
                let stats = RunStats {
                    time_survived_secs: time,
                    loops_completed: loops,
                    blocks_mined: blocks,
                };
                let nr = r.update_from_stats(&stats);
                save_records(&r);
                new_records.set(nr.iter().map(|s| s.to_string()).collect());
                records.set(r);
                records_updated.set(true);
            }
            || ()
        });
    }

    let restart_cb = props.restart.clone();
    let restart_btn = Callback::from(move |_| restart_cb.emit(()));
    let upgrades_btn = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_| cb.emit(()))
    };

    let nr = &*new_records;
    let rec = &*records;
    let record_marker = |key: &str| -> Html {
        if nr.iter().any(|s| s == key) {
            html! { <span style="color:#d29922; font-weight:bold; margin-left:6px;">{"NEW!"}</span> }
        } else {
            html! {}
        }
    };

    html! {
        <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); background:rgba(0,0,0,0.85); border:2px solid #f85149; padding:24px 32px; border-radius:12px; text-align:center; min-width:320px;">
            <h2 style="margin:0 0 12px 0; color:#f85149;">{"Game Over"}</h2>
            <p style="margin:4px 0;">{ format!("Time Survived: {}", format_time(props.time_survived)) }{ record_marker("time") }</p>
            <p style="margin:4px 0;">{ format!("Loops Completed: {}", props.loops_completed) }{ record_marker("loops") }</p>
            <p style="margin:4px 0;">{ format!("Blocks Mined: {}", props.blocks_mined) }{ record_marker("blocks") }</p>
            if rec.total_runs > 1 {
                <div style="margin-top:12px; border-top:1px solid #30363d; padding-top:8px;">
                    <p style="margin:2px 0; font-size:0.85em; color:#8b949e;">{ format!("Best Time: {}", format_time(rec.best_time_secs)) }</p>
                    <p style="margin:2px 0; font-size:0.85em; color:#8b949e;">{ format!("Best Loops: {}", rec.best_loops) }</p>
                    <p style="margin:2px 0; font-size:0.85em; color:#8b949e;">{ format!("Best Blocks: {}", rec.best_blocks_mined) }</p>
                    <p style="margin:2px 0; font-size:0.85em; color:#8b949e;">{ format!("Total Runs: {}", rec.total_runs) }</p>
                </div>
            }
            <div style="margin-top:16px; display:flex; gap:12px; justify-content:center;">
                <button onclick={restart_btn}>{"Restart Run"}</button>
                <button onclick={upgrades_btn}>{"Upgrades"}</button>
            </div>
        </div>
    }
}
