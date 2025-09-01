// Touch/pinch gesture state extracted from main.rs
#[derive(Default, Debug, Clone)]
pub struct TouchState {
    pub single_active: bool,
    pub pinch: bool,
    pub _start_pinch_dist: f64,
    pub _start_zoom: f64,
    pub _world_center_x: f64,
    pub _world_center_y: f64,
    pub last_touch_x: f64,
    pub last_touch_y: f64,
}
