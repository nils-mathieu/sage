use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::instance::Instance;
use crate::Result;

/// A surface on which it is possible to present things to the user.
pub struct Surface {
    loader: ash::extensions::khr::Surface,
    handle: vk::SurfaceKHR,
}

impl Surface {
    /// Creates a new [`Surface`] for the provided window.
    ///
    /// # Safety
    ///
    /// * The provided [`Instance`] must have been created with the [`RawDisplayHandle`] associated
    /// to `window`.
    ///
    /// * The created [`Surface`] must be destroyed before the `window` is destroyed.
    pub unsafe fn new<W>(instance: &Instance, window: &W) -> Result<Self>
    where
        W: HasRawDisplayHandle + HasRawWindowHandle,
    {
        Self::_new(
            instance,
            window.raw_window_handle(),
            window.raw_display_handle(),
        )
    }

    fn _new(
        instance: &Instance,
        win_handle: RawWindowHandle,
        disp_handle: RawDisplayHandle,
    ) -> Result<Self> {
        let loader = ash::extensions::khr::Surface::new(instance.entry(), instance.instance());
        let handle = unsafe {
            ash_window::create_surface(
                instance.entry(),
                instance.instance(),
                disp_handle,
                win_handle,
                None,
            )?
        };

        Ok(Self { loader, handle })
    }

    /// Returns a reference to the inner [`ash::extensions::khr::Surface`].
    pub fn loader(&self) -> &ash::extensions::khr::Surface {
        &self.loader
    }

    /// Returns the inner [`vk::SurfaceKHR`].
    #[inline(always)]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.handle
    }
}

impl Drop for Surface {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.handle, None) }
    }
}
