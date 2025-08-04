use crate::World;
use std::collections::VecDeque;

const DEFAULT_RUNNING_SIZE: usize = 60;

#[derive(Debug, Clone, Default)]
pub struct FrameCounter {
    frame_times: VecDeque<f32>,
}

impl FrameCounter {
    pub fn new_frame(&mut self, delta_time: f32) {
        if self.frame_times.len() >= DEFAULT_RUNNING_SIZE {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(delta_time);
    }

    pub fn new_frame_from_world(&mut self, world: &World) {
        let frame_time = world.delta_time().as_secs_f32();
        self.new_frame(frame_time);
    }

    pub fn mean_delta_time(&self) -> f32 {
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    pub fn fps(&self) -> u32 {
        (1.0 / self.mean_delta_time()) as u32
    }
}
