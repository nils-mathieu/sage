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

/// A color, encoded in the sRGB (non-linear) color space, with an alpha channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Zeroable, bytemuck::Pod))]
#[repr(C, align(4))]
pub struct Srgba8 {
    /// The red component of the color.
    pub red: u8,
    /// The green component of the color.
    pub green: u8,
    /// The blue component of the color.
    pub blue: u8,
    /// The alpha component of the color.
    pub alpha: u8,
}

impl Srgba8 {
    /// The color white.
    ///
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// The color black.
    ///
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Self = Self::rgb(0, 0, 0);

    /// The color red.
    ///
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: Self = Self::rgb(255, 0, 0);

    /// The color green.
    ///
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: Self = Self::rgb(0, 255, 0);

    /// The color blue.
    ///
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: Self = Self::rgb(0, 0, 255);

    /// The color yellow.
    ///
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: Self = Self::rgb(255, 255, 0);

    /// The color cyan.
    ///
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: Self = Self::rgb(0, 255, 255);

    /// The color magenta.
    ///
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);

    /// The color transparent.
    ///
    /// <div style="background-color:rgba(0%, 0%, 0%, 0); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    /// Creates a new [`Srgba8`] color from the provided components.
    #[inline]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a new [`Srgba8`] color from the provided components, with an alpha of `255`.
    #[inline]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// Returns whether the color is transparent.
    #[inline]
    pub const fn is_transparent(&self) -> bool {
        self.alpha == 0
    }
}

impl From<Srgba> for Srgba8 {
    fn from(value: Srgba) -> Self {
        Self {
            red: (value.red * 255.0).round() as u8,
            green: (value.green * 255.0).round() as u8,
            blue: (value.blue * 255.0).round() as u8,
            alpha: (value.alpha * 255.0).round() as u8,
        }
    }
}

impl From<Srgba8> for Srgba {
    fn from(value: Srgba8) -> Self {
        Self {
            red: value.red as f32 / 255.0,
            green: value.green as f32 / 255.0,
            blue: value.blue as f32 / 255.0,
            alpha: value.alpha as f32 / 255.0,
        }
    }
}

impl From<Srgba8> for LinearSrgba {
    #[inline]
    fn from(value: Srgba8) -> Self {
        LinearSrgba::from(Srgba::from(value))
    }
}

impl From<LinearSrgba> for Srgba8 {
    #[inline]
    fn from(value: LinearSrgba) -> Self {
        Srgba8::from(Srgba::from(value))
    }
}
