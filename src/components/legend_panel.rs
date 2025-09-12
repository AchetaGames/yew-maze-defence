use super::legend::LegendRow;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LegendPanelProps {
    pub has_start: bool,
    pub has_entrance: bool,
    pub has_exit: bool,
    pub has_indestructible: bool,
    pub has_basic: bool,
    pub has_gold: bool,
    pub has_empty: bool,
    pub has_wall: bool,
    // Hover info / highlight flags
    pub hover_text: Option<String>,
    #[prop_or(false)]
    pub highlight_start: bool,
    #[prop_or(false)]
    pub highlight_entrance: bool,
    #[prop_or(false)]
    pub highlight_exit: bool,
    #[prop_or(false)]
    pub highlight_indestructible: bool,
    #[prop_or(false)]
    pub highlight_basic: bool,
    #[prop_or(false)]
    pub highlight_gold: bool,
    #[prop_or(false)]
    pub highlight_empty: bool,
    #[prop_or(false)]
    pub highlight_wall: bool,
}

#[function_component]
pub fn LegendPanel(props: &LegendPanelProps) -> Html {
    html! {<div style="position:absolute; right:12px; bottom:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; min-width:170px;">
        <div style="font-weight:600; margin-bottom:4px;">{"Legend"}</div>
        { if let Some(t) = &props.hover_text { html!{<div style="font-size:11px; color:#8b949e; margin-bottom:6px;">{t}</div>} } else { html!{} } }
        { if props.has_start { html!{ <LegendRow color="#58a6ff" label="Start" highlight={props.highlight_start}/> } } else { html!{} } }
        { if props.has_entrance { html!{ <LegendRow color="#2ea043" label="Entrance" highlight={props.highlight_entrance}/> } } else { html!{} } }
        { if props.has_exit { html!{ <LegendRow color="#f0883e" label="Exit" highlight={props.highlight_exit}/> } } else { html!{} } }
        { if props.has_indestructible { html!{ <LegendRow color="#3c4454" label="Indestructible" highlight={props.highlight_indestructible}/> } } else { html!{} } }
        { if props.has_basic { html!{ <LegendRow color="#1d2430" label="Rock" highlight={props.highlight_basic}/> } } else { html!{} } }
        { if props.has_gold { html!{ <LegendRow color="#4d3b1f" label="Gold Rock" highlight={props.highlight_gold}/> } } else { html!{} } }
        { if props.has_empty { html!{ <LegendRow color="#082235" label="Path" highlight={props.highlight_empty}/> } } else { html!{} } }
        { if props.has_wall { html!{ <LegendRow color="#2a2f38" label="Wall" highlight={props.highlight_wall}/> } } else { html!{} } }
    </div>}
}
