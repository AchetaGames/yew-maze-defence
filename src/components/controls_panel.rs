use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ControlsPanelProps {
    pub pause_label: String,
    pub on_toggle_pause: Callback<()>,
    pub to_upgrades: Callback<()>,
    pub tower_feedback: Option<String>,
    pub on_show_help: Callback<()>,
    pub on_open_settings: Callback<()>,
}

#[function_component]
pub fn ControlsPanel(props: &ControlsPanelProps) -> Html {
    let pause_cb = {
        let cb = props.on_toggle_pause.clone();
        Callback::from(move |_| cb.emit(()))
    };
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
    html! {<div style="position:absolute; top:12px; right:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:200px; display:flex; flex-direction:column; gap:6px;">
        <button onclick={pause_cb}>{ props.pause_label.clone() }</button>
        <button onclick={settings_cb}>{"Settings"}</button>
        <button onclick={upgrades_cb}>{"Upgrades"}</button>
        <button onclick={help_cb}>{"Help"}</button>
        <div style="font-size:11px; opacity:0.7;">{"Hotkey: 'T' place/remove tower"}</div>
        { if let Some(txt) = &props.tower_feedback { if !txt.is_empty() { html!{ <div style="font-size:11px; line-height:1.2; background:#1c2128; border:1px solid #30363d; padding:4px 6px; border-radius:6px;">{ txt.clone() }</div> } } else { html!{} } } else { html!{} } }
    </div>}
}
