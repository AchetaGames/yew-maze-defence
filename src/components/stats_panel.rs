use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct StatsPanelProps {
    pub gold: u64,
    pub life: u32,
    pub research: u64,
    pub run_id: u64,
    pub enemy_count: usize,
    pub path_len: usize,
    pub path_nodes_text: Option<String>,
}

#[function_component]
pub fn StatsPanel(props: &StatsPanelProps) -> Html {
    let nodes = props.path_nodes_text.as_ref().map(
        |s| html! { <div style="font-size:11px; opacity:0.7;">{format!("PathNodes: {}", s)}</div> },
    );
    html! {
        <div style="position:absolute; top:12px; left:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:180px; display:flex; flex-direction:column; gap:6px;">
            <div>{ format!("Gold: {}", props.gold) }</div>
            <div>{ format!("Life: {}", props.life) }</div>
            <div>{ format!("Research: {}", props.research) }</div>
            <div style="font-size:11px; opacity:0.7;">{ format!("Run: {}", props.run_id) }</div>
            <div style="font-size:11px; opacity:0.7;">{ format!("Enemies: {}", props.enemy_count) }</div>
            <div style="font-size:11px; opacity:0.7;">{ format!("Path: {}", props.path_len) }</div>
            { for nodes }
        </div>
    }
}
