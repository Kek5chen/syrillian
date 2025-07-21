use crate::components::{
    Collider3D, Component, PointLightComponent, RigidBodyComponent, RopeComponent, RotateComponent,
};
use crate::core::{GameObject, GameObjectId};
use rapier3d::dynamics::RigidBody;
use rapier3d::prelude::Collider;
use std::ops::{Deref, DerefMut};

pub trait GameObjectExt {
    fn at(&mut self, x: f32, y: f32, z: f32) -> &mut Self;
    fn scale(&mut self, scale: f32) -> &mut Self;
    fn non_uniform_scale(&mut self, x: f32, y: f32, z: f32) -> &mut Self;
}

pub trait GOComponentExt<'a>: Component {
    type Outer: Deref<Target=GameObject> + DerefMut;

    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer;
    fn finish(outer: &'a mut Self::Outer) -> &'a mut GameObject {
        outer.deref_mut()
    }
}

pub struct GOColliderExt<'a>(&'a mut Collider, &'a mut GameObject);
pub struct GORigidBodyExt<'a>(&'a mut RigidBody, &'a mut GameObject);
pub struct GOLightExt<'a>(&'a mut PointLightComponent, &'a mut GameObject);
pub struct GORotateExt<'a>(&'a mut RotateComponent, &'a mut GameObject);
pub struct GORopeExt<'a>(&'a mut RopeComponent, &'a mut GameObject);

impl GameObjectExt for GameObject {
    #[inline]
    fn at(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.transform.set_position(x, y, z);
        self
    }

    #[inline]
    fn scale(&mut self, scale: f32) -> &mut Self {
        self.transform.set_scale(scale);
        self
    }

    #[inline]
    fn non_uniform_scale(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.transform.set_nonuniform_scale(x, y, z);
        self
    }
}

impl GameObject {
    #[inline]
    pub fn build_component<'a, C: GOComponentExt<'a>>(&'a mut self) -> C::Outer {
        let component = self.add_component::<C>();
        C::build_component(component, self)
    }
}

impl<'a> GOComponentExt<'a> for Collider3D {
    type Outer = GOColliderExt<'a>;

    #[inline]
    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer {
        let collider = self.get_collider_mut().expect("Collider should be created");
        GOColliderExt(collider, obj)
    }

    #[inline]
    fn finish(outer: &'a mut Self::Outer) -> &'a mut GameObject {
        outer.1
    }
}

impl GOColliderExt<'_> {
    #[inline]
    pub fn mass(self, mass: f32) -> Self {
        self.0.set_mass(mass);
        self
    }

    #[inline]
    pub fn restitution(self, restitution: f32) -> Self {
        self.0.set_restitution(restitution);
        self
    }
}

impl Deref for GOColliderExt<'_> {
    type Target = GameObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl DerefMut for GOColliderExt<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}

impl<'a> GOComponentExt<'a> for RigidBodyComponent {
    type Outer = GORigidBodyExt<'a>;

    #[inline]
    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer {
        let rb = self.get_body_mut().expect("Rigid Body should be created");
        GORigidBodyExt(rb, obj)
    }

    #[inline]
    fn finish(outer: &'a mut Self::Outer) -> &'a mut GameObject {
        outer.1
    }
}

impl GORigidBodyExt<'_> {
    /// Enables continuous collision detection on this rigid body.
    /// Use this if it's bugging through walls, expected to move at fast speeds or
    /// expected to collide with high mass or high speed bodies.
    ///
    /// This makes the physics simulation more stable at the cost of performance
    ///
    /// This is disabled by default, this builder only provides an enable method.
    /// Please use RigidBodyComponent::get_body_mut for more settings
    #[inline]
    pub fn enable_ccd(self) -> Self {
        self.0.enable_ccd(true);
        self
    }

    #[inline]
    pub fn gravity_scale(self, scale: f32) -> Self {
        self.0.set_gravity_scale(scale, true);
        self
    }

    #[inline]
    pub fn angular_damping(self, damping: f32) -> Self {
        self.0.set_angular_damping(damping);
        self
    }

    #[inline]
    pub fn linear_damping(self, damping: f32) -> Self {
        self.0.set_linear_damping(damping);
        self
    }
}

impl Deref for GORigidBodyExt<'_> {
    type Target = GameObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl DerefMut for GORigidBodyExt<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}

impl<'a> GOComponentExt<'a> for PointLightComponent {
    type Outer = GOLightExt<'a>;

    #[inline]
    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer {
        GOLightExt(self, obj)
    }
}

impl GOLightExt<'_> {
    #[inline]
    pub fn color(self, r: f32, g: f32, b: f32) -> Self {
        self.0.set_color_rgb(r, g, b);
        self
    }

    #[inline]
    pub fn brightness(self, amount: f32) -> Self {
        self.0.set_intensity(amount);
        self
    }
}

impl Deref for GOLightExt<'_> {
    type Target = GameObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl DerefMut for GOLightExt<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}

impl<'a> GOComponentExt<'a> for RotateComponent {
    type Outer = GORotateExt<'a>;

    #[inline]
    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer {
        GORotateExt(self, obj)
    }
}

impl GORotateExt<'_> {
    #[inline]
    pub fn speed(&mut self, speed: f32) -> &mut Self {
        self.0.rotate_speed = speed;
        self
    }

    #[inline]
    pub fn scaling(&mut self, scaling: f32) -> &mut Self {
        self.0.scale_coefficient = scaling;
        self
    }
}

impl Deref for GORotateExt<'_> {
    type Target = GameObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl DerefMut for GORotateExt<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}

impl<'a> GOComponentExt<'a> for RopeComponent {
    type Outer = GORopeExt<'a>;

    #[inline]
    fn build_component(&'a mut self, obj: &'a mut GameObject) -> Self::Outer {
        GORopeExt(self, obj)
    }
}

impl GORopeExt<'_> {
    #[inline]
    pub fn connect_to(&mut self, other: GameObjectId) -> &mut Self {
        self.0.connect_to(other);
        self
    }
}

impl Deref for GORopeExt<'_> {
    type Target = GameObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.1
    }
}

impl DerefMut for GORopeExt<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1
    }
}
