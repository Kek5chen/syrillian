use std::collections::HashMap;
use std::rc::Rc;

use log::{error, warn};
use nalgebra::Vector3;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
    BufferUsages, Device, Queue,
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use crate::asset_management::bindgroup_layout_manager::MATERIAL_UBGL_ID;
use crate::asset_management::shadermanager;
use crate::asset_management::shadermanager::ShaderId;
use crate::asset_management::texturemanager::{
    FALLBACK_DIFFUSE_TEXTURE, FALLBACK_NORMAL_TEXTURE, FALLBACK_SHININESS_TEXTURE, TextureId,
};
use crate::ensure_aligned;
use crate::world::World;

pub type MaterialId = usize;

pub const FALLBACK_MATERIAL_ID: usize = 0;

// TODO: Consider Builder Pattern
//
#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub diffuse: Vector3<f32>,
    pub diffuse_texture: Option<TextureId>,
    pub normal_texture: Option<TextureId>,
    pub shininess: f32,
    pub shininess_texture: Option<TextureId>,
    pub opacity: f32,
    pub shader: Option<ShaderId>,
}

#[derive(Debug)]
pub struct MaterialItem {
    raw: Material,
    runtime: Option<RuntimeMaterial>,
}

impl Material {
    pub(crate) fn init_runtime(
        &self,
        world: &mut World,
        device: &Device,
        _queue: &Queue,
    ) -> RuntimeMaterial {
        let data = MaterialUniform {
            diffuse: self.diffuse,
            _padding: 0xFFFFFFFF,
            use_diffuse_texture: self.diffuse_texture.is_some() as u32,
            use_normal_texture: self.normal_texture.is_some() as u32,
            shininess: self.shininess,
            opacity: self.opacity,
        };

        let material_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Material Data Buffer"),
            contents: bytemuck::bytes_of(&data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let world: *mut World = world;
        unsafe {
            let diffuse_texture_id = self.diffuse_texture.unwrap_or(FALLBACK_DIFFUSE_TEXTURE);
            let diffuse_texture = (*world)
                .assets
                .textures
                .get_runtime_texture_ensure_init(diffuse_texture_id)
                .unwrap();

            let normal_texture_id = self.diffuse_texture.unwrap_or(FALLBACK_NORMAL_TEXTURE);
            let normal_texture = (*world)
                .assets
                .textures
                .get_runtime_texture_ensure_init(normal_texture_id)
                .unwrap();

            let shininess_texture_id = self.diffuse_texture.unwrap_or(FALLBACK_SHININESS_TEXTURE);
            // TODO: Implement shininess texture into bind group
            let _shininess_texture = (*world)
                .assets
                .textures
                .get_runtime_texture_ensure_init(shininess_texture_id)
                .unwrap();

            let mat_bgl = (*world).assets.bind_group_layouts.get_bind_group_layout(MATERIAL_UBGL_ID).unwrap();

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Material Bind Group"),
                layout: mat_bgl,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: material_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&diffuse_texture.view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&normal_texture.view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&normal_texture.sampler),
                    },
                ],
            });

            RuntimeMaterial {
                data,
                buffer: material_buffer,
                bind_group,
                shader: self.shader
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    diffuse: Vector3<f32>,
    use_diffuse_texture: u32,
    use_normal_texture: u32,
    shininess: f32,
    opacity: f32,
    _padding: u32,
}

ensure_aligned!(MaterialUniform { diffuse }, align <= 16 * 2 => size);

#[allow(dead_code)]
#[derive(Debug)]
pub struct RuntimeMaterial {
    pub(crate) data: MaterialUniform,
    pub(crate) buffer: Buffer,
    pub(crate) bind_group: BindGroup,
    pub(crate) shader: Option<ShaderId>,
}

#[derive(Debug)]
pub enum MaterialError {
    MaterialNotFound,
    DeviceNotInitialized,
    QueueNotInitialized,
}

#[derive(Debug)]
pub struct MaterialManager {
    materials: HashMap<usize, MaterialItem>,
    next_id: MaterialId,
    device: Option<Rc<Device>>,
    queue: Option<Rc<Queue>>,
}

impl Default for MaterialManager {
    fn default() -> Self {
        let fallback = Material {
            name: "Fallback Material".to_string(),
            diffuse: Vector3::new(1.0, 1.0, 1.0),
            diffuse_texture: None,
            normal_texture: None,
            shininess: 0.0,
            shader: Some(shadermanager::DIM3_SHADER_ID),
            opacity: 1.0,
            shininess_texture: None,
        };
        let mut manager = MaterialManager {
            materials: HashMap::new(),
            next_id: 0,
            device: None,
            queue: None,
        };
        manager.add_material(fallback);
        manager
    }
}

#[allow(dead_code)]
impl MaterialManager {
    pub fn invalidate_runtime(&mut self) {
        for mat in self.materials.values_mut() {
            mat.runtime = None;
        }
        self.device = None;
        self.queue = None;
    }

    pub fn init_runtime(
        &mut self,
        device: Rc<Device>,
        queue: Rc<Queue>,
    ) {
        self.device = Some(device.clone());
        self.queue = Some(queue.clone());
    }

    pub fn add_material(&mut self, material: Material) -> MaterialId {
        let id = self.next_id;

        self.materials.insert(
            id,
            MaterialItem {
                raw: material,
                runtime: None,
            },
        );
        self.next_id += 1;

        id
    }

    pub fn get_material_internal_mut(&mut self, id: MaterialId) -> Option<&mut MaterialItem> {
        self.materials.get_mut(&id).or_else(|| {
            warn!("Material not found for id {}", id);
            None
        })
    }

    pub fn get_raw_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id).map(|m| &m.raw)
    }

    pub fn get_runtime_material(&mut self, id: MaterialId) -> Option<&RuntimeMaterial> {
        self.get_runtime_material_mut(id).map(|mat| &*mat).or_else(|| {
            error!("Runtime Material not available.");
            None
        })
    }

    pub fn get_runtime_material_mut(&mut self, id: MaterialId) -> Option<&mut RuntimeMaterial> {
        let Some(mat) = self.materials.get_mut(&id) else {
            warn!("Material not found for id {}", id);
            return None;
        };

        if mat.runtime.is_none() {
            mat.runtime = Some(mat.raw.init_runtime(
                World::instance(),
                self.device.as_ref()?.as_ref(),
                self.queue.as_ref()?.as_ref(),
            ));
        }
        mat.runtime.as_mut()
    }

    fn init_runtime_material_internal(
        world: &mut World,
        material: &mut MaterialItem,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), MaterialError> {
        material.runtime = Some(material.raw.init_runtime(
            world,
            device,
            queue,
        ));
        Ok(())
    }

    pub fn init_runtime_material(&self, world: &mut World, material: &mut MaterialItem) -> Result<(), MaterialError> {
        let device = self.device.as_ref()
            .ok_or(MaterialError::DeviceNotInitialized)?;
        let queue = self.queue.as_ref()
            .ok_or(MaterialError::QueueNotInitialized)?;
        Self::init_runtime_material_internal(world, material, device, queue)
    }
    
    pub fn init_runtime_material_id(&mut self, world: &mut World, id: MaterialId) -> Result<(), MaterialError> {
        let material = self.materials.get_mut(&id)
            .ok_or(MaterialError::MaterialNotFound)?;
        let device = self.device.as_ref()
            .ok_or(MaterialError::DeviceNotInitialized)?;
        let queue = self.queue.as_ref()
            .ok_or(MaterialError::QueueNotInitialized)?;

        Self::init_runtime_material_internal(world, material, device, queue)
    }
}
