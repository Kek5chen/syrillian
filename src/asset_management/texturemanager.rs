use std::collections::HashMap;
use std::rc::Rc;

use wgpu::{AddressMode, Device, Extent3d, Queue, SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension};
use wgpu::util::{DeviceExt, TextureDataOrder};

pub const FALLBACK_DIFFUSE_TEXTURE: TextureId = 0;
pub const FALLBACK_NORMAL_TEXTURE: TextureId = 1;
pub const FALLBACK_SHININESS_TEXTURE: TextureId = 2;

#[allow(dead_code)]
pub struct RuntimeTexture {
    texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
}

pub struct RawTexture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub data: Option<Vec<u8>>,
}

pub struct Texture {
    pub raw: RawTexture,
    pub runtime: Option<RuntimeTexture>,
}

pub type TextureId = usize;

#[allow(dead_code)]
pub struct TextureManager {
    textures: HashMap<TextureId, Texture>,
    next_id: TextureId,
    device: Option<Rc<Device>>,
    queue: Option<Rc<Queue>>,
}

impl Default for TextureManager {
    fn default() -> Self {
        let mut manager = TextureManager {
            textures: HashMap::new(),
            next_id: 0,
            device: None,
            queue: None,
        };

        manager.init();

        manager
    }
}

#[allow(dead_code)]
impl TextureManager {
    pub fn generate_new_fallback_diffuse_texture(width: u32, height: u32) -> Vec<u8> {
        let mut diffuse = vec![];
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                if x % 2 == y % 2 {
                    diffuse.extend_from_slice(&[0, 0, 0, 255]);
                } else {
                    diffuse.extend_from_slice(&[255, 0, 255, 255]);
                }
            }
        }
        diffuse
    }

    pub fn init(&mut self) {
        const FALLBACK_SIZE: u32 = 35;

        let id = self.add_texture(
            FALLBACK_SIZE,
            FALLBACK_SIZE,
            TextureFormat::Bgra8UnormSrgb,
            Some(Self::generate_new_fallback_diffuse_texture(FALLBACK_SIZE, FALLBACK_SIZE)),
        );
        assert_eq!(id, FALLBACK_DIFFUSE_TEXTURE);

        let id = self.add_texture(1, 1, TextureFormat::Bgra8UnormSrgb, Some(vec![0, 0, 0, 0]));
        assert_eq!(id, FALLBACK_NORMAL_TEXTURE);

        let id = self.add_texture(1, 1, TextureFormat::Bgra8UnormSrgb, Some(vec![0, 0, 0, 0]));
        assert_eq!(id, FALLBACK_SHININESS_TEXTURE);
    }

    pub fn init_runtime(&mut self, device: Rc<Device>, queue: Rc<Queue>) {
        self.device = Some(device);
        self.queue = Some(queue);
    }
    
    pub fn invalidate_runtime(&mut self) {
        for (_, tex) in &mut self.textures {
            tex.runtime = None;
        }
        
        self.device = None;
        self.queue = None;
    }

    pub fn add_texture(
        &mut self,
        width: u32,
        height: u32,
        format: TextureFormat,
        data: Option<Vec<u8>>,
    ) -> TextureId {
        let raw = RawTexture {
            width,
            height,
            format,
            data,
        };
        let id = self.next_id;

        let texture = Texture { raw, runtime: None };

        self.textures.insert(id, texture);
        self.next_id += 1;

        id
    }

    fn get_internal_texture_mut(&mut self, texture: TextureId) -> Option<&mut Texture> {
        self.textures.get_mut(&texture)
    }

    pub fn get_raw_texture(&self, texture: TextureId) -> Option<&RawTexture> {
        let tex = self.textures.get(&texture)?;
        Some(&tex.raw)
    }

    pub fn get_runtime_texture(&self, texture: TextureId) -> Option<&RuntimeTexture> {
        let tex = self.textures.get(&texture)?;
        tex.runtime.as_ref()
    }

    pub fn get_runtime_texture_ensure_init(
        &mut self,
        texture: TextureId,
    ) -> Option<&RuntimeTexture> {
        let device = self.device.clone().unwrap();
        let queue = self.queue.clone().unwrap();
        let tex = self.get_internal_texture_mut(texture)?;
        if tex.runtime.is_some() {
            return tex.runtime.as_ref();
        }
        let runtime_texture = tex.initialize_texture(device.as_ref(), queue.as_ref());
        Some(runtime_texture)
    }
}

impl Texture {
    fn initialize_texture(&mut self, device: &Device, queue: &Queue) -> &RuntimeTexture {
        if self.runtime.is_some() {
            self.runtime = None;
        }
        let raw = &self.raw;

        let gpu_tex = match self.raw.data {
            None => self.initialize_empty_texture(device, raw),
            Some(_) => self.initialize_preset_texture(device, queue, raw),
        };
        let view = gpu_tex.create_view(&TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(TextureFormat::Bgra8UnormSrgb),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: Default::default(),
            min_filter: Default::default(),
            mipmap_filter: Default::default(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });
        let run_texture = RuntimeTexture {
            texture: gpu_tex,
            view,
            sampler,
        };

        self.runtime = Some(run_texture);
        self.runtime.as_ref().unwrap()
    }
    
    fn initialize_texture_descriptor(&self, raw: &RawTexture) -> TextureDescriptor{
        TextureDescriptor {
            label: Some("Texture"),
            size: Extent3d {
                width: raw.width,
                height: raw.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: raw.format,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
        }
    }
    
    fn initialize_preset_texture(&self, device: &Device, queue: &Queue, raw: &RawTexture) -> wgpu::Texture {
        device.create_texture_with_data(
            queue,
            &self.initialize_texture_descriptor(raw),
            TextureDataOrder::LayerMajor,
            raw.data.as_ref().expect("Data should be set."),
        )
    }
    
    fn initialize_empty_texture(&self, device: &Device, raw: &RawTexture) -> wgpu::Texture {
        device.create_texture(
            &self.initialize_texture_descriptor(raw),
        )
    }
}
