# Bevy Sly Compute

Plugin aims to provide easy way access data created or modified on the GPU, back in app world.

> This is a hack, but will work for my needs.  From this I do think I understand the problem better.  Now repo is mainly me exploring bevy, rust macros and wgpu.

An ideal solution would most likely be a PR to bevy:

  - No Compute World, stay in render world, worthless without long running compute tasks, and not possable in wgpu currently (find Issue for this) and no texture duplication issues.
  - Extend AsBindGroup to create staging buffers (similar to this repo)
  - In cleanup, map staging buffers back to render world resource, then send an event or just storing in a resource (resources are retained in render world)
  - _ReverseExtract_ Phase or Events to move data back to app world.
  
### Issue

You can't get data out of render_app as far as I know, this bypasses that by using the app world by duplicating things. 
  - ```PiplelineCache```
  - ```RenderAssets<Image>```

See [PR #8440](https://github.com/bevyengine/bevy/issues/8440)

### TODO

- [ ] 0.13
- [ ] Data:
  - [ ] Uniform
  - [x] Storage
  - [ ] Storage Texture - **WIP**
  - [ ] Buffer  
- Examples:
  - [x] Basic
  - [ ] Inspect **WIP**
  - [ ] Image **WIP**
  - [ ] [AsBindGroupShaderType] (https:///github.com/bevyengine/bevy/crates/bevy_sprite/src/mesh2d/color_material.rs)
  - [x] Many - Multiple ComputeWorkerPlugins
  [AsBindGroupShaderType]
- [ ] Instancing
  - How about a pass per entity?
- [ ] Go over macro - Right now I have just been adding what I need as I need it
- [ ] Use OwnedBindingResource
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
- storage_texture - 'staging' - image transfer after compute and ```Assets<Image>``` updated
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

- See [Cargo.toml](Cargo.toml) patch section.

## Examples

See Examples:

- [basic](examples/basic.rs)-[wgsl](assets/basic.wgsl) - The Simplest use case
- [inspect](examples/inspect.rs)-[wgsl](examples/inspect.wgsl) - Egui inspector with embedded compute shader, (my workflow)
- [image](examples/image.rs)-[wgsl](assets/image.wgsl) - **WIP**
- [many](examples/many.rs)-(uses basic) - Multiple ComputeWorkerPlugins

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

Use change tracking to see when to see new values (Should this be event as well?)

```rust
fn log_change( simple: Res<Simple> ) {
    if simple.is_changed() {        
        dbg!(&simple);
    }    
}
```

## References

- [bevy-app-compute](https://github.com/Kjolnyr/bevy_app_compute) - Closest thing to what I want, but no texture support, and magic strings.
- [hello-compute](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/hello-compute/main.rs) - Example of using compute shaders in wgpu.
- [AsBindGroup docs](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) - Documenation for AsBindGroup, which is duplicated here for now.
- [PR #8440](https://github.com/bevyengine/bevy/issues/8440) - Issue for adding support for compute shaders in bevy.