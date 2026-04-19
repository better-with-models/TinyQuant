//! `BackendPreference`: caller-facing backend and adapter selection hint.

use crate::error::TinyQuantGpuError;

/// Controls which wgpu backend and physical adapter `WgpuBackend` selects.
///
/// Passed to [`WgpuBackend::new_with_preference`] and
/// [`WgpuBackend::enumerate_adapters`].
///
/// # Platform notes
///
/// Not all variants are available on all platforms:
/// - [`Metal`](Self::Metal) is only available on macOS/iOS.
/// - [`Dx12`](Self::Dx12) is only available on Windows.
/// - [`Vulkan`](Self::Vulkan) requires a Vulkan-capable driver (Linux,
///   Windows, Android; optional on macOS via MoltenVK).
///
/// When the preferred backend has no adapter,
/// [`new_with_preference`](crate::WgpuBackend::new_with_preference) returns
/// [`TinyQuantGpuError::NoPreferredAdapter`] rather than silently falling
/// through to another backend.  Use [`Auto`](Self::Auto) for transparent
/// fallback behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendPreference {
    /// Select the highest-performance available adapter automatically.
    ///
    /// Tries primary backends (Metal → DX12 → Vulkan → WebGPU) in
    /// platform-native preference order.  Falls back to secondary backends
    /// (GL) if no primary adapter is found.  This is the behaviour of
    /// [`WgpuBackend::new`](crate::WgpuBackend::new).
    #[default]
    Auto,
    /// Prefer a Vulkan adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) if no
    /// Vulkan driver is available.
    Vulkan,
    /// Prefer a Metal adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) on
    /// non-Apple platforms.
    Metal,
    /// Prefer a DirectX 12 adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) on
    /// non-Windows platforms.
    Dx12,
    /// Prefer the discrete GPU.  Falls back to integrated GPU, then
    /// virtual GPU, then software.  Uses
    /// [`wgpu::PowerPreference::HighPerformance`] within the primary
    /// backend set.
    HighPerformance,
    /// Prefer the integrated or battery-saving GPU.  Uses
    /// [`wgpu::PowerPreference::LowPower`] within the primary backend set.
    LowPower,
    /// Force a software (CPU-emulated) renderer.  Never selects a
    /// physical GPU.  Available wherever wgpu compiles the GL backend
    /// (e.g., Mesa llvmpipe on Linux, ANGLE on Windows).  Useful for
    /// headless testing.
    Software,
}

impl BackendPreference {
    /// Returns the `wgpu::Backends` bitset to use when creating the wgpu
    /// instance for this preference.
    pub(crate) fn to_backends(self) -> wgpu::Backends {
        match self {
            Self::Auto | Self::HighPerformance | Self::LowPower => wgpu::Backends::all(),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Metal => wgpu::Backends::METAL,
            Self::Dx12 => wgpu::Backends::DX12,
            Self::Software => wgpu::Backends::GL,
        }
    }

    /// Returns the `wgpu::PowerPreference` for this preference.
    pub(crate) fn to_power_preference(self) -> wgpu::PowerPreference {
        match self {
            Self::LowPower => wgpu::PowerPreference::LowPower,
            _ => wgpu::PowerPreference::HighPerformance,
        }
    }

    /// Returns the error to use when no adapter is found for this preference.
    pub(crate) fn no_adapter_error(self) -> TinyQuantGpuError {
        match self {
            Self::Auto | Self::HighPerformance | Self::LowPower => TinyQuantGpuError::NoAdapter,
            _ => TinyQuantGpuError::NoPreferredAdapter,
        }
    }
}

/// A discovered wgpu adapter candidate returned by
/// [`WgpuBackend::enumerate_adapters`](crate::WgpuBackend::enumerate_adapters).
///
/// Does not hold a live device handle — creating this struct does not
/// consume driver resources.
#[derive(Debug, Clone)]
pub struct AdapterCandidate {
    /// Driver-provided adapter name (e.g. `"NVIDIA GeForce RTX 3060"`).
    pub name: String,
    /// The wgpu backend this adapter belongs to.
    pub backend: wgpu::Backend,
    /// Device type: discrete GPU, integrated GPU, virtual GPU, or CPU (software renderer).
    pub device_type: wgpu::DeviceType,
    /// PCI vendor ID (0 for software adapters).
    pub vendor: u32,
    /// PCI device ID (0 for software adapters).
    pub device: u32,
}
