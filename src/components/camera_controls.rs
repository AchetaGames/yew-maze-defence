use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct CameraControlsProps {
    pub on_zoom_in: Callback<()>,
    pub on_zoom_out: Callback<()>,
    pub on_pan_left: Callback<()>,
    pub on_pan_right: Callback<()>,
    pub on_pan_up: Callback<()>,
    pub on_pan_down: Callback<()>,
    pub on_center: Callback<()>,
}

#[function_component(CameraControls)]
pub fn camera_controls(props: &CameraControlsProps) -> Html {
    let zi = {
        let cb = props.on_zoom_in.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let zo = {
        let cb = props.on_zoom_out.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let pl = {
        let cb = props.on_pan_left.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let pr = {
        let cb = props.on_pan_right.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let pu = {
        let cb = props.on_pan_up.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let pd = {
        let cb = props.on_pan_down.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let cc = {
        let cb = props.on_center.clone();
        Callback::from(move |_| cb.emit(()))
    };
    html! {<div style="position:absolute; left:12px; bottom:12px; background:rgba(22,27,34,0.9); border:1px solid #30363d; border-radius:8px; padding:8px; display:flex; gap:6px; align-items:center;">
        <button onclick={zo}> {"-"} </button>
        <button onclick={zi}> {"+"} </button>
        <span style="width:8px;"></span>
        <button onclick={pr}> {"←"} </button>
        <button onclick={pd}> {"↑"} </button>
        <button onclick={pu}> {"↓"} </button>
        <button onclick={pl}> {"→"} </button>
        <span style="width:8px;"></span>
        <button onclick={cc}> {"Center"} </button>
    </div>}
}
