use crate::util::format_time;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct GameOverOverlayProps {
    pub show: bool,
    pub time_survived: u64,
    pub loops_completed: u32,
    pub blocks_mined: u32,
    pub restart: Callback<()>,
    pub to_upgrades: Callback<()>,
}

#[function_component]
pub fn GameOverOverlay(props: &GameOverOverlayProps) -> Html {
    if !props.show {
        return html! {};
    }
    let restart_cb = props.restart.clone();
    let restart_btn = Callback::from(move |_| restart_cb.emit(()));
    let upgrades_btn = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_| cb.emit(()))
    };
    html! {
        <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); background:rgba(0,0,0,0.85); border:2px solid #f85149; padding:24px 32px; border-radius:12px; text-align:center; min-width:320px;">
            <h2 style="margin:0 0 12px 0; color:#f85149;">{"Game Over"}</h2>
            <p style="margin:4px 0;">{ format!("Time Survived: {}", format_time(props.time_survived)) }</p>
            <p style="margin:4px 0;">{ format!("Loops Completed: {}", props.loops_completed) }</p>
            <p style="margin:4px 0;">{ format!("Blocks Mined: {}", props.blocks_mined) }</p>
            <div style="margin-top:16px; display:flex; gap:12px; justify-content:center;">
                <button onclick={restart_btn}>{"Restart Run"}</button>
                <button onclick={upgrades_btn}>{"Upgrades"}</button>
            </div>
        </div>
    }
}
