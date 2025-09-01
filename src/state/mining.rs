// Mining progress state extracted from main.rs
#[derive(Default, Debug, Clone)]
pub struct Mining {
    pub tile_x: i32,
    pub tile_y: i32,
    pub required_secs: f64,
    pub elapsed_secs: f64,
    pub progress: f64,
    pub active: bool,
    pub mouse_down: bool,
}
