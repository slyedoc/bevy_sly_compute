# Bevy Sly Compute **WIP**

Plugin aims to provide easy way access data created or modified on the GPU, back in app world.

Bevy has a lot of machinery to handle assets and bind group generation, including AsBindGroup, This is an attempt to extend that a bit farther.

Context - See [PR #8440](https://github.com/bevyengine/bevy/issues/8440)

 > TLDR; You can't get data out of render_app in bevy.  If there was I could easily move this to render_app and not have to duplicate images and shader cache.

## Examples

See Examples:

- [basic.rs](examples/basic.rs)-[wgsl](assets/basic.wgsl) - The Simplest use case
- [inspect.rs](examples/inspect.rs)-[wgsl](examples/inspect.wgsl) - Egui inspector with embedded compute shader, (my workflow)
- [image.rs](examples/image.rs)-[wgsl](assets/image.wgsl) - **WIP**

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

## Important

Require patches currently:

- Bevy
  - Make DefaultImageSampler pub
  - Change from [PR #9943](https://github.com/bevyengine/bevy/pull/9943) - add storage_texture option to as_bind_group macro
    - This will come in 0.13, but would affect this plugin.

- bevy-inspector-egui
  - add few image formats, I should put in a PR

- See [Cargo.toml](Cargo.toml) patch section.

## Macro

See [AsBindGroup](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) for more information on how to use the attributes.

Only differences so far are:

- uniform - _none_
- storage - 'staging' - have resource updated after compute
- texture - _none_ - **WIP**

Would love to take to someone that has more knowledge in what would be ideal for this.

## TODO

- [ ] Figure out ideal solution:
      - Render World - most ideal, but not possible far as I know currently.
      - Compute World - doable but can't share with render world.  Would still require a _ReverseExtract_.
      - App World - This is basically the "I will do it live, F*CK It" option.  This is what I am doing now.
  - [ ] Work on System Scheduals and labels once path above is figured out
- [ ] Data:
  - [ ] Uniform
  - [x] Storage
  - [-] StorageTexture - **WIP**
  - [ ] Buffer  
- [x] Multiple Entry Points
- [x] Multiple Passes
- [ ] Instancing
- [ ] Go over macro - Right now I have just been adding what I need as I need it
- [ ] Many Plugin Instances
- [ ] Many Entity Instances - Will look into this after Instancing

- [ ] Stop Duplicating ```RenderAssets<Image>``` - This duplicate texture memory usage, come back to this once compute world or render options are figured out
- [ ] Profile and check system scheduals
- [ ] Remove Bevy patches

## References

- [bevy-app-compute](https://github.com/Kjolnyr/bevy_app_compute) - Closest thing to what I want, but no texture support, and magic strings.
- [hello-compute](https://github.com/gfx-rs/wgpu-rs/blob/master/examples/hello-compute/main.rs) - Example of using compute shaders in wgpu.
- [AsBindGroup docs](https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html) - Documenation for AsBindGroup, which is duplicated here for now.
- [PR #8440](https://github.com/bevyengine/bevy/issues/8440) - Issue for adding support for compute shaders in bevy.

## Notes

- ```bevy_app_compute``` - Works, but no texture support, and magic strings.
- Compute world - like render_app but for compute shaders
  - Would require a _ReverseExtract_ (Is this ideal?)
  - Currently, duplicating ```RenderAssets<Image>``` and resources in app world, a hack and will double the memory usage of all textures.
    - Ideally we would only extract the textures we need if any.