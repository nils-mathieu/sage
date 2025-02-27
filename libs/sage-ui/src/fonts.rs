use {
    cosmic_text::FontSystem,
    sage_core::{TypeUuid, Uuid, app::Global},
};

/// A global resource that caches loaded fonts.
#[derive(Debug)]
pub struct Fonts(FontSystem);

impl Fonts {
    /// Returns the inner [`FontSystem`].
    #[inline]
    pub fn as_font_system_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.0
    }
}

impl Default for Fonts {
    fn default() -> Self {
        Self(cosmic_text::FontSystem::new())
    }
}

unsafe impl TypeUuid for Fonts {
    const UUID: Uuid = Uuid::from_u128(0xd10f9be53546ecca4bca9de1f545416f);
}

impl Global for Fonts {}
