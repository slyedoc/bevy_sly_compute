# Bevy Sly Compute

Plugin aims to provide easy way access data created or modified on the GPU, back in app world.  Current approach uses channel to send data back to app world.  Uses Events to trigger and notify compute is complete.

> Note: This is a work in progress, and an experment.  I got it working and plan on dog fooding it for a while.

Currently requires:

- [bevy fork](https://github.com/slyedoc/bevy/tree/bevy_compute)
for modified AsBindGroup macro
- [bevy-inspector-egui fork](https://github.com/slyedoc/bevy-inspector-egui) -  Also a few image formats and change detection

> See [Compute in App branch](https://github.com/slyedoc/bevy_sly_compute/tree/bevy_0.12) for first attempt at this. It duplicating render resources in app world like from [bevy-app-compute](https://github.com/Kjolnyr/bevy_app_compute) Kinda worked. For context see [PR #8440](https://github.com/bevyengine/bevy/issues/8440)

## Staging

See [AsBindGroup](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) for more information on how to use the attributes.

The only additional option is 'staging', which when used with ComputePlugin will arrange to have copied back to the app world after the compute shader is run.

- uniform - TODO (haven't found a need for this myself)
- storage - 'staging' - have resource updated after compute
  - ```Vec<T>``` works if ```T: Pod```, Color doesn't work
  - TODO: Look into encase ReadFrom and WriteTo
- storage_texture - 'staging' - image transfered after compute and ```Assets<Image>``` updated with AssetModified event
- buffer - TODO

## Issues

[Github issues](https://github.com/slyedoc/bevy_sly_compute/issues)

## Examples

See Examples:



- [basic](examples/basic.rs)-[wgsl](assets/basic.wgsl) - The Simplest use case
- [image](examples/image.rs)-[wgsl](assets/image.wgsl) - Compute image and use it in material, and save image it to file
- [terrain](examples/terrain.rs) - generates mesh and collider from image, brush to let you paint on it
- [paint](examples/paint.rs) - (doesn't use staging), lets you paint to different standard materials entities

- [many](examples/many.rs)-(uses basic) - Multiple ComputeWorkerPlugins

### TODO

- [ ] Data:
  - [ ] Uniform
  - [x] Storage
  - [x] StorageTexture
    - [X] Tested with R32Float, R8uint, Rgba8Unorm
    - [ ] add error to macro when and check for readwrite support depending on format [notes](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html)
  - [ ] Buffer  
- Examples:
  - [x] Basic
  - [x] Inspect
  - [x] Image  
  - [x] Many - Multiple ComputeWorkerPlugins
- [x] AssetEvent::Modified

  - [x] Egui Inspector - added patch to bevy-inspector-egui to clear resized images on asset modified events
  - [x] Any Material - See mark_shader_modified, StandardMaterial added by default
- [ ] Instancing
- [ ] Components - Big TODO
- [x] Multiple Entry Points
- [x] Multiple Passes
- [x] Many Plugin Instances

## References

- [WebGPU Storage Textures](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html)
- [bevy-app-compute](https://github.com/Kjolnyr/bevy_app_compute) - Closest thing to what I want, but no texture support, and magic strings.
- [wgpu hello-compute](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/hello-compute/main.rs) - Example of using compute shaders in wgpu.
- [wgpu capture](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/capture/main.rs)
- [AsBindGroup docs](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) - Documenation for AsBindGroup, which is duplicated here for now.
- [PR #8440](https://github.com/bevyengine/bevy/issues/8440) - Issue for adding support for compute shaders in bevy.

## Bevy support table

| bevy    | bevy_sly_compute |
| ------- | ------------------- |
| 0.13 *[forked](https://github.com/slyedoc/bevy/tree/bevy_compute) | 0.2                 |
