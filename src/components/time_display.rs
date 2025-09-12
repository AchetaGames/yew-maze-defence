use crate::util::format_time;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct TimeDisplayProps {
    pub time_survived: u64,
}

#[function_component(TimeDisplay)]
pub fn time_display(props: &TimeDisplayProps) -> Html {
    html! {<div style="position:absolute; top:12px; left:50%; transform:translateX(-50%); font-size:20px; font-weight:600;">{ format_time(props.time_survived) }</div>}
}
