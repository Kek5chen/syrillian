use crate::components::Component;
use crate::core::{GameObjectId, Transform};
use crate::utils::FloatMathExt;
use crate::{ensure_aligned, World};
use nalgebra::{Matrix4, Perspective3, Point3, Vector2, Vector3, Vector4};
use rapier3d::geometry::Ray;

pub struct CameraComponent {
    pub projection: Perspective3<f32>,
    pub projection_inverse: Matrix4<f32>,
    fov_active: f32,
    fov_target: f32,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    parent: GameObjectId,
    pub zoom_speed: f32,
}

impl CameraComponent {
    /// Returns the fov in degrees
    pub fn fov(&self) -> f32 {
        self.fov_active
    }

    /// Sets the fov in degrees. Only use this for camera switches/init etc.
    /// Prefer set_fov_target for a smooth zoom.
    pub fn set_fov_instant(&mut self, fov: f32) {
        self.fov_active = fov;
        self.regenerate();
    }

    /// Smoothly zoom to the fov using the speed specified with set_zoom_speed
    pub fn set_fov_target(&mut self, fov: f32) {
        self.fov_target = fov;
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn set_near(&mut self, near: f32) {
        self.near = near;
        self.regenerate();
    }

    #[inline]
    pub fn far(&self) -> f32 {
        self.far
    }

    pub fn set_far(&mut self, far: f32) {
        self.far = far;
        self.regenerate();
    }

    #[inline]
    pub fn resolution(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    #[inline]
    pub fn mouse_viewport_position(&self, x: f32, y: f32) -> Vector2<f32> {
        let x = x / self.width;
        let y = y / self.height;
        Vector2::new(x.max(0.), y.max(0.))
    }

    #[inline]
    pub fn mouse_viewport_ndc_position(&self, x: f32, y: f32) -> Vector2<f32> {
        let mut viewport = self.mouse_viewport_position(x, y);
        viewport.x = (viewport.x * 2.0 - 1.0).clamp(-1.0, 1.0);
        viewport.y = (viewport.y * 2.0 - 1.0).clamp(-1.0, 1.0);
        viewport
    }

    #[inline]
    pub fn mouse_eye_clip_dir(&self, x: f32, y: f32) -> Vector4<f32> {
        let ndc = self.mouse_viewport_ndc_position(x, y);
        Vector4::new(ndc.x, ndc.y, -1.0, 0.0)
    }

    pub fn click_ray(&self, x: f32, y: f32) -> Ray {
        let eye = self.mouse_eye_clip_dir(x, y);
        let view = self
            .parent()
            .transform
            .get_global_transform_matrix()
            .to_homogeneous();

        let world = view * eye;
        let dir = world.xyz().normalize();

        Ray::new(view.transform_point(&Point3::new(0., 0., 0.)), dir)
    }

    pub fn regenerate(&mut self) {
        self.projection = Perspective3::new(
            self.width / self.height,
            self.fov_active.to_radians(),
            self.near,
            self.far,
        );
        self.projection_inverse = self.projection.inverse();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.regenerate();
    }

    #[cfg(debug_assertions)]
    pub fn push_debug_ray(&mut self, ray: Ray, max_toi: f32) {
        use crate::drawables::CameraDebug;

        let mut parent = self.parent();
        let Some(debug) = parent.drawable_mut::<CameraDebug>() else {
            log::warn!("No camera debug drawable found!");
            return;
        };

        debug.push_ray(ray, max_toi);
    }

}

impl Component for CameraComponent {
    fn new(parent: GameObjectId) -> Self {
        let projection = Perspective3::new(800.0 / 600.0, 60f32.to_radians(), 0.01, 1000.0);
        let projection_inverse = projection.inverse();

        add_debug_drawable(parent);

        CameraComponent {
            projection,
            projection_inverse,
            fov_active: 60.0,
            fov_target: 0.0,
            zoom_speed: 10.0,
            near: 0.01,
            far: 1000.0,
            width: 800.0,
            height: 600.0,
            parent,
        }
    }

    fn init(&mut self) {
        self.parent().transform.set_compound_pos_first(true);
    }

    fn update(&mut self) {
        let delta_time = World::instance().delta_time().as_secs_f32();

        if self.fov_target != 0.0 && (self.fov_active - self.fov_target).abs() > f32::EPSILON {
            self.fov_active = self.fov_active.lerp(self.fov_target, self.zoom_speed * delta_time);
            self.regenerate();
        }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

fn add_debug_drawable(_parent: GameObjectId) {
    #[cfg(debug_assertions)]
    {
        use crate::drawables::CameraDebug;

        let mut parent = _parent;
        parent.set_drawable(Box::new(CameraDebug::default()));
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pos: Vector3<f32>,
    _padding: u32,
    view_mat: Matrix4<f32>,
    projection_mat: Matrix4<f32>,
    pub proj_view_mat: Matrix4<f32>,
}

ensure_aligned!(CameraUniform { pos, view_mat, projection_mat, proj_view_mat }, align <= 16 * 13 => size);

impl CameraUniform {
    pub fn empty() -> Self {
        CameraUniform {
            pos: Vector3::zeros(),
            _padding: 0,
            view_mat: Matrix4::identity(),
            projection_mat: Matrix4::identity(),
            proj_view_mat: Matrix4::identity(),
        }
    }

    pub fn update(&mut self, proj_matrix: &Perspective3<f32>, cam_transform: &Transform) {
        self.pos = cam_transform.position();
        self.view_mat = cam_transform
            .get_global_transform_matrix_ext(true)
            .inverse()
            .to_homogeneous();
        self.projection_mat = proj_matrix.to_homogeneous();
        self.proj_view_mat = self.projection_mat * self.view_mat;
    }
}
