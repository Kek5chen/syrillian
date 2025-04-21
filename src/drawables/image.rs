use std::sync::RwLock;

use log::error;

use crate::{asset_management::{materialmanager::MaterialId, Mesh, MeshId}, buffer::UNIT_SQUARE, World};

use super::Drawable;

static UNIT_SQUARE_ID: RwLock<Option<MeshId>> = RwLock::new(None);

// TODO: Finish Image

pub struct Image {
    material: MaterialId,
    left: u32,
    right: u32,
    up: u32,
    bottom: u32,
}

impl Image {
    pub fn new(material: MaterialId) -> Box<Image> {
        Box::new(Image {
            material,
            left: 0,
            right: 0,
            up: 0,
            bottom: 0,
        })
    }
}

impl Drawable for Image {
    fn setup(
            &mut self,
            _device: &wgpu::Device,
            _queue: &wgpu::Queue,
            world: &mut World,
        ) {
        let mut unit_square = UNIT_SQUARE_ID.write().unwrap();
        if unit_square.is_some() {
            return;
        }

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

    fn draw(&self, world: &mut World, rpass: &mut wgpu::RenderPass) {
        let unit_square_id = UNIT_SQUARE_ID.read().unwrap();
        let Some(id) = *unit_square_id else {
            error!("Unit Square ID should've been set in setup()");
            return;
        };

        let Some(unit_square_runtime) = world
            .assets
            .meshes
            .get_runtime_mesh(id) else {
            error!("Unit Square Mesh should exist.");
            return;
        };

        let Some(material) = world
            .assets
            .materials
            .get_runtime_material(self.material) else {
            error!("Runtime Material not available.");
            return;
        };

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


















