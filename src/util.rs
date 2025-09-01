// Utility helpers extracted from main.rs

pub fn format_time(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{:01}:{:02}:{:02}", h, m, s)
    } else if m > 0 {
        format!("{:02}:{:02}", m, s)
    } else {
        format!("{}s", s)
    }
}

pub fn clog(msg: &str) {
    // Debug logging disabled to reduce console spam
    let _ = msg; // keep param to avoid warnings
}
