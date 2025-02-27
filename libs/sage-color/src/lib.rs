//! Provides a bunch of color types useful for graphics programming.

mod srgba;
pub use self::srgba::*;

mod linear_srgb;
pub use self::linear_srgb::*;

/// Converts a non-linear sRGB value to a linear sRGB value via gamma correction.
pub fn gamma_function(x: f32) -> f32 {
    if x <= 0.0 {
        x
    } else if x <= 0.04045 {
        x / 12.92
    } else {
        f32::powf((x + 0.055) / 1.055, 2.4)
    }
}

/// Converts a linear sRGB value to a non-linear sRGB value via gamma correction.
pub fn gamma_function_inverse(x: f32) -> f32 {
    if x <= 0.0 {
        x
    } else if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * f32::powf(x, 1.0 / 2.4) - 0.055
    }
}
