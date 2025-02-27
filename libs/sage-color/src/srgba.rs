use crate::{LinearSrgba, gamma_function_inverse};

/// A color, encoded in the sRGB (non-linear) color space, with an alpha channel.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Zeroable, bytemuck::Pod))]
#[repr(C, align(16))]
pub struct Srgba {
    /// The red component of the color.
    pub red: f32,
    /// The green component of the color.
    pub green: f32,
    /// The blue component of the color.
    pub blue: f32,
    /// The alpha component of the color.
    pub alpha: f32,
}

impl Srgba {
    /// The color white.
    ///
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    /// The color black.
    ///
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);

    /// The color red.
    ///
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);

    /// The color green.
    ///
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);

    /// The color blue.
    ///
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);

    /// The color yellow.
    ///
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);

    /// The color cyan.
    ///
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);

    /// The color magenta.
    ///
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);

    /// The color transparent.
    ///
    /// <div style="background-color:rgba(0%, 0%, 0%, 0); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    /// Creates a new [`Srgba`] color from the provided components.
    #[inline]
    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a new [`Srgba`] color from the provided components, with an alpha of `1.0`.
    #[inline]
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// Creates a new [`Srgba`] color from the provided grayscale value.
    #[inline]
    pub const fn gray(value: f32) -> Self {
        Self {
            red: value,
            green: value,
            blue: value,
            alpha: 1.0,
        }
    }
}

impl From<LinearSrgba> for Srgba {
    fn from(value: LinearSrgba) -> Self {
        Self {
            red: gamma_function_inverse(value.red),
            green: gamma_function_inverse(value.green),
            blue: gamma_function_inverse(value.blue),
            alpha: value.alpha,
        }
    }
}
