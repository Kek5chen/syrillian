use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, Device, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension};

pub type BGLId = usize;

#[derive(Debug)]
pub struct LayoutDescriptor {
    label: Option<&'static str>,
    entries: Vec<BindGroupLayoutEntry>,
}

impl LayoutDescriptor {
    pub fn init_runtime(&self, device: &Rc<Device>) -> BindGroupLayout {
        device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: self.label,
                entries: self.entries.as_slice(),
            })
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutManager {
    raw_layouts: HashMap<usize, LayoutDescriptor>,
    runtime_layouts: HashMap<usize, BindGroupLayout>,
    next_id: BGLId,
    device: Option<Rc<Device>>
}

pub const RENDER_UBGL_ID: BGLId = 0;
pub const MODEL_UBGL_ID: BGLId = 1;
pub const MATERIAL_UBGL_ID: BGLId = 2;
pub const LIGHT_UBGL_ID: BGLId = 3;
pub const POST_PROCESS_BGL_ID: BGLId = 4;

impl Default for BindGroupLayoutManager {
    fn default() -> Self {
        let mut manager = Self {
            raw_layouts: HashMap::new(),
            runtime_layouts: HashMap::new(),
            next_id: 0,
            device: None,
        };
        
        let two_entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ];

        let id = manager.add_bind_group_layout(Some("Render Uniform Bind Group Layout"), two_entries.clone());
        assert_eq!(id, RENDER_UBGL_ID);

        let id = manager.add_bind_group_layout(Some("Model Uniform Bind Group Layout"), two_entries);
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

        let id = manager.add_bind_group_layout(
            Some("Light Bind Group Layout"),
            vec![
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer { 
                        ty: BufferBindingType::Storage { 
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        );
        assert_eq!(id, LIGHT_UBGL_ID);

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
        self.runtime_layouts.clear();


        self.init_all_runtime();
    }

    pub fn invalidate_runtime(&mut self) {
        self.device = None;
        self.runtime_layouts.clear();
    }

    pub fn init_all_runtime(&mut self) {
        let device = self.device.clone().unwrap();
        for (id, layout) in self.raw_layouts.iter() {
            let runtime_layout = layout.init_runtime(&device);
            self.runtime_layouts.insert(*id, runtime_layout);
        }
    }

    pub fn get_bind_group_layout(&self, id: BGLId) -> Option<&BindGroupLayout> {
        self.runtime_layouts.get(&id)
    }

    pub fn get_bind_group_layout_mut(&mut self, id: BGLId) -> Option<&mut BindGroupLayout> {
        self.runtime_layouts.get_mut(&id)
    }

    pub fn add_bind_group_layout(&mut self, label: Option<&'static str>, entries: Vec<BindGroupLayoutEntry>) -> BGLId {
        let descriptor = LayoutDescriptor {
            label,
            entries,
        };

        // init right away if a device already exists 
        if let Some(device) = &self.device {
            let runtime = descriptor.init_runtime(device);
            self.runtime_layouts.insert(self.next_id, runtime);
        }
        self.raw_layouts.insert(self.next_id, descriptor);

        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
