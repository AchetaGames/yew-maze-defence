use crate::util::format_time;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct TimeDisplayProps {
    pub time_survived: u64,
    pub pause_label: String,
    pub on_toggle_pause: Callback<()>,
}

#[function_component(TimeDisplay)]
pub fn time_display(props: &TimeDisplayProps) -> Html {
    let pause_cb = {
        let cb = props.on_toggle_pause.clone();
        Callback::from(move |_| cb.emit(()))
    };
    html! {<div style="position:absolute; top:12px; left:50%; transform:translateX(-50%); display:flex; flex-direction:column; align-items:center; gap:6px;">
        <div style="font-size:20px; font-weight:600;">{ format_time(props.time_survived) }</div>
        <button onclick={pause_cb} style="padding:4px 10px; font-size:12px;">{ props.pause_label.clone() }</button>
    </div>}
}
