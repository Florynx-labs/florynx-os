pub fn lerp(current: f32, target: f32, alpha: f32) -> f32 {
    current + (target - current) * alpha
}

pub fn tick_value(current: f32, target: f32, dt_ms: u32, speed_per_sec: f32) -> f32 {
    let dt = (dt_ms as f32) / 1000.0;
    let alpha = (speed_per_sec * dt).clamp(0.0, 1.0);
    lerp(current, target, alpha)
}
