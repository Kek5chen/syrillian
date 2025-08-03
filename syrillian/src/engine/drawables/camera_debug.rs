use crate::assets::HShader;
use crate::core::GameObjectId;
use crate::drawables::{DebugRuntimePatternData, Drawable};
use crate::rendering::{DrawCtx, Renderer};
use crate::{ensure_aligned, World};
use log::warn;
use nalgebra::{Matrix4, Point3, Vector3};
use rapier3d::geometry::Ray;
use std::time::{Duration, Instant};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, Device};

#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
#[cfg(debug_assertions)]
pub struct DebugRay {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
    pub toi: f32,
}

ensure_aligned!(DebugRay { origin, direction }, align <= 4 * 7 => size);

#[cfg(debug_assertions)]
pub struct CameraDebug {
    rays: Vec<DebugRay>,
    ray_times: Vec<Instant>,
    dirty: bool,

    pub lifetime: Duration,

    data: Option<DebugRuntimePatternData>,
}

impl CameraDebug {
    pub fn push_ray(&mut self, ray: Ray, max_toi: f32) {
        self.rays.push(DebugRay {
            origin: ray.origin,
            direction: ray.dir,
            toi: max_toi,
        });
        self.ray_times.push(Instant::now());
        self.dirty = true;
    }

    pub fn clear_rays(&mut self) {
        self.rays.clear();
        self.ray_times.clear();
        self.dirty = true;
    }

    fn new_ray_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Debug Ray Data Buffer"),
            contents: bytemuck::cast_slice(&self.rays[..]),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        })
    }

    pub fn timeout_rays(&mut self) {
        let mut i = 0;
        while i < self.rays.len() {
            if let Some(time) = self.ray_times.get(i)
                && time.elapsed() < self.lifetime
            {
                i += 1;
                continue;
            }

            self.rays.remove(i);
            self.ray_times.remove(i);
            self.dirty = true;
        }
    }
}

impl Default for CameraDebug {
    fn default() -> Self {
        Self {
            rays: vec![],
            ray_times: vec![],

            lifetime: Duration::from_secs(5),

            dirty: true,
            data: None,
        }
    }
}

impl Drawable for CameraDebug {
    fn setup(&mut self, renderer: &Renderer, _world: &mut World) {
        let device = renderer.state.device.as_ref();

        let vertices_buf = self.new_ray_buffer(device);

        let debug_data = DebugRuntimePatternData {
            vertices_buf,
        };

        self.data = Some(debug_data);
        self.dirty = false;
    }

    fn update(
        &mut self,
        world: &mut World,
        _parent: GameObjectId,
        renderer: &Renderer,
        _outer_transform: &Matrix4<f32>,
    ) {
        if self.data.is_none() {
            self.setup(renderer, world);
            return; // everything should be fresh now, anyway
        };

        self.timeout_rays();

        if !self.dirty {
            return;
        }

        let device = renderer.state.device.as_ref();
        let vertices_buf = self.new_ray_buffer(device);

        self.data
            .as_mut()
            .expect("Data was checked for None")
            .vertices_buf = vertices_buf;
        self.dirty = false;
    }

    fn draw(&self, _world: &mut World, ctx: &DrawCtx) {
        if self.rays.is_empty() || !ctx.frame.debug.rays {
            return;
        }

        let Some(data) = &self.data else {
            warn!("Tried to draw camera debug without setup");
            return;
        };

        if self.dirty {
            warn!("Ray Debugger was dirty when drawing, this should not happen");
            return;
        }

        let mut pass = ctx.pass.write().unwrap();

        pass.set_vertex_buffer(0, data.vertices_buf.slice(..));

        let shader = ctx.frame.cache.shader(HShader::DEBUG_RAYS);
        pass.set_pipeline(&shader.pipeline);

        pass.draw(0..2, 0..self.rays.len() as u32);
    }
}
