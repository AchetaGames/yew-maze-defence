use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct TowerPanelProps {
    pub tower_feedback: Option<String>,
}

#[function_component]
pub fn TowerPanel(props: &TowerPanelProps) -> Html {
    html! {<div style="position:absolute; left:50%; bottom:28px; transform:translateX(-50%); background:rgba(22,27,34,0.92); border:1px solid #30363d; border-radius:10px; padding:10px 14px; display:flex; flex-direction:column; gap:6px; min-width:240px; text-align:center;">
        <div style="font-size:13px; opacity:0.8;">{"Press 'T' to place/remove tower on Rock/Wall"}</div>
        { if let Some(msg) = &props.tower_feedback {
            if !msg.is_empty() {
                html!{ <div style="font-size:12px; line-height:1.25; background:#1c2128; border:1px solid #30363d; padding:6px 8px; border-radius:6px;">{ msg.clone() }</div>}
            } else { html!{} }
        } else { html!{} } }
    </div> }
}
