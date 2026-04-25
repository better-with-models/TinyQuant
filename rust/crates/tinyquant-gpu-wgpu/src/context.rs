//! `WgpuContext`: wgpu instance → adapter → device + queue setup.

use crate::{
    backend_preference::{AdapterCandidate, BackendPreference},
    error::TinyQuantGpuError,
};

/// Owns the wgpu `Device`, `Queue`, and adapter info for the GPU backend.
pub struct WgpuContext {
    /// The logical wgpu device used to create buffers, pipelines, and commands.
    pub device: wgpu::Device,
    /// The command queue used to submit GPU work.
    pub queue: wgpu::Queue,
    /// Metadata about the selected physical adapter (name, backend, etc.).
    pub adapter_info: wgpu::AdapterInfo,
}

impl WgpuContext {
    /// Initialise wgpu with [`BackendPreference::Auto`].
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        Self::new_with_preference(BackendPreference::Auto).await
    }

    /// Initialise wgpu using the given backend preference.
    ///
    /// Returns `Err(NoPreferredAdapter)` when the preference specifies a
    /// concrete backend (Vulkan, Metal, DX12, Software) and no adapter for
    /// that backend is present.
    pub async fn new_with_preference(pref: BackendPreference) -> Result<Self, TinyQuantGpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: pref.to_backends(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: pref.to_power_preference(),
                compatible_surface: None,
                force_fallback_adapter: matches!(pref, BackendPreference::Software),
            })
            .await
            .ok_or_else(|| pref.no_adapter_error())?;

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(TinyQuantGpuError::DeviceRequest)?;

        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }

    /// Synchronously enumerate all adapters for the given preference, in
    /// selection order (discrete GPU first, software last).
    ///
    /// Does not create a `Device` — safe to call before committing to a
    /// particular adapter.
    pub fn enumerate_adapters(pref: BackendPreference) -> Vec<AdapterCandidate> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: pref.to_backends(),
            ..Default::default()
        });
        let mut candidates: Vec<AdapterCandidate> = instance
            .enumerate_adapters(pref.to_backends())
            .into_iter()
            .map(|a| {
                let info = a.get_info();
                AdapterCandidate {
                    name: info.name,
                    backend: info.backend,
                    device_type: info.device_type,
                    vendor: info.vendor,
                    device: info.device,
                }
            })
            .collect();

        // Sort: discrete GPU > integrated > virtual > CPU (software).
        candidates.sort_by_key(|c| device_type_rank(c.device_type));
        candidates
    }

    /// Build a compute pipeline from WGSL source.
    pub(crate) fn build_compute_pipeline(
        &self,
        label: &str,
        wgsl: &str,
        entry_point: &str,
    ) -> wgpu::ComputePipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &shader,
                entry_point,
                compilation_options: Default::default(),
                cache: None,
            })
    }
}

/// Lower rank = higher selection preference.
fn device_type_rank(dt: wgpu::DeviceType) -> u8 {
    match dt {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Cpu => 3,
        wgpu::DeviceType::Other => 4,
    }
}
