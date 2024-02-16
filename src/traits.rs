use bevy::{
    prelude::*,
    render::{
        render_resource::{
            AsBindGroupError, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, PreparedBindGroup, ShaderDefVal, ShaderRef, UnpreparedBindGroup,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
    },
};
use wgpu::PushConstantRange;

use super::ComputeAssets;

// Define a new trait with all the combined requirements
pub trait ComputeTrait:
    AsBindGroupCompute + ComputeShader + Resource + Clone + Sized + Send + Sync + 'static
{
}

// Now implement MyTrait for any type that satisfies the individual requirements
impl<T> ComputeTrait for T where
    T: AsBindGroupCompute + ComputeShader + Resource + Clone + Sized + Send + Sync + 'static
{
}

pub trait AsBindGroupCompute {
    /// Data that will be stored alongside the "prepared" bind group.
    type Data: Send + Sync;

    /// label
    fn label() -> Option<&'static str> {
        None
    }

    /// Creates a bind group for `self` matching the layout defined in [`AsBindGroup::bind_group_layout`].
    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &ComputeAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        let UnpreparedBindGroup { bindings, data } =
            Self::unprepared_bind_group(self, layout, render_device, images, fallback_image)?;

        let entries = bindings
            .iter()
            .map(|(index, binding)| BindGroupEntry {
                binding: *index,
                resource: binding.get_binding(),
            })
            .collect::<Vec<_>>();

        let bind_group = render_device.create_bind_group(Self::label(), layout, &entries);

        Ok(PreparedBindGroup {
            bindings,
            bind_group,
            data,
        })
    }

    /// Returns a vec of (binding index, `OwnedBindingResource`).
    /// In cases where `OwnedBindingResource` is not available (as for bindless texture arrays currently),
    /// an implementor may define `as_bind_group` directly. This may prevent certain features
    /// from working correctly.
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &ComputeAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError>;

    /// Creates the bind group layout matching all bind groups returned by [`AsBindGroup::as_bind_group`]
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout
    where
        Self: Sized,
    {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Self::label(),
            entries: &Self::bind_group_layout_entries(render_device),
        })
    }

    /// Returns a vec of bind group layout entries
    fn bind_group_layout_entries(render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized;

    /// Creates cpu staging buffers, should be used to read back data from the gpu.
    fn create_staging_buffers(
        &self,
        render_device: &RenderDevice,
    ) -> Vec<(u32, bevy::render::render_resource::Buffer)>
    where
        Self: Sized;

    /// Maps the staging buffer slices to Self
    fn map_staging_mappings(
        &mut self,
        staging_buffers: &Vec<(u32, bevy::render::render_resource::BufferSlice<'_>)>,
    ) where
        Self: Sized;
}

pub trait ComputeShader: Send + Sync + 'static {
    /// Implement your [`ShaderRef`]
    ///
    /// Usually, it comes from a path:
    /// ```
    /// fn shader() -> ShaderRef {
    ///     "shaders/my_shader.wgsl".into()
    /// }
    /// ```
    fn shader() -> ShaderRef;

    /// TODO: Is this a use case?
    /// If you don't want to use wgpu's reflection for
    /// your binding layout, you can declare them here.
    // fn layouts<'a>() -> &'a [BindGroupLayout] {
    //     &[]
    // }

    fn shader_defs<'a>() -> &'a [ShaderDefVal] {
        &[]
    }
    fn push_constant_ranges<'a>() -> &'a [PushConstantRange] {
        &[]
    }

    /// By default, the shader entry point is `main`.
    /// You can change it from here.
    fn entry_point<'a>() -> &'a str {
        "main"
    }
}
