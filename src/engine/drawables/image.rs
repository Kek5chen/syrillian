use std::{error::Error, sync::{RwLock, RwLockWriteGuard}};

use log::{error, warn};
use nalgebra::{Matrix4, Scale3, Translation3};
use wgpu::{util::{BufferInitDescriptor, DeviceExt}, BindGroupDescriptor, BindGroupEntry, BufferUsages};
use winit::window::Window;
use crate::asset_management::{MaterialId, Mesh, MeshId, MeshManager, RuntimeMesh, ShaderId, DIM2_SHADER_ID, MODEL_UBGL_ID};
use crate::core::{Bones, GameObjectId, ModelUniform};
use crate::engine::rendering::Renderer;
use crate::utils::UNIT_SQUARE;
use crate::World;
use super::{BoneData, Drawable};

static UNIT_SQUARE_ID: RwLock<Option<MeshId>> = RwLock::new(None);

#[derive(Debug, Clone, Copy)]
pub enum ImageScalingMode {
    Absolute {
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    Relative {
        width: u32,
        height: u32,
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    RelativeStretch {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    },
}

#[derive(Debug)]
struct ImageGPUData {
    translation_data: ModelUniform,
    translation_data_buffer: wgpu::Buffer,

    _dummy_bone_data: BoneData,
    _dummy_bone_data_buffer: wgpu::Buffer,

    model_bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Image {
    material: MaterialId,
    scaling: ImageScalingMode,
    gpu_data: Option<ImageGPUData>,
}

impl Image {
    pub fn new(material: MaterialId) -> Box<Image> {
        Box::new(Image {
            material,
            scaling: ImageScalingMode::Absolute {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
            gpu_data: None,
        })
    }

    pub fn new_with_size(material: MaterialId, scaling: ImageScalingMode) -> Box<Image> {
        Box::new(Image {
            material,
            scaling,
            gpu_data: None,
        })
    }

    pub fn scaling_mode(&self) -> ImageScalingMode {
        self.scaling
    }

    pub fn set_scaling_mode(&mut self, scaling: ImageScalingMode) {
        self.scaling = scaling;
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
            _outer_transform: &Matrix4<f32>,
        ) {
        self.update_model_matrix(&renderer.state.queue, &renderer.window);
    }

    fn draw(
        &self,
        world: &mut World,
        rpass: &mut wgpu::RenderPass,
        _renderer: &Renderer,
        _shader_override: Option<ShaderId>,
    ) {
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
            return;
        };

        let Some(shader) = world
            .assets
            .shaders
            .get_shader_opt(DIM2_SHADER_ID) else {
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

        let translation_data = ModelUniform::empty();
        let translation_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image Translation Buffer"),
            contents: bytemuck::bytes_of(&translation_data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let _dummy_bone_data = BoneData::default();
        let _dummy_bone_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image Dummy Bone Buffer"),
            contents: _dummy_bone_data.as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });


        let model_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Image Model Bind Group"),
            layout: bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: translation_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: _dummy_bone_data_buffer.as_entire_binding(),
                }
            ],
        });

        self.gpu_data = Some(ImageGPUData {
            translation_data,
            translation_data_buffer,

            _dummy_bone_data,
            _dummy_bone_data_buffer,

            model_bind_group,
        });
    }

    fn calculate_model_matrix_absolute(&self, window_width: f32, window_height: f32) -> Matrix4<f32> {
        let ImageScalingMode::Absolute { left, right, top, bottom } = self.scaling else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let left   = (left   as f32 / window_width)  * 2.0 - 1.0;
        let right  = (right  as f32 / window_width)  * 2.0 - 1.0;
        let bottom = (bottom as f32 / window_height) * 2.0 - 1.0;
        let top    = (top    as f32 / window_height) * 2.0 - 1.0;

        let sx = (right - left)   * 0.5;
        let sy = (top   - bottom) * 0.5;

        // clip space
        let tx = (right + left)   * 0.5;
        let ty = (top   + bottom) * 0.5;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    fn calculate_model_matrix_relative(&self) -> Matrix4<f32> {
        let ImageScalingMode::Relative { 
            width,
            height,
            left,
            right,
            top,
            bottom,
        } = self.scaling else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let width = width as f32;
        let height = height as f32;

        let left   = (left   as f32 / width)  * 2.0 - 1.0;
        let right  = (right  as f32 / width)  * 2.0 - 1.0;
        let bottom = (bottom as f32 / height) * 2.0 - 1.0;
        let top    = (top    as f32 / height) * 2.0 - 1.0;

        let sx = (right - left)   * 0.5;
        let sy = (top   - bottom) * 0.5;

        // clip space
        let tx = (right + left)   * 0.5;
        let ty = (top   + bottom) * 0.5;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    fn calculate_model_matrix_relative_stretch(&self) -> Matrix4<f32> {
        let ImageScalingMode::RelativeStretch { left, right, top, bottom } = self.scaling else {
            return Matrix4::zeros();
        };

        if right <= left || top <= bottom {
            return Matrix4::zeros();
        }

        let sx = right - left;
        let sy = top   - bottom;

        let tx = left   + right - 1.0;
        let ty = bottom + top   - 1.0;

        Translation3::new(tx, ty, 0.0).to_homogeneous()
            * Scale3::new(sx, sy, 1.0).to_homogeneous()
    }

    fn calculate_model_matrix(&self, window_width: f32, window_height: f32) -> Matrix4<f32> {
        match self.scaling {
            ImageScalingMode::Absolute {..} => self.calculate_model_matrix_absolute(window_width, window_height),
            ImageScalingMode::Relative {..} => self.calculate_model_matrix_relative(),
            ImageScalingMode::RelativeStretch {..} => self.calculate_model_matrix_relative_stretch(),
        }
    }

    fn update_model_matrix(&mut self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let width = window_size.width as f32;
        let height = window_size.height as f32;

        let new_model_mat = self.calculate_model_matrix(width, height);

        let Some(gpu_data) = &mut self.gpu_data else {
            error!("GPU data not set");
            return;
        };
        gpu_data.translation_data.model_mat = new_model_mat;

        queue.write_buffer(
            &gpu_data.translation_data_buffer,
            0,
            bytemuck::bytes_of(&gpu_data.translation_data)
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
                None,
                Bones::none(),
            )
        );

    *unit_square = Some(id);
}
