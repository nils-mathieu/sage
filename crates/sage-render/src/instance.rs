//! The entry point to the Vulkan API.
//!
//! Creating an [`Instance`] is the first step to using Vulkan. It will be used to create other
//! rendering resources and objects that are not directly tied to a specific physical device.

use ash::vk;
use raw_window_handle::RawDisplayHandle;

use crate::{Error, Result};

use std::ffi::CStr;
use std::mem::ManuallyDrop;

/// The version of an application.
pub struct Version(u32);

impl Version {
    /// Creates a new [`Version`] instance from the provided value.
    ///
    /// This function should hardly every be used directly and is mostly provided for completeness.
    #[inline(always)]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Creates a new [`Version`] instance from the provided `variant`, `major`, `minor`, and
    /// `patch` numbers.
    ///
    /// This function is not *unsafe* (despite the name), but providing invalid values will result
    /// in an unspecified [`Version`] instance.
    ///
    /// The valid ranges of the arguments are specified in the documentation for [`Version::new`].
    #[inline(always)]
    pub const fn new_unchecked(variant: u32, major: u32, minor: u32, patch: u32) -> Self {
        Self::from_raw(vk::make_api_version(variant, major, minor, patch))
    }

    /// Creates a new [`Version`] instance from the provided `variant`, `major`, `minor`, and
    /// `patch` numbers.
    ///
    /// If any of the provided values are out of range, the function returns [`None`]. The range of
    /// valid values are specified in the documentation for [`Version::new`].
    pub const fn checked_new(variant: u32, major: u32, minor: u32, patch: u32) -> Option<Self> {
        if variant >= 8 || major >= 128 || minor >= 1024 || patch >= 4096 {
            None
        } else {
            Some(Self::new_unchecked(variant, major, minor, patch))
        }
    }

    /// Creates a new [`Version`] instance from the provided `variant`, `major`, `minor`, and
    /// `patch` numbers.
    ///
    /// # Panics
    ///
    /// This function panics if any of the provided values are invalid.
    ///
    /// * `variant` must be in the range `0..=7`.
    /// * `major` must be in the range `0..=127`.
    /// * `minor` must be in the range `0..=1023`.
    /// * `patch` must be in the range `0..=4095`.
    pub const fn new(variant: u32, major: u32, minor: u32, patch: u32) -> Self {
        match Self::checked_new(variant, major, minor, patch) {
            Some(v) => v,
            None => panic!("invalid version numbers"),
        }
    }

    /// Returns the variant number of the version.
    #[inline(always)]
    pub const fn variant(&self) -> u32 {
        vk::api_version_major(self.0)
    }

    /// Returns the major number of the version.
    #[inline(always)]
    pub const fn major(&self) -> u32 {
        vk::api_version_major(self.0)
    }

    /// Returns the minor number of the version.
    #[inline(always)]
    pub const fn minor(&self) -> u32 {
        vk::api_version_minor(self.0)
    }

    /// Returns the patch number of the version.
    #[inline(always)]
    pub const fn patch(&self) -> u32 {
        vk::api_version_patch(self.0)
    }

    /// Returns the raw value of the version.
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// The engine name that will be passed to Vulkan.
const ENGINE_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Sage Engine\0") };
const ENGINE_VERSION: Version = Version::new(0, 0, 0, 1);

/// Describes how to create an [`Instance`].
pub struct Config<'a> {
    /// The name of the application that will be passed to Vulkan.
    ///
    /// If this name contains a null character, the remainder of the string will be ignored (and
    /// this will save an allocation ;) ).
    pub app_name: &'a str,
    /// The version of the application that will be passed to Vulkan.
    pub app_version: Version,
    /// A optional [`RawDisplayHandle`] with which the instance must be compatible.
    pub supports_display: Option<RawDisplayHandle>,
    /// Whether validation layers should be enabled.
    pub validation: bool,
}

