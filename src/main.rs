use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <canvas id="game-canvas" width="800" height="600"></canvas>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
