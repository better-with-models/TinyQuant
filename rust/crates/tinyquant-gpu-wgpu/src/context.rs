//! `WgpuContext`: wgpu instance → adapter → device + queue setup.

use crate::error::TinyQuantGpuError;

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
    /// Initialise wgpu. Returns `Err(NoAdapter)` if no adapter is available.
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or(TinyQuantGpuError::NoAdapter)?;

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(TinyQuantGpuError::DeviceRequest)?;

        Ok(Self { device, queue, adapter_info })
    }

    /// Build a compute pipeline from WGSL source.
    ///
    /// Does not require an adapter for compilation validation —
    /// use `device.create_shader_module` directly for that.
    pub(crate) fn build_compute_pipeline(
        &self,
        label: &str,
        wgsl: &str,
        entry_point: &str,
    ) -> wgpu::ComputePipeline {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: None,
            module: &shader,
            entry_point,
            compilation_options: Default::default(),
            cache: None,
        })
    }
}
