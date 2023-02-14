use std::fmt;

use ash::vk;

/// An error that can occur when interacting with the Vulkan API.
#[derive(Debug, Clone)]
pub enum Error {
    /// The Vulkan API does not seem to be present on the current system.
    NoVulkan,
    /// The Vulkan API behaved in an unexpected way.
    UnexpectedBehavior,
    /// The system is out of host memory.
    OutOfMemory,
    /// The physical device is out of memory.
    OutOfGpuMemory,
    /// The requested surface is not supported.
    SurfaceNotSupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoVulkan => f.write_str("Vulkan API not found"),
            Self::UnexpectedBehavior => f.write_str("unexpected behavior from the Vulkan API"),
            Self::OutOfMemory => f.write_str("out of host memory"),
            Self::OutOfGpuMemory => f.write_str("out of GPU memory"),
            Self::SurfaceNotSupported => f.write_str("surface not supported"),
        }
    }
}

impl From<vk::Result> for Error {
    fn from(value: vk::Result) -> Self {
        match value {
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => Self::OutOfMemory,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => Self::OutOfGpuMemory,
            _ => Self::UnexpectedBehavior,
        }
    }
}

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
