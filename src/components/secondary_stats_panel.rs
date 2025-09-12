use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct SecondaryStatsPanelProps {
    pub run_id: u64,
    pub enemy_count: usize,
    pub path_len: usize,
    pub path_nodes_text: Option<String>,
    pub show: bool,
}

#[function_component]
pub fn SecondaryStatsPanel(props: &SecondaryStatsPanelProps) -> Html {
    if !props.show {
        return html! {};
    }
    html! {<div style="position:absolute; left:12px; bottom:150px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px 10px; min-width:210px; display:flex; flex-direction:column; gap:4px; font-size:12px; line-height:1.3;">
        <div style="display:flex; justify-content:space-between; gap:12px;"><span style="opacity:0.7;">{"Run"}</span><span style="color:#d29922; font-weight:600;">{props.run_id}</span></div>
        <div style="display:flex; justify-content:space-between; gap:12px;"><span style="opacity:0.7;">{"Enemies"}</span><span style="color:#f85149; font-weight:600;">{props.enemy_count}</span></div>
        <div style="display:flex; justify-content:space-between; gap:12px;"><span style="opacity:0.7;">{"Path Len"}</span><span style="color:#58a6ff; font-weight:600;">{props.path_len}</span></div>
    </div> }
}
