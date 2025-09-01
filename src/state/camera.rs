// Camera state extracted from main.rs
#[derive(Debug, Clone)]
pub struct Camera {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub panning: bool,
    pub last_x: f64,
    pub last_y: f64,
    pub initialized: bool,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 2.5,
            offset_x: 0.0,
            offset_y: 0.0,
            panning: false,
            last_x: 0.0,
            last_y: 0.0,
            initialized: false,
        }
    }
}
