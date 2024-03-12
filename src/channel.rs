use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use crate::ComputeTrait;

/// Data to pass from Render World to App World
pub struct ComputeMessage<T: ComputeTrait> {
    pub data: Option<T>,
    pub images: Vec<(Handle<Image>, Vec<u8>)>,
}

/// Channel resource used to receive ComputeMessage from render world.
#[derive(Resource, Deref, DerefMut)]
pub struct ComputeReceiver<T: ComputeTrait> (pub Receiver<ComputeMessage<T>>);


/// Channel resource used to send time from the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct ComputeSender<T: ComputeTrait> ( pub Sender<ComputeMessage<T>>);

/// Creates channels used for sending time between the render world and the main world.
pub fn create_compute_channels<'a, T: ComputeTrait>() -> (ComputeSender<T>, ComputeReceiver<T>) {
    // bound the channel to 2 since when pipelined the render phase can finish before
    // the time system runs.
    let (s, r) = crossbeam_channel::bounded::<ComputeMessage<T>>(2);
    (ComputeSender(s), ComputeReceiver(r))
}