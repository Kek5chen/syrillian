use crate::core::GameObjectId;
use nalgebra::{Affine3, Scale3, Translation3, UnitQuaternion, Vector3};

/// Stores the translation, rotation and scale of a [`GameObject`](crate::core::GameObject).
///
/// The transform keeps precomputed matrices for each component so that
/// operations such as retrieving the final model matrix are fast.
#[repr(C)]
pub struct Transform {
    pos: Vector3<f32>,
    rot: UnitQuaternion<f32>,
    scale: Vector3<f32>,
    pos_mat: Translation3<f32>,
    scale_mat: Scale3<f32>,
    compound_mat: Affine3<f32>,
    invert_position: bool,
    owner: GameObjectId,
    compound_pos_first: bool,
}

impl Clone for Transform {
    fn clone(&self) -> Self {
        Transform {
            pos: self.pos,
            rot: self.rot,
            scale: self.scale,
            pos_mat: self.pos_mat,
            scale_mat: self.scale_mat,
            compound_mat: self.compound_mat,
            invert_position: self.invert_position,
            owner: GameObjectId::invalid(),
            compound_pos_first: self.compound_pos_first,
        }
    }
}

#[allow(dead_code)]
impl Transform {
    /// Creates a new [`Transform`] owned by the given [`GameObjectId`].
    ///
    /// The transform starts at the origin with no rotation and a uniform scale
    /// of `1.0`.
    pub fn new(owner: GameObjectId) -> Self {
        Transform {
            pos: Vector3::zeros(),
            rot: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            pos_mat: Translation3::identity(),
            scale_mat: Scale3::identity(),
            compound_mat: Affine3::identity(),
            invert_position: false,
            owner,
            compound_pos_first: true,
        }
    }

