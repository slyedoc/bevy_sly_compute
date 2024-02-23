# Bevy Sly Compute

Plugin aims to provide easy way access data created or modified on the GPU, back in app world.

> **Experiment**: This is a collection of compromises to just get this working.  

An ideal solution would:

- No Compute World, and don't duplicate render assets like I do here.  Stay in render world
  - Main reason to separate would be async long-running tasks, and while nice, not needed for most use cases (make long-running v2).
  - Leaves render app polling alone.
- Extend AsBindGroup to create staging buffers (similar to this repo)
- **BIG** In cleanup, or new stage, copes data back to app world from render world
  - This could be a _ReverseExtract_ Phase or Event queue to move data back to app world.

See [PR #8440](https://github.com/bevyengine/bevy/issues/8440)



### TODO

- [ ] 0.13 - waiting on bevy-inspector-egui
- [ ] Data:
  - [ ] Uniform
  - [x] Storage
  - [x] StorageTexture
    - [ ] Tested with R32Float, Rgba8Unorm
    - [ ] add error to macro when and check for readwrite support depending on format [notes](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html)
  - [ ] Buffer  
- Examples:
  - [x] Basic
  - [x] Inspect
  - [x] Image  
  - [ ] [AsBindGroupShaderType](https:///github.com/bevyengine/bevy/crates/bevy_sprite/src/mesh2d/color_material.rs)
  - [x] Many - Multiple ComputeWorkerPlugins
- [x] AssetEvent::Modified
  - [x] Egui Inspector - added patch to bevy-inspector-egui to clear resized images on asset modified events
  - [x] StandardMaterial - See mark_shader_modified, workaround to mark all materials as modified, not ideal, you can also use mark_shader_modified on any materials you want to be updated on compute image changes  
  [AsBindGroupShaderType]
- [ ] Instancing
  - How about a pass per entity?
- [ ] Go over macro - Right now I have just been adding what I need as I need it
- [x] Multiple Entry Points
- [x] Multiple Passes
- [x] Many Plugin Instances
- [ ] Stop Duplicating ```RenderAssets<Image>``` - This duplicate texture memory usage, come back to this once compute world or render options are figured out
- [ ] Profile and check system ordering
- [ ] Remove Bevy patches

## Macro

See [AsBindGroup](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) for more information on how to use the attributes.

Only differences so far are:

- uniform - TODO
- storage - 'staging' - have resource updated after compute
  - ```Vec<T>``` works if ```T: Pod```, Color doesn't work
  - TODO: Look into encase ReadFrom and WriteTo
- storage_texture - 'staging' - image transfered after compute and ```Assets<Image>``` updated with AssetModified event
- buffer - TODO

> Would love to take to someone that has more knowledge in what would be ideal for this.

## Important

Require patches currently:

- Bevy
  - Make DefaultImageSampler pub
  - Change from [PR #9943](https://github.com/bevyengine/bevy/pull/9943) - add storage_texture option to as_bind_group macro
    - This will come in 0.13, but would affect this plugin.
- bevy-inspector-egui
  - add few image formats, I should put in a PR
  - added asset_image_modified system to clear resized images on asset modified events
- See [Cargo.toml](Cargo.toml) patch section.

## Examples

See Examples:

- [basic](examples/basic.rs)-[wgsl](assets/basic.wgsl) - The Simplest use case
- [image](examples/image.rs)-[wgsl](assets/image.wgsl) - Compute image and use it in material
- [many](examples/many.rs)-(uses basic) - Multiple ComputeWorkerPlugins
- [terrain](examples/terrain.rs)-[wgsl](examples/terrain.wgsl) - Egui inspector with embedded compute shader, generates mesh from image and collider

## Usage

Create resource with AsBindGroupCompute, and ComputeShader.

```rust
#[derive(AsBindGroupCompute, Resource, Debug, Clone)]
pub struct Simple {
    #[uniform(0)]
    uni: f32,

    #[storage(1, staging)] // <-- 'staging' will use buffers to copy data back
    vec: Vec<f32>,
}

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {
        "basic.wgsl".into() // Asset path to the shader 
    }
}
```

Add ```ComputeWorkerPlugin<T>``` and your resource.

```rust
    .add_plugins((
        DefaultPlugins,
        ComputeWorkerPlugin::<Simple>::default(), // Add the Worker Plugin
    ))
    .insert_resource(Simple {
        uni: 1.0,
        vec: vec![1.0, 2.0, 3.0, 4.0],
    })
```

Use can events to execute compute shader.

```rust
  mut compute_events: EventWriter<ComputeEvent<Simple>>,
  ...
  compute_events.send(ComputeEvent::<Simple>::new_xyz(simple.vec.len() as u32, 1, 1));
```

 Multiple entry points and multiple passes are supported.  See [inspect.rs](examples/inspect.rs) example.

```rust
  let count = world.resource::<Simple>().vec.len() as u32;
  world.send_event(ComputeEvent::<Simple> {
      // first pass depends on the vec length                        
      passes: vec![
        ComputePass {
          entry:"pre", 
          workgroups: vec![
              UVec3::new(count, 1, 1),
              UVec3::new(1, 1, 1) // running second pass updating only first position 
          ],                            
        },
        // second pass depends on the image size
        ComputePass {
            entry:"main", 
            workgroups: vec![WORKGROUP],                            
        }
      ],
      ..default()
  });
```

Use events to see data is changed.

```rust
.add_systems(Last, compute_complete.run_if(on_event::<ComputeComplete<Simple>>()))
...
fn compute_complete( simple: Res<Simple> ) {
    dbg!(&simple);
}
```

## References

- [WebGPU Storage Textures](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html)
- [bevy-app-compute](https://github.com/Kjolnyr/bevy_app_compute) - Closest thing to what I want, but no texture support, and magic strings.
- [wgpu hello-compute](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/hello-compute/main.rs) - Example of using compute shaders in wgpu.
- [wgpu capture](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/capture/main.rs)
- [AsBindGroup docs](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) - Documenation for AsBindGroup, which is duplicated here for now.
- [PR #8440](https://github.com/bevyengine/bevy/issues/8440) - Issue for adding support for compute shaders in bevy.
