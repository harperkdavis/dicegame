use std::ops::{Add, Mul, Sub};

pub fn lerp<T: Copy + Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T>>(
    a: T,
    b: T,
    t: T,
) -> T {
    a + (b - a) * t
}
