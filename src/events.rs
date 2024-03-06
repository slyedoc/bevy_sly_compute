use std::marker::PhantomData;

use bevy::prelude::*;

use crate::ComputeTrait;

/// Message to notify the App world that the compute has completed
#[derive(Event)]
pub struct ComputeComplete<T: ComputeTrait> {    
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeComplete<T> {
    fn default() -> Self {
        ComputeComplete {
            _marker: Default::default(),
        }
    }
}

/// Event to trigger a compute shader, you can specify multiple passes and workgroups
#[derive(Event, Clone)]
pub struct ComputeEvent<T: ComputeTrait> {
    pub passes: Vec<Pass>,
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
            _marker: Default::default(),
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
            _marker: Default::default(),
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
            _marker: Default::default(),
        }
    }

    pub fn add_pass(&mut self, entry: &'static str, workgroup: UVec3) -> &mut Self {
        self.passes.push(Pass::new(entry, workgroup));
        self
    }

}

/// A pass to run a compute shader
#[derive(Clone, Debug)]
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

