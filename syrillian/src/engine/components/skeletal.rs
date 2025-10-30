use crate::World;
use crate::components::{Component, MeshRenderer};
use crate::core::{Bones, GameObjectId};
use crate::utils::MATRIX4_ID;
use log::warn;
use nalgebra::{Matrix4, Scale3, Vector3};
use nalgebra::{Translation3, UnitQuaternion};

pub struct SkeletalComponent {
    parent: GameObjectId,
    bones_static: Bones,
    delta_local: Vec<Matrix4<f32>>,
    globals: Vec<Matrix4<f32>>,
    palette: Vec<Matrix4<f32>>,
    dirty: bool,
}

impl Component for SkeletalComponent {
    fn new(parent: GameObjectId) -> Self {
        Self {
            parent,
            bones_static: Bones::none(),
            delta_local: Vec::new(),
            globals: Vec::new(),
            palette: Vec::new(),
            dirty: true,
        }
    }

    fn init(&mut self, world: &mut World) {
        let Some(renderer) = self.parent.get_component::<MeshRenderer>() else {
            warn!("No Mesh Renderer found on Skeletal Object");
            return;
        };
        let Some(mesh) = world.assets.meshes.try_get(renderer.mesh()) else {
            warn!("No Mesh found for the Mesh linked in a Mesh Renderer");
            return;
        };

        let n = mesh.bones.len();
        self.bones_static = mesh.bones.clone();

        self.delta_local = vec![Matrix4::identity(); n];
        self.globals = vec![Matrix4::identity(); n];
        self.palette = vec![Matrix4::identity(); n];
        self.dirty = true;
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl SkeletalComponent {
    pub fn bone_count(&self) -> usize {
        self.bones_static.len()
    }

    /// Access bones metadata (names/parents/inv_bind)
    pub fn bones(&self) -> &Bones {
        &self.bones_static
    }

    /// Set local TRS for (some/all) bones.
    pub fn set_local_pose_trs(
        &mut self,
        locals: &[(Vector3<f32>, UnitQuaternion<f32>, Vector3<f32>)],
    ) {
        let n = self.bones_static.len();
        self.delta_local.resize(n, MATRIX4_ID);
        for (i, (pos, rot, scale)) in locals.iter().enumerate().take(n) {
            let m = Translation3::from(*pos).to_homogeneous()
                * rot.to_homogeneous()
                * Scale3::from(*scale).to_homogeneous();
            self.set_local_transform(i, m);
        }
        self.dirty = true;
    }

    /// Set a bone's local delta rotation (about its local origin)
    pub fn set_local_rotation(&mut self, index: usize, q: UnitQuaternion<f32>) {
        let mut rot = Matrix4::identity();
        rot.fixed_view_mut::<3, 3>(0, 0)
            .copy_from(q.to_rotation_matrix().matrix());
        self.delta_local[index] = rot;
        self.dirty = true;
    }

    pub fn set_local_transform(&mut self, index: usize, pos: Matrix4<f32>) {
        self.delta_local[index] = pos;
        self.dirty = true;
    }

    pub fn palette(&self) -> &[Matrix4<f32>] {
        &self.palette
    }

    pub fn update_palette(&mut self) -> bool {
        if !self.dirty {
            return false;
        }

        fn visit(
            i: usize,
            bones: &Bones,
            globals: &mut [Matrix4<f32>],
            delta_local: &[Matrix4<f32>],
            palette: &mut [Matrix4<f32>],
            parent_global: Matrix4<f32>,
        ) {
            let local_new = bones.bind_local[i] * delta_local[i];
            let g = parent_global * local_new;
            globals[i] = g;
            palette[i] = g * bones.inverse_bind[i];
            for &c in &bones.children[i] {
                visit(c, bones, globals, delta_local, palette, g);
            }
        }

        for &root in &self.bones_static.roots {
            visit(
                root,
                &self.bones_static,
                &mut self.globals,
                &self.delta_local,
                &mut self.palette,
                MATRIX4_ID,
            );
        }

        self.dirty = false;
        true
    }
}
