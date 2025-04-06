use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, Device, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension};

pub type BGLId = usize;

#[derive(Debug)]
pub struct BindGroupLayoutDefinition {
    label: Option<&'static str>,
    entries: Vec<BindGroupLayoutEntry>,
}

#[derive(Debug)]
pub struct BindGroupLayoutItem {
    raw: BindGroupLayoutDefinition,
    runtime: Option<BindGroupLayout>,
}

impl BindGroupLayoutItem {
    pub fn init_runtime(&mut self, device: &Rc<Device>) {
        if self.runtime.is_some() {
            return;
        }

        let run_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: self.raw.label,
                entries: self.raw.entries.as_slice(),
            });
        self.runtime = Some(run_layout);
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutManager {
    layouts: HashMap<usize, BindGroupLayoutItem>,
    next_id: BGLId,
    device: Option<Rc<Device>>
}

pub const CAMERA_UBGL_ID: BGLId = 0;
pub const MODEL_UBGL_ID: BGLId = 1;
pub const MATERIAL_UBGL_ID: BGLId = 2;
pub const POST_PROCESS_BGL_ID: BGLId = 3;

impl Default for BindGroupLayoutManager {
    fn default() -> Self {
        let mut manager = Self {
            layouts: HashMap::new(),
            next_id: 0,
            device: None,
        };

        let id = manager.add_bind_group_layout(Some("Camera Uniform Bind Group Layout"), vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );
        assert_eq!(id, CAMERA_UBGL_ID);

        let id = manager.add_bind_group_layout(Some("Model Uniform Bind Group Layout"), vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }]
        );
        assert_eq!(id, MODEL_UBGL_ID);

        let id = manager.add_bind_group_layout(Some("Material Uniform Bind Group Layout"), vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            },
        ]);
        assert_eq!(id, MATERIAL_UBGL_ID);

        let id = manager.add_bind_group_layout(Some("Post-Processing Bind Group Layout"), vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ]);
        assert_eq!(id, POST_PROCESS_BGL_ID);

        manager
    }
}

impl BindGroupLayoutManager {
    pub fn init_runtime(&mut self, device: Rc<Device>) {
        self.device = Some(device);
        self.layouts.values_mut().for_each(|item| item.runtime = None );


        self.init_all_runtime();
    }

    pub fn init_all_runtime(&mut self) {
        let device = self.device.clone().unwrap();
        for layout in self.layouts.values_mut() {
            layout.init_runtime(&device);
        }
    }

    pub fn get_bind_group_layout(&self, id: BGLId) -> Option<&BindGroupLayout> {
        self.layouts.get(&id)?
            .runtime.as_ref()
    }

    pub fn get_bind_group_layout_mut(&mut self, id: BGLId) -> Option<&mut BindGroupLayout> {
        self.layouts.get_mut(&id)?
            .runtime.as_mut()
    }

    pub fn add_bind_group_layout(&mut self, label: Option<&'static str>, entries: Vec<BindGroupLayoutEntry>) -> BGLId {
        self.layouts.insert(self.next_id, BindGroupLayoutItem {
            raw: BindGroupLayoutDefinition {
                label,
                entries,
            },
            runtime: None,
        });
        if let Some(device) = &self.device {
            self.layouts.get_mut(&self.next_id).unwrap().init_runtime(device);
        }

        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
