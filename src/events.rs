use std::marker::PhantomData;

use bevy::{prelude::*, render::render_resource::ShaderRef};

use crate::{ComputeShader, ComputeTrait};

/// Message to notify the App world that the compute has completed
#[derive(Event)]
pub struct ComputeComplete<T: ComputeTrait> {    
    pub dont_copy: bool,
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeComplete<T> {
    fn default() -> Self {
        ComputeComplete {
            dont_copy: false,
            _marker: Default::default(),
        }
    }
}

/// Event to trigger a compute shader, you can specify multiple passes and workgroups
#[derive(Event, Clone)]
pub struct ComputeEvent<T: ComputeTrait> {
    pub passes: Vec<Pass>,
    pub dont_copy: bool,
    pub _marker: PhantomData<T>,
    
}

// Helpers to create compute events
impl<T: ComputeTrait> Default for ComputeEvent<T> {
    fn default() -> Self {
        Self { 
            passes: vec![Pass {
                entry: T::entry_points().first().expect("no entry points"),
                workgroups: vec![UVec3::new(1, 1, 1)],
            }], 
            dont_copy: false,
            _marker: Default::default()
         }
    }
}

impl<T: ComputeTrait> ComputeEvent<T> {
    pub fn new(workgroups: UVec3) -> Self {        
        ComputeEvent::<T> {

            passes: vec![
                Pass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![workgroups],
                }
            ],
            ..default()
        }
    }

    pub fn new_named(name: &'static str, workgroups: UVec3) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                Pass {
                    entry: name,
                    workgroups: vec![workgroups],
                }
            ],
            ..default()
        }
    }


    pub fn new_xyz( x: u32, y: u32, z: u32) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                Pass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![UVec3::new(x, y, z)],
                }
            ],
            ..default()
        }
    }

    pub fn add_pass(&mut self, entry: &'static str, workgroup: UVec3) -> &mut Self {
        self.passes.push(Pass::new(entry, workgroup));
        self
    }

}

/// A pass to run a compute shader
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pass {
    /// entry point for pipeline, need pipeline per entry point
    pub entry: &'static str,

    /// workgroup sizes to run for entry point
    pub workgroups: Vec<UVec3>,
}

impl Pass {
    pub fn new(entry: &'static str, workgroups: UVec3) -> Self {
        Pass {
            entry,
            workgroups: vec![workgroups],
        }
    }
}


#[derive(Event)]
pub struct ComputeShaderModified<T: ComputeTrait> {    
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeShaderModified<T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

/// System to notify the compute shader has been modified
pub fn shader_modified<T: ComputeShader + ComputeTrait>(
    mut events: EventReader<AssetEvent<Shader>>,
    mut asset_id_option: Local<Option<AssetId<Shader>>>,
    mut notify_events: EventWriter<ComputeShaderModified<T>>,
    asset_server: Res<AssetServer>,
) {
    let asset_id = match *asset_id_option {        
        Some(id) => {
            let n  = id.clone();
            n
        }
        None => {
            let id = match T::shader() {
                ShaderRef::Handle(handle) => handle.id(),
                ShaderRef::Path(path) => asset_server.load(path).id(),
                _ => todo!(),
            };
            *asset_id_option = Some(id.clone());
            id
        },
    };
    
    if events.read().any(|e| match e {
        AssetEvent::Modified { id } => {
            if id == &asset_id {
                true
            } else {
                false
            }
        }
        _ => false,
    }) {
        info!("Shader just changed: {:?}", asset_id);
        notify_events.send(ComputeShaderModified::<T>::default());
    }

}