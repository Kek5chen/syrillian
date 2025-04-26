use std::{error::Error, sync::{RwLock, RwLockWriteGuard}};

use log::{error, warn};

use crate::{asset_management::{materialmanager::MaterialId, Mesh, MeshId, MeshManager, RuntimeMesh, ShaderId}, buffer::UNIT_SQUARE, World};

use super::Drawable;

static UNIT_SQUARE_ID: RwLock<Option<MeshId>> = RwLock::new(None);

// TODO: Finish Image

pub struct Image {
    material: MaterialId,
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl Image {
    pub fn new(material: MaterialId) -> Box<Image> {
        Box::new(Image {
            material,
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        })
    }

    pub fn set_left(&mut self, left: u32) {
        self.left = left;
    }

    pub fn set_right(&mut self, right: u32) {
        self.right = right;
    }

    pub fn set_top(&mut self, top: u32) {
        self.top = top;
    }

    pub fn set_bottom(&mut self, bottom: u32) {
        self.bottom = bottom;
    }

    pub fn left(&self) -> u32 {
        self.left
    }

    pub fn right(&self) -> u32 {
        self.right
    }

    pub fn top(&self) -> u32 {
        self.top
    }

    pub fn bottom(&self) -> u32 {
        self.bottom
    }
}

impl Drawable for Image {
    fn setup(
            &mut self,
            _device: &wgpu::Device,
            _queue: &wgpu::Queue,
            world: &mut World,
        ) {
        ensure_unit_square(world);
    }

    fn draw(&self, world: &mut World, rpass: &mut wgpu::RenderPass, default_shader: Option<ShaderId>) {
        let unit_square_runtime = match unit_square_mesh(&world.assets.meshes) {
            Ok(s) => s,
            Err(e) => {
                warn!("Can't render image because the unit square mesh couldn't be fetched: {e}");
                return;
            },
        };

        let Some(material) = world
            .assets
            .materials
            .get_runtime_material(self.material) else {
            error!("Runtime Material not available.");
            return;
        };

        let Some(shader) = world
            .assets
            .shaders
            .get_shader(material.shader) else {
            warn!("Shader not found");
            return;
        };

        rpass.set_pipeline(&shader.pipeline);

        let vertex_buf_slice = unit_square_runtime.data.vertices_buf.slice(..);
        let mesh_bind_group = &unit_square_runtime.data.model_bind_group;
        let material_bind_group = &material.bind_group;
        let vertices_count = unit_square_runtime.data.vertices_num as u32;

        rpass.set_vertex_buffer(0, vertex_buf_slice);
        rpass.set_bind_group(1, mesh_bind_group, &[]);
        rpass.set_bind_group(2, material_bind_group, &[]);
        rpass.draw(0..vertices_count, 0..1)
    }
}

fn unit_square_mesh(mesh_manager: &MeshManager) -> Result<&RuntimeMesh, Box<dyn Error>> {
    let unit_square_id = UNIT_SQUARE_ID.read().unwrap();
    let Some(id) = *unit_square_id else {
        return Err("Unit Square ID should've been set in setup()".into());
    };

    let Some(unit_square_runtime) = mesh_manager
        .get_runtime_mesh(id) else {
        return Err("Unit Square Mesh should exist.".into());
    };

    Ok(unit_square_runtime)
}

fn ensure_unit_square(world: &mut World) {
    let unit_square = UNIT_SQUARE_ID.write().unwrap();
    if unit_square.is_some() {
        return;
    }
    remake_unit_square(world, unit_square);
}

fn remake_unit_square(world: &mut World, mut unit_square: RwLockWriteGuard<'_, Option<MeshId>>) {
    let id = world
        .assets
        .meshes
        .add_mesh(
            Mesh::new(
                UNIT_SQUARE.to_vec(),
                None,
                None
            )
        );
    *unit_square = Some(id);
}