    /// Sets the global position of the transform.
    #[inline(always)]
    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.set_position_vec(Vector3::new(x, y, z))
    }

    /// Sets the global position using a vector.
    pub fn set_position_vec(&mut self, pos: Vector3<f32>) {
        let mat = self.get_global_transform_matrix_ext(false);
        self.set_local_position_vec(mat.inverse_transform_vector(&pos));
    }

    /// Returns the global position of the transform.
    pub fn position(&self) -> Vector3<f32> {
        let mat = self.get_global_transform_matrix().to_homogeneous();
        Vector3::new(mat.m14, mat.m24, mat.m34)
    }

    /// Collects the list of parents up to the root.
    fn get_parent_list(&self) -> Vec<GameObjectId> {
        let mut parents = vec![];
        let mut parent_opt = Some(self.owner);

        while let Some(parent) = parent_opt {
            parents.push(parent);
            parent_opt = parent.parent;
        }
        parents.reverse();

        parents
    }

    pub fn get_global_transform_matrix_ext(&self, include_self: bool) -> Affine3<f32> {
        let mut mat = Affine3::identity();
        let mut parents = self.get_parent_list();

        if !include_self {
            parents.pop();
        }

        for parent in parents {
            mat *= parent.transform.compound_mat;
        }
        mat
    }

    /// Returns the global model matrix for this transform.
    pub fn get_global_transform_matrix(&self) -> Affine3<f32> {
        self.get_global_transform_matrix_ext(true)
    }

    /// Calculates the global rotation, optionally excluding this transform.
    pub fn get_global_rotation_ext(&self, include_self: bool) -> UnitQuaternion<f32> {
        let mut global_rotation = UnitQuaternion::identity();
        let mut parents = self.get_parent_list();

        if !include_self {
            parents.pop();
        }

        for parent in parents {
            global_rotation *= parent.transform.rot;
        }
        global_rotation
    }

    /// Calculates the global scale matrix, optionally excluding this transform.
    pub fn get_global_scale_matrix_ext(&self, include_self: bool) -> Scale3<f32> {
        let mut mat = Scale3::identity();
        let mut parents = self.get_parent_list();

        if !include_self {
            parents.pop();
        }

        for parent in parents {
            mat *= parent.transform.scale_mat;
        }
        mat
    }

    /// Returns the global scale matrix for this transform.
    pub fn get_global_scale_matrix(&self) -> Scale3<f32> {
        self.get_global_scale_matrix_ext(true)
    }

    /// Sets the local position of the transform.
    #[inline]
    pub fn set_local_position(&mut self, x: f32, y: f32, z: f32) {
        let position = Vector3::new(x, y, z);
        self.set_local_position_vec(position);
    }

    /// Sets the local position using a vector.
    pub fn set_local_position_vec(&mut self, position: Vector3<f32>) {
        self.pos = position;
        self.recalculate_pos_matrix();
    }

    /// Returns a reference to the local position vector.
    pub fn local_position(&self) -> &Vector3<f32> {
        &self.pos
    }

    /// Inverts the sign of the position when true.
    pub fn set_invert_position(&mut self, invert: bool) {
        self.invert_position = invert;
    }

    /// Adds the given offset to the local position.
    pub fn translate(&mut self, other: Vector3<f32>) {
        self.pos += other;
        self.recalculate_pos_matrix();
    }

    /// Sets the local model-space rotation of this transform
    pub fn set_local_rotation(&mut self, rotation: UnitQuaternion<f32>) {
        self.rot = rotation;
        self.recalculate_combined_matrix()
    }

    /// Returns a reference to the local rotation quaternion.
    pub fn local_rotation(&self) -> &UnitQuaternion<f32> {
        &self.rot
    }

    /// Sets the global rotation of the transform.
    pub fn set_rotation(&mut self, rotation: UnitQuaternion<f32>) {
        let parent_global_rotation = self.get_global_rotation_ext(false);
        let local_rotation_change = parent_global_rotation.rotation_to(&rotation);

        self.set_local_rotation(local_rotation_change);
    }

    /// Returns the global rotation quaternion.
    pub fn rotation(&self) -> UnitQuaternion<f32> {
        self.get_global_rotation_ext(true)
    }

    /// Returns the global rotation euler angles
    pub fn euler_rotation(&self) -> Vector3<f32> {
        let (x, y, z) = self.get_global_rotation_ext(true).euler_angles();
        Vector3::new(x, y, z)
    }

    /// Applies a relative rotation to the transform.
    pub fn rotate(&mut self, rot: UnitQuaternion<f32>) {
        self.rot *= rot;
        self.recalculate_combined_matrix();
    }

    /// Sets the local scale using three independent factors.
    pub fn set_nonuniform_local_scale(&mut self, scale: Vector3<f32>) {
        self.scale = scale;
        self.recalculate_scale_matrix();
    }

    /// Sets the local scale uniformly.
    pub fn set_uniform_local_scale(&mut self, factor: f32) {
        self.set_nonuniform_local_scale(Vector3::new(factor, factor, factor));
    }

    /// Returns a reference to the local scale vector.
    pub fn local_scale(&self) -> &Vector3<f32> {
        &self.scale
    }

    /// Sets the global scale, preserving the current global orientation.
    pub fn set_nonuniform_scale(&mut self, x: f32, y: f32, z: f32) {
        self.set_nonuniform_scale_vec(Vector3::new(x, y, z));
    }

    /// Sets the global scale, preserving the current global orientation.
    pub fn set_nonuniform_scale_vec(&mut self, scale: Vector3<f32>) {
        let global_scale = self.scale();
        let scale_delta = scale.component_div(&global_scale);
        let new_local_scale = self.scale.component_mul(&scale_delta);

        self.set_nonuniform_local_scale(new_local_scale);
    }

    /// Sets the global scale uniformly.
    pub fn set_scale(&mut self, factor: f32) {
        self.set_nonuniform_scale_vec(Vector3::new(factor, factor, factor));
    }

    /// Returns the global scale factors.
    pub fn scale(&self) -> Vector3<f32> {
        let global_scale = self.get_global_scale_matrix();
        global_scale.vector
    }

    /// Recalculates all cached matrices.
    pub fn regenerate_matrices(&mut self) {
        self.recalculate_pos_matrix();
        self.recalculate_scale_matrix();
        self.recalculate_combined_matrix();
    }

    fn recalculate_pos_matrix(&mut self) {
        let pos = if self.invert_position {
            -self.pos
        } else {
            self.pos
        };
        self.pos_mat = Translation3::from(pos);
        self.recalculate_combined_matrix()
    }

    fn recalculate_scale_matrix(&mut self) {
        self.scale_mat = Scale3::from(self.scale);
        self.recalculate_combined_matrix()
    }

    pub fn set_compound_pos_first(&mut self, state: bool) {
        self.compound_pos_first = state;
    }

    fn recalculate_combined_matrix(&mut self) {
        if self.compound_pos_first {
            self.compound_mat = Affine3::from_matrix_unchecked(
                self.pos_mat.to_homogeneous()
                    * self.rot.to_homogeneous()
                    * self.scale_mat.to_homogeneous(),
            );
        } else {
            self.compound_mat = Affine3::from_matrix_unchecked(
                self.rot.to_homogeneous()
                    * self.pos_mat.to_homogeneous()
                    * self.scale_mat.to_homogeneous(),
            );
        }
    }

    /// Returns a reference to the combined transformation matrix.
    pub fn full_matrix(&self) -> &Affine3<f32> {
        &self.compound_mat
    }

    /// Returns the forward direction in world space.
    pub fn forward(&self) -> Vector3<f32> {
        self.rotation() * Vector3::new(0.0, 0.0, -1.0)
    }

    /// Returns the right direction in world space.
    pub fn right(&self) -> Vector3<f32> {
        self.rotation() * Vector3::new(1.0, 0.0, 0.0)
    }

    /// Returns the up direction in world space.
    pub fn up(&self) -> Vector3<f32> {
        self.rotation() * Vector3::new(0.0, 1.0, 0.0)
    }

    /// Returns the forward direction relative to the parent.
    pub fn local_forward(&self) -> Vector3<f32> {
        self.local_rotation() * Vector3::new(0.0, 0.0, -1.0)
    }

    /// Returns the right direction relative to the parent.
    pub fn local_right(&self) -> Vector3<f32> {
        self.local_rotation() * Vector3::new(1.0, 0.0, 0.0)
    }

    /// Returns the up direction relative to the parent.
    pub fn local_up(&self) -> Vector3<f32> {
        self.local_rotation() * Vector3::new(0.0, 1.0, 0.0)
    }
}
