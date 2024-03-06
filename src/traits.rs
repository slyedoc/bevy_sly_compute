use bevy::{
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::{AsBindGroup, PushConstantRange, ShaderDefVal, ShaderRef}},
};

// Define a new trait with all the combined requirements
// TODO: Remove Debug after testing
pub trait ComputeTrait:
    AsBindGroup + ExtractResource + ComputeShader + Resource + core::fmt::Debug + Clone + Sized + Send + Sync + 'static
{
}

// Now implement MyTrait for any type that satisfies the individual requirements
impl<T> ComputeTrait for T where
    T: AsBindGroup + ExtractResource + ComputeShader + Resource + core::fmt::Debug + Clone + Sized + Send + Sync + 'static
{
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
    /// if you have multiple entry points, you can return them all.
    fn entry_points<'a>() -> Vec<&'a str> {
        vec!["main"]
    }
}
