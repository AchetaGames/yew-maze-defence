use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct IntroOverlayProps {
    pub show: bool,
    pub game_over: bool,
    pub hide_intro: Callback<()>,
    pub to_upgrades: Callback<()>,
}

#[function_component(IntroOverlay)]
pub fn intro_overlay(props: &IntroOverlayProps) -> Html {
    if !props.show || props.game_over {
        return html! {};
    }
    let hide_cb = props.hide_intro.clone();
    let hide_btn = Callback::from(move |_| hide_cb.emit(()));
    let hide_cb2 = props.hide_intro.clone();
    let start_btn = Callback::from(move |_| hide_cb2.emit(()));
    let upgrades_cb = {
        let cb = props.to_upgrades.clone();
        Callback::from(move |_| cb.emit(()))
    };
    html! {
        <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); background:rgba(0,0,0,0.87); border:2px solid #30363d; padding:28px 36px; border-radius:14px; max-width:520px; width:90%; box-shadow:0 0 0 1px #1a1f24, 0 6px 18px rgba(0,0,0,0.6); font-size:14px; line-height:1.4;">
            <h2 style="margin:0 0 12px 0; font-size:22px; color:#58a6ff; text-align:center;">{"Maze Defence"}</h2>
            <p style="margin:4px 0 10px 0; text-align:center; opacity:0.85;">{"Build, mine, and defend. Survive as long as you can."}</p>
            <ul style="margin:0 0 12px 18px; padding:0; list-style:disc; display:flex; flex-direction:column; gap:4px;">
                <li>{"Hold Left Mouse on a Rock/Wall to mine it (progress bar fills)."}</li>
                <li>{"Click an Empty path tile to place a Rock (cannot block all paths)."}</li>
                <li>{"Hover a Rock and press 'T' to place a Tower (again to remove & refund)."}</li>
                <li>{"Press Space to Pause/Resume (also dismisses this screen)."}</li>
                <li>{"Zoom with wheel or +/- buttons; drag (right/middle mouse) to pan."}</li>
                <li>{"Enemies loop the path; each completed loop costs 1 Life."}</li>
                <li>{"Earn Research from kills; spend it in Upgrades between runs."}</li>
                <li>{"Boost Rocks (colors) unlock via upgrades and change tower stats."}</li>
            </ul>
            <div style="display:flex; gap:12px; justify-content:center; margin-top:8px;">
                <button onclick={start_btn}>{"Start"}</button>
                <button onclick={upgrades_cb}>{"Upgrades"}</button>
                <button onclick={hide_btn}>{"Close"}</button>
            </div>
            <div style="margin-top:12px; font-size:11px; opacity:0.6; text-align:center;">{"Tip: Place a tower early then mine to shape a longer looping path."}</div>
        </div>
    }
}
