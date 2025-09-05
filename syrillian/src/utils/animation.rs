use crate::core::GameObjectId;
use nalgebra::{UnitQuaternion, Vector3};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Per-node keyframes.
/// Times are in **seconds**.
#[derive(Debug, Clone, Default)]
pub struct TransformKeys {
    pub t_times: Vec<f32>,
    pub t_values: Vec<Vector3<f32>>,

    pub r_times: Vec<f32>,
    pub r_values: Vec<UnitQuaternion<f32>>,

    pub s_times: Vec<f32>,
    pub s_values: Vec<Vector3<f32>>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub target_name: String,
    pub keys: TransformKeys,
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    /// Duration in seconds
    pub duration: f32,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Default, Clone)]
pub struct ClipIndex {
    pub by_name: HashMap<String, usize>,
}

impl ClipIndex {
    pub fn new(clip: &AnimationClip) -> Self {
        let mut by_name = HashMap::new();
        for (i, ch) in clip.channels.iter().enumerate() {
            by_name.insert(ch.target_name.clone(), i);
        }
        Self { by_name }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Binding {
    Transform(GameObjectId),
    Bone { skel: GameObjectId, idx: usize },
}

#[derive(Debug, Clone)]
pub struct ChannelBinding {
    /// Index into clip.channels
    pub ch_index: usize,
    pub target: Binding,
}

#[derive(Debug, Clone)]
pub struct Playback {
    pub clip_index: usize,
    pub time: f32,
    pub speed: f32,
    pub weight: f32,
    pub looping: bool,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            clip_index: 0,
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            looping: true,
        }
    }
}

fn find_key(times: &[f32], t: f32) -> usize {
    if times.is_empty() {
        return 0;
    }
    if t <= times[0] {
        return 0;
    }
    if t >= *times.last().unwrap_or_else(|| {
        log::error!("Failed to get times.last() in find_key in animation.rs");
        std::process::exit(1);
    }) {
        return times.len() - 1;
    }
    times
        .binary_search_by(|k| k.partial_cmp(&t).unwrap_or(Ordering::Equal))
        .unwrap_or_else(|i| (i - 1).max(0))
}

fn lerp_vec3(a: &Vector3<f32>, b: &Vector3<f32>, alpha: f32) -> Vector3<f32> {
    a * (1.0 - alpha) + b * alpha
}

pub fn sample_translation(keys: &TransformKeys, t: f32) -> Vector3<f32> {
    let n = keys.t_times.len();
    if n == 0 {
        return Vector3::zeros();
    }
    if n == 1 {
        return keys.t_values[0];
    }

    let i = find_key(&keys.t_times, t);
    if i == n - 1 {
        return keys.t_values[i];
    }
    let t0 = keys.t_times[i];
    let t1 = keys.t_times[i + 1];
    let a = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    lerp_vec3(&keys.t_values[i], &keys.t_values[i + 1], a)
}

pub fn sample_scale(keys: &TransformKeys, t: f32) -> Vector3<f32> {
    let n = keys.s_times.len();
    if n == 0 {
        return Vector3::new(1.0, 1.0, 1.0);
    }
    if n == 1 {
        return keys.s_values[0];
    }

    let i = find_key(&keys.s_times, t);
    if i == n - 1 {
        return keys.s_values[i];
    }
    let t0 = keys.s_times[i];
    let t1 = keys.s_times[i + 1];
    let a = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    lerp_vec3(&keys.s_values[i], &keys.s_values[i + 1], a)
}

pub fn sample_rotation(keys: &TransformKeys, t: f32) -> UnitQuaternion<f32> {
    let n = keys.r_times.len();
    if n == 0 {
        return UnitQuaternion::identity();
    }
    if n == 1 {
        return keys.r_values[0];
    }

    let i = find_key(&keys.r_times, t);
    if i == n - 1 {
        return keys.r_values[i];
    }
    let t0 = keys.r_times[i];
    let t1 = keys.r_times[i + 1];
    let a = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    keys.r_values[i].slerp(&keys.r_values[i + 1], a)
}
