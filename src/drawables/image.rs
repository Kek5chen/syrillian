use std::{error::Error, sync::{RwLock, RwLockWriteGuard}};

use log::{error, warn};
use nalgebra::{Matrix4, Scale3, Translation3};
use wgpu::{util::{BufferInitDescriptor, DeviceExt}, BindGroupDescriptor, BindGroupEntry, BufferUsages};
use winit::window::Window;

use crate::{asset_management::{bindgroup_layout_manager::MODEL_UBGL_ID, materialmanager::MaterialId, Mesh, MeshId, MeshManager, RuntimeMesh, DIM2_SHADER_ID}, buffer::UNIT_SQUARE, object::{GameObjectId, ModelData}, renderer::Renderer, World};

use super::Drawable;

static UNIT_SQUARE_ID: RwLock<Option<MeshId>> = RwLock::new(None);

struct ImageGPUData {
    model_data: ModelData,
    model_data_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
}

pub struct Image {
    material: MaterialId,
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
    initialized: bool,
    gpu_data: Option<ImageGPUData>,
}

impl Image {
    pub fn new(material: MaterialId) -> Box<Image> {
        Box::new(Image {
            material,
            left: 0,
            right: 0,
            bottom: 0,
            top: 0,
            initialized: false,
            gpu_data: None,
        })
    }

    pub fn set_left(&mut self, left: u32) {
        self.left = left.min(self.right);
        self.initialized = true;
    }

    pub fn set_right(&mut self, right: u32) {
        self.right = right.max(self.left);
        self.initialized = true;
    }

    pub fn set_top(&mut self, top: u32) {
        self.top = top.max(self.bottom);
        self.initialized = true;
    }

    pub fn set_bottom(&mut self, bottom: u32) {
        self.bottom = bottom.min(self.top);
        self.initialized = true;
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
            renderer: &Renderer,
            world: &mut World,
        ) {
        ensure_unit_square(world);

        self.setup_model_data(world, &renderer.state.device);
    }

    fn update(
            &mut self,
            _world: &mut World,
            _parent: GameObjectId,
            renderer: &Renderer,
            _outer_transform: &nalgebra::Matrix4<f32>,
        ) {
        self.update_model_matrix(&renderer.state.queue, &renderer.window);
    }

    fn draw(&self, world: &mut World, rpass: &mut wgpu::RenderPass, _renderer: &Renderer) {
        let unit_square_runtime = match unit_square_mesh(&mut world.assets.meshes) {
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
            .get_shader(Some(DIM2_SHADER_ID)) else {
            error!("2 Dimensional Shader Pipeline is not available.");
            return;
        };

        let Some(gpu_data) = &self.gpu_data else {
            error!("Image GPU Data wasn't set up.");
            return;
        };

        rpass.set_pipeline(&shader.pipeline);

        let vertex_buf_slice = unit_square_runtime.data.vertices_buf.slice(..);
        let material_bind_group = &material.bind_group;
        let vertices_count = unit_square_runtime.data.vertices_num as u32;

        rpass.set_vertex_buffer(0, vertex_buf_slice);
        rpass.set_bind_group(1, &gpu_data.model_bind_group, &[]);
        rpass.set_bind_group(2, material_bind_group, &[]);
        rpass.draw(0..vertices_count, 0..1)
    }
}

impl Image {
    fn setup_model_data(&mut self, world: &World, device: &wgpu::Device) {
        let bgl = world.assets.bind_group_layouts.get_bind_group_layout(MODEL_UBGL_ID).unwrap();

        let model_data = ModelData::empty();
        let model_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image Model Buffer"),
            contents: bytemuck::bytes_of(&model_data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let model_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Image Model Bind Group"),
            layout: bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: model_data_buffer.as_entire_binding(),
            }],
        });

        self.gpu_data = Some(ImageGPUData {
            model_data,
            model_data_buffer,
            model_bind_group,
        });
    }

    fn calculate_model_matrix(&self, window_width: f32, window_height: f32) -> Matrix4<f32> {
        if self.right == 0 || self.top == 0 {
            return Matrix4::zeros();
        }

        let left   = (self.left   as f32 / window_width)  * 2.0 - 1.0;
        let right  = (self.right  as f32 / window_width)  * 2.0 - 1.0;
        let bottom = (self.bottom as f32 / window_height) * 2.0 - 1.0;
        let top    = (self.top    as f32 / window_height) * 2.0 - 1.0;

        let sx = (right - left)  * 0.5;
        let sy = (top   - bottom) * 0.5;

        // clip space
        let tx = (right + left)   * 0.5;
        let ty = (top   + bottom) * 0.5;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    fn update_model_matrix(&mut self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let new_model_mat = self.calculate_model_matrix(window_size.width as f32, window_size.height as f32);

        let Some(gpu_data) = &mut self.gpu_data else {
            error!("GPU data not set");
            return;
        };
        gpu_data.model_data.model_mat = new_model_mat;

        queue.write_buffer(
            &gpu_data.model_data_buffer,
            0,
            bytemuck::bytes_of(&gpu_data.model_data)
        );
    }
}

fn unit_square_mesh(mesh_manager: &mut MeshManager) -> Result<&RuntimeMesh, Box<dyn Error>> {
    let unit_square_id = UNIT_SQUARE_ID.read().unwrap();
    let Some(id) = *unit_square_id else {
        return Err("Unit Square ID should've been set in setup()".into());
    };

    let Some(unit_square_runtime) = mesh_manager
        .get_runtime_mesh_or_init(id) else {
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
