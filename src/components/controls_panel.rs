use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ControlsPanelProps {
    pub to_upgrades: Callback<()>,
    pub on_show_help: Callback<()>,
    pub on_open_settings: Callback<()>,
}

#[function_component]
pub fn ControlsPanel(props: &ControlsPanelProps) -> Html {
    let upgrades_cb = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let help_cb = {
        let cb = props.on_show_help.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let settings_cb = {
        let cb = props.on_open_settings.clone();
        Callback::from(move |_| cb.emit(()))
    };
    html! {<div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:10px 12px; min-width:170px; display:flex; flex-direction:column; gap:6px;">
        <button onclick={settings_cb} style="display:flex; align-items:center; gap:6px;">{"⚙"}<span>{"Settings"}</span></button>
        <button onclick={upgrades_cb} style="display:flex; align-items:center; gap:6px;">{"🧬"}<span>{"Upgrades"}</span></button>
        <button onclick={help_cb} style="display:flex; align-items:center; gap:6px;">{"❓"}<span>{"Help"}</span></button>
    </div>}
}
