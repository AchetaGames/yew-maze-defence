use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct SettingsModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub show_path: bool,
    pub on_toggle_path: Callback<()>,
    pub show_damage_numbers: bool,
    pub on_toggle_damage_numbers: Callback<()>,
    pub show_secondary_stats: bool,
    pub on_toggle_secondary_stats: Callback<()>,
    pub on_hard_reset: Callback<()>,
}

#[function_component]
pub fn SettingsModal(props: &SettingsModalProps) -> Html {
    if !props.show {
        return html! {};
    }

    let close_cb = {
        let cb = props.on_close.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let toggle_path_cb = {
        let cb = props.on_toggle_path.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let toggle_damage_cb = {
        let cb = props.on_toggle_damage_numbers.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let toggle_secondary_cb = {
        let cb = props.on_toggle_secondary_stats.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let hard_reset_cb = {
        let cb = props.on_hard_reset.clone();
        Callback::from(move |_| {
            if let Some(win) = web_sys::window() {
                if win
                    .confirm_with_message(
                        "This will WIPE all progress (upgrades, research, settings) and start fresh. Are you sure?",
                    )
                    .unwrap_or(false)
                {
                    cb.emit(());
                }
            } else {
                cb.emit(());
            }
        })
    };

    html! {<div style="position:absolute; inset:0; display:flex; align-items:center; justify-content:center; background:rgba(0,0,0,0.55); z-index:50;">
        <div style="background:#161b22; border:1px solid #30363d; border-radius:12px; padding:16px 20px; min-width:340px; max-width:480px; display:flex; flex-direction:column; gap:14px;">
            <div style="display:flex; justify-content:space-between; align-items:center;">
                <h3 style="margin:0; font-size:18px;">{"Settings"}</h3>
                <button onclick={close_cb.clone()} style="padding:4px 8px;">{"Close"}</button>
            </div>
            <div style="display:flex; flex-direction:column; gap:10px;">
                <label style="display:flex; align-items:center; gap:8px; cursor:pointer;">
                    <input type="checkbox" checked={props.show_path} onclick={toggle_path_cb} />
                    <span>{"Show Path"}</span>
                </label>
                <label style="display:flex; align-items:center; gap:8px; cursor:pointer;">
                    <input type="checkbox" checked={props.show_damage_numbers} onclick={toggle_damage_cb} />
                    <span>{"Show Damage Numbers"}</span>
                </label>
                <label style="display:flex; align-items:center; gap:8px; cursor:pointer;">
                    <input type="checkbox" checked={props.show_secondary_stats} onclick={toggle_secondary_cb} />
                    <span>{"Show Secondary Stats"}</span>
                </label>
            </div>
            <div style="display:flex; gap:8px; flex-wrap:wrap;">
                <button onclick={hard_reset_cb} style="background:#f85149; border:1px solid #b62324; color:#fff; flex:1;">{"Hard Reset (Wipe Progress)"}</button>
                <button onclick={close_cb} style="flex:0 0 auto;">{"Done"}</button>
            </div>
            <div style="font-size:11px; line-height:1.4; opacity:0.7;">{"Hard Reset removes all saved upgrades, research, and settings (including intro seen)."}</div>
        </div>
    </div>}
}
