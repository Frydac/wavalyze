// simple comparison, good enough in many usecases with an appropriate epsilon for the context
pub fn near_absolute(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() < epsilon
}

// based on https://floating-point-gui.de/errors/comparison/
// epsilon is relative to the magnitude of the input numbers
pub fn near_relative(a: f32, b: f32, epsilon: f32) -> bool {
    if a == b {
        true
    } else if (a == 0.0 || b == 0.0) || (a.abs() + b.abs()) < f32::MIN_POSITIVE {
        (a - b).abs() < epsilon * f32::MIN_POSITIVE
    } else {
        (a - b).abs() / (a.abs() + b.abs()) < epsilon
    }
}