/// Represents a live Vulkan instance.
///
/// More information in the [top-level documentation](index.html).
pub struct Instance {
    // The inner instance must be dropped *before* the entry. I know the drop order of fields is
    // well-defined but I'm not confortable with relying on that.
    entry: ManuallyDrop<ash::Entry>,
    instance: ManuallyDrop<ash::Instance>,
}

impl Instance {
    /// Creates a new [`Instance`].
    pub fn new(config: &Config) -> Result<Self> {
        let mut app_name_container = Vec::new();
        let app_name = create_cstr(config.app_name.as_bytes(), &mut app_name_container);

        let entry = unsafe { ash::Entry::load() }.map_err(|err| match err {
            ash::LoadingError::LibraryLoadFailure(_) => Error::NoVulkan,
            ash::LoadingError::MissingEntryPoint(_) => Error::UnexpectedBehavior,
        })?;

        let api_version = match entry.try_enumerate_instance_version()? {
            Some(_) => vk::HEADER_VERSION_COMPLETE,
            // If the `vkEnumerateInstanceVersion` function is not available, then the current
            // version of Vulkan is 1.0.0. In that case, we *have to* request that same version, or
            // we'll get an error.
            None => vk::make_api_version(0, 1, 0, 0),
        };

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(config.app_version.raw())
            .engine_name(ENGINE_NAME)
            .engine_version(ENGINE_VERSION.raw())
            .api_version(api_version);

        // Enable validation layer when debug assertions are enabled, and that they are available.
        let mut enabled_layers = Vec::new();
        if config.validation {
            const VALIDATION_LAYER: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };

            let available_layers = entry.enumerate_instance_layer_properties()?;
            if available_layers
                .iter()
                .map(|props| unsafe { CStr::from_ptr(props.layer_name.as_ptr()) })
                .any(|name| name == VALIDATION_LAYER)
            {
                enabled_layers.push(VALIDATION_LAYER.as_ptr());
            } else {
                // TODO: use a proper logging system.
                eprintln!("WARN: validation layers are not available");
            }
        }

        let mut enabled_extensions = Vec::new();

        if let Some(disp_handle) = config.supports_display {
            let surface_extensions = ash_window::enumerate_required_extensions(disp_handle)
                .map_err(|_| Error::SurfaceNotSupported)?;
            enabled_extensions.extend_from_slice(surface_extensions);
        }

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&enabled_layers)
            .enabled_extension_names(&enabled_extensions);

        // SAFETY:
        //  We are storing the instance and the entry together in the same struct, and dropping
        //  order will be taken care of in the `Drop` implementation.
        let instance = unsafe { entry.create_instance(&instance_info, None)? };

        Ok(Self {
            entry: ManuallyDrop::new(entry),
            instance: ManuallyDrop::new(instance),
        })
    }

    /// Returns a reference to the inner [`ash::Instance`].
    #[inline(always)]
    pub fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    /// Returns a reference to the inner [`ash::Instance`].
    #[inline(always)]
    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }
}

impl Drop for Instance {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) }

        unsafe {
            ManuallyDrop::drop(&mut self.instance);
            ManuallyDrop::drop(&mut self.entry);
        }
    }
}

/// Tries to create a [`CStr`] instance from the provided bytes.
///
/// If those bytes contain a null byte, the remainder of the string is ignored and the function
/// returns those bytes. Otherwise, the bytes are copied into the provided buffer and a new
/// [`CStr`] instance is created from that buffer.
fn create_cstr<'a>(bytes: &'a [u8], buf: &'a mut Vec<u8>) -> &'a CStr {
    let checked = match bytes.iter().position(|&b| b == b'\0') {
        Some(i) => unsafe { bytes.get_unchecked(..=i) },
        None => {
            buf.clear();
            buf.reserve(bytes.len() + 1);
            buf.extend_from_slice(bytes);
            buf.push(b'\0');
            buf.as_slice()
        }
    };

    unsafe { CStr::from_bytes_with_nul_unchecked(checked) }
}
