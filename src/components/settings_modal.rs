use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct SettingsModalProps {
    pub show: bool,
    pub on_close: Callback<()>,
    pub show_path: bool,
    pub on_toggle_path: Callback<()>,
    pub show_damage_numbers: bool,
    pub on_toggle_damage_numbers: Callback<()>,
    pub on_restart_run: Callback<()>,
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
    let restart_cb = {
        let cb = props.on_restart_run.clone();
        Callback::from(move |_| cb.emit(()))
    };

    html! {<div style="position:absolute; inset:0; display:flex; align-items:center; justify-content:center; background:rgba(0,0,0,0.55); z-index:50;">
        <div style="background:#161b22; border:1px solid #30363d; border-radius:12px; padding:16px 20px; min-width:320px; max-width:420px; display:flex; flex-direction:column; gap:14px;">
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
            </div>
            <div style="display:flex; gap:8px; flex-wrap:wrap;">
                <button onclick={restart_cb} style="background:#d29922; border:1px solid #9e6a00;">{"Reset Run"}</button>
                <button onclick={close_cb} style="flex:1;">{"Done"}</button>
            </div>
            <div style="font-size:11px; line-height:1.4; opacity:0.7;">{"Settings are saved locally (browser storage). More options coming soon."}</div>
        </div>
    </div>}
}
