use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LegendRowProps {
    pub color: &'static str,
    pub label: &'static str,
    #[prop_or(false)]
    pub highlight: bool,
}

#[function_component(LegendRow)]
pub fn legend_row(props: &LegendRowProps) -> Html {
    let bg = if props.highlight {
        "rgba(88,166,255,0.18)"
    } else {
        "transparent"
    };
    let weight = if props.highlight { "600" } else { "400" };
    html! {
        <div style={format!("display:flex; align-items:center; gap:8px; margin:3px 0; padding:2px 4px; border-radius:4px; background:{}; font-weight:{};", bg, weight)}>
            <span style={format!("display:inline-block; width:12px; height:12px; background:{}; border:1px solid #30363d; border-radius:2px;", props.color)}></span>
            <span>{ props.label }</span>
        </div>
    }
}
