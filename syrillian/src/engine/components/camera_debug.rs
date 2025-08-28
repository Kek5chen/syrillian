use crate::components::Component;
use crate::core::GameObjectId;
use crate::rendering::CPUDrawCtx;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::proxies::debug_proxy::{DebugLine, DebugSceneProxy};
use crate::{World, proxy_data_mut};
use itertools::Itertools;
use nalgebra::{Point3, Vector3, Vector4};
use rapier3d::geometry::Ray;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DebugRay {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
    pub toi: f32,
}

impl From<&DebugRay> for DebugLine {
    fn from(value: &DebugRay) -> Self {
        DebugLine {
            start: value.origin,
            end: value.origin + value.direction * value.toi,
            start_color: Vector4::new(0.9, 0.2, 0.2, 1.0),
            end_color: Vector4::new(0.4, 0.4, 0.2, 1.0),
        }
    }
}

pub struct CameraDebug {
    parent: GameObjectId,

    rays: Vec<DebugRay>,
    ray_times: Vec<Instant>,
    dirty: bool,

    pub lifetime: Duration,
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

impl Component for CameraDebug {
    fn new(parent: GameObjectId) -> Self
    where
        Self: Sized,
    {
        Self {
            parent,
            rays: vec![],
            ray_times: vec![],

            lifetime: Duration::from_secs(5),

            dirty: true,
        }
    }

    fn create_render_proxy(&mut self, _world: &World) -> Option<Box<dyn SceneProxy>> {
        let lines = self.rays.iter().map_into().collect();
        Some(Box::new(DebugSceneProxy {
            lines,
            meshes: vec![],
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
        }))
    }

    fn update_proxy(&mut self, _world: &World, mut ctx: CPUDrawCtx) {
        if self.rays.is_empty() {
            return;
        }

        self.timeout_rays();

        if !self.dirty {
            return;
        }

        let lines = self.rays.iter().map_into().collect();
        ctx.send_proxy_update(move |proxy| {
            let proxy: &mut DebugSceneProxy = proxy_data_mut!(proxy);

            proxy.lines = lines;
        })
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}
