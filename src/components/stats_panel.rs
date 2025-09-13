use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct StatsPanelProps {
    pub gold: u64,
    pub life: u32,
    pub research: u64,
}

#[function_component]
pub fn StatsPanel(props: &StatsPanelProps) -> Html {
    let row_style = "display:flex; align-items:center; gap:8px;"; // icon | label | value
    let icon_style = "width:20px; text-align:center; flex-shrink:0;";
    let label_style = "flex:1; font-weight:500;";
    let value_style =
        "min-width:70px; text-align:right; font-variant-numeric:tabular-nums; font-weight:600;";
    html! {
        <div style="position:absolute; top:12px; left:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:10px 14px; min-width:230px; display:flex; flex-direction:column; gap:10px; font-size:14px;">
            <div style={row_style}>
                <span style={format!("{} color:#d4af37;", icon_style)}>{"🪙"}</span>
                <span style={format!("{} color:#d4af37;", label_style)}>{"Gold"}</span>
                <span style={format!("{} color:#d4af37;", value_style)}>{ props.gold }</span>
            </div>
            <div style={row_style}>
                <span style={format!("{} color:#f85149;", icon_style)}>{"❤"}</span>
                <span style={format!("{} color:#f85149;", label_style)}>{"Life"}</span>
                <span style={format!("{} color:#f85149;", value_style)}>{ props.life }</span>
            </div>
            <div style={row_style}>
                <span style={format!("{} color:#58a6ff;", icon_style)}>{"🔬"}</span>
                <span style={format!("{} color:#58a6ff;", label_style)}>{"Research"}</span>
                <span style={format!("{} color:#58a6ff;", value_style)}>{ props.research }</span>
            </div>
        </div>
    }
}
