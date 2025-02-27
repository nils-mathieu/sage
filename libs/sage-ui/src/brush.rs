use sage_color::LinearSrgba;

/// A brush that can be used to paint a shape.
#[derive(Clone, Debug)]
pub enum Brush {
    /// A solid color.
    Solid(LinearSrgba),
}

impl Brush {
    /// Returns whether the brush contains parts that are partially transparent.
    pub fn is_transparent(&self) -> bool {
        match self {
            Brush::Solid(color) => color.alpha < 1.0,
        }
    }
}
