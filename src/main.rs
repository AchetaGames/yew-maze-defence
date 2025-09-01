mod components;
mod model;
mod state;
mod util;

fn main() {
    yew::Renderer::<components::App>::new().render();
}
