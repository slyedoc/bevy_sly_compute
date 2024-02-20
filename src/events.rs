use std::marker::PhantomData;

use bevy::prelude::*;

use crate::ComputeTrait;


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

// hack to retry when pipeline is not ready
#[derive(Event, Clone)]
pub struct RequeueComputeEvent<T: ComputeTrait> {
    pub passes: Vec<ComputePass>,
    pub retry: u32,
    pub _marker: PhantomData<T>,
}

#[derive(Event, Clone)]
pub struct ComputeEvent<T: ComputeTrait> {
    pub passes: Vec<ComputePass>,
    pub retry: u32,
    pub _marker: PhantomData<T>,
}


impl<T: ComputeTrait> Default for ComputeEvent<T> {
    fn default() -> Self {
        Self { 
            passes: vec![ComputePass {
                entry: T::entry_points().first().expect("no entry points"),
                workgroups: vec![UVec3::new(1, 1, 1)],
            }], 
            retry: 0,
            _marker: Default::default()
         }
    }
}

impl<T: ComputeTrait> ComputeEvent<T> {
    pub fn new(workgroups: UVec3) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                ComputePass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![workgroups],
                }
            ],
            retry: 0,
            _marker: Default::default(),
        }
    }

    pub fn new_xyz( x: u32, y: u32, z: u32) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                ComputePass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![UVec3::new(x, y, z)],
                }
            ],
            retry: 0,
            _marker: Default::default(),
        }
    }

    pub fn add_pass(&mut self, entry: &'static str, workgroup: UVec3) -> &mut Self {
        self.passes.push(ComputePass::new(entry, workgroup));
        self
    }

}

#[derive(Clone, Debug)]
pub struct ComputePass {
    /// entry point for pipeline, need pipeline per entry point
    pub entry: &'static str,

    /// workgroup sizes to run for entry point
    pub workgroups: Vec<UVec3>,
}

impl ComputePass {
    pub fn new(entry: &'static str, workgroups: UVec3) -> Self {
        ComputePass {
            entry,
            workgroups: vec![workgroups],
        }
    }
}

#[cfg(feature = "egui")]
#[derive(Event, Debug, Clone)]
pub struct ComputeUpdateEgui {
    pub handle: Handle<Image>,    
}
