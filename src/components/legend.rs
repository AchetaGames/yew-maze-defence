use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LegendRowProps {
    pub color: &'static str,
    pub label: &'static str,
}

#[function_component(LegendRow)]
pub fn legend_row(props: &LegendRowProps) -> Html {
    html! { <div style="display:flex; align-items:center; gap:8px; margin:3px 0;"> <span style={format!("display:inline-block; width:12px; height:12px; background:{}; border:1px solid #30363d; border-radius:2px;", props.color)}></span> <span>{ props.label }</span> </div> }
}
