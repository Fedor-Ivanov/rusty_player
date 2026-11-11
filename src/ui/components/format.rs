//! Formatting values for display.

/// Seconds as `mm:ss` or `hh:mm:ss`.
pub fn format_secs(seconds: f32) -> String {
    let seconds = seconds.max(0.0) as u64;
    let (h, m, s) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);

    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
