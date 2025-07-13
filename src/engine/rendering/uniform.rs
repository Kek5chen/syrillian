use smallvec::{SmallVec, smallvec};
use std::marker::PhantomData;
use syrillian_utils::{ShaderUniformIndex, ShaderUniformMultiIndex, ShaderUniformSingleIndex};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferAddress, BufferDescriptor, BufferUsages, Device, Sampler, TextureView,
};

#[derive(Debug, Clone)]
pub struct ShaderUniform<I: ShaderUniformIndex> {
    buffers: UniformBufferStorage<I>,
    bind_group: BindGroup,
}

pub struct ShaderUniformBuilder<'a, I: ShaderUniformIndex> {
    bind_group_layout: &'a BindGroupLayout,
    data: SmallVec<[ResourceDesc<'a, I>; 1]>,
}

#[allow(unused)]
enum ResourceDesc<'a, I: ShaderUniformIndex> {
    DataBuffer { data: &'a [u8], name: I },
    StorageBuffer { data: &'a [u8], name: I },
    EmptyBuffer { size: u64, name: I, map: bool },
    TextureView { view: &'a TextureView, name: I },
    Sampler { sampler: &'a Sampler, name: I },
}

#[derive(Debug, Clone)]
pub struct UniformBufferStorage<I: ShaderUniformIndex> {
    buffers: SmallVec<[Option<Buffer>; 1]>,

    _indexer: PhantomData<I>,
}

#[allow(unused)]
impl<'a, I: ShaderUniformIndex + 'static> ShaderUniformBuilder<'a, I> {
    #[inline]
    pub fn build(self, device: &Device) -> ShaderUniform<I> {
        let buffers = UniformBufferStorage::new(&device, &self.data);
        let bind_group = self.bind_group(&device, self.bind_group_layout, &buffers);

        ShaderUniform {
            buffers,
            bind_group,
        }
    }

    #[inline]
    fn _next_index(&self) -> I {
        let idx = self.data.len() as u64;
        I::by_index(idx).unwrap_or_else(|| {
            panic!(
                "The buffer index #{idx} was not registered as a member of shader uniform {}",
                I::name()
            );
        })
    }

    #[inline]
    pub fn with_buffer_data<B>(mut self, data: &'a B) -> Self
    where
        B: bytemuck::Pod + bytemuck::Zeroable + 'a,
    {
        let name = self._next_index();
        let data = bytemuck::bytes_of(data);
        self.data.push(ResourceDesc::DataBuffer { data, name });
        self
    }

    #[inline]
    pub fn with_buffer_data_slice<B>(mut self, data: &'a [B]) -> Self
    where
        B: bytemuck::Pod + bytemuck::Zeroable + 'a,
    {
        let name = self._next_index();
        let data = bytemuck::cast_slice(data);
        self.data.push(ResourceDesc::DataBuffer { data, name });
        self
    }

    #[inline]
    pub fn with_buffer_storage<B>(mut self, data: &'a [B]) -> Self
    where
        B: bytemuck::Pod + bytemuck::Zeroable + 'a,
    {
        let name = self._next_index();
        let data = bytemuck::cast_slice(data);
        self.data.push(ResourceDesc::StorageBuffer { data, name });
        self
    }

    #[inline]
    pub fn with_buffer_sized(mut self, size: u64, map: bool) -> Self {
        let name = self._next_index();
        self.data
            .push(ResourceDesc::EmptyBuffer { size, name, map });
        self
    }

    #[inline]
    pub fn with_texture_view(mut self, view: &'a TextureView) -> Self {
        let name = self._next_index();
        self.data.push(ResourceDesc::TextureView { view, name });
        self
    }

    #[inline]
    pub fn with_sampler(mut self, sampler: &'a Sampler) -> Self {
        let name = self._next_index();
        self.data.push(ResourceDesc::Sampler { sampler, name });
        self
    }

    #[inline]
    fn entries<'b>(&'b self, buffers: &'b UniformBufferStorage<I>) -> SmallVec<[BindGroupEntry<'b>; 1]> {
            self.data
                .iter()
                .map(|desc| desc.entry(buffers))
                .collect()
    }

    #[inline]
    fn bind_group(
        &self,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        buffers: &UniformBufferStorage<I>,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{} Uniform Bind Group", I::name())),
            layout: bind_group_layout,
            entries: &self.entries(buffers),
        })
    }
}

impl<I: ShaderUniformIndex> ShaderUniform<I> {
    #[inline]
    pub fn builder(bind_group_layout: &BindGroupLayout) -> ShaderUniformBuilder<'_, I> {
        ShaderUniformBuilder {
            bind_group_layout,
            data: smallvec![],
        }
    }

    #[inline]
    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}

impl<I: ShaderUniformMultiIndex> ShaderUniform<I> {
    pub fn buffer(&self, idx: I) -> &Buffer {
        self.buffers.buffers[idx.index() as usize]
            .as_ref()
            .expect("Requested a binding resource that isn't a buffer")
    }
}

#[allow(unused)]
impl<I: ShaderUniformSingleIndex> ShaderUniform<I> {
    pub fn buffer_one(&self) -> &Buffer {
        self.buffers.buffers[0]
            .as_ref()
            .expect("Requested a binding resource that isn't a buffer")
    }
}

impl<I: ShaderUniformIndex> UniformBufferStorage<I> {
    #[inline]
    fn new(device: &Device, desc: &[ResourceDesc<I>]) -> Self {
        assert_eq!(desc.len(), I::MAX + 1);

        let buffers =
            desc.iter().map(|desc| desc.make_buffer(device)).collect();

        UniformBufferStorage {
            buffers,
            _indexer: PhantomData::default(),
        }
    }
}

impl<I: ShaderUniformIndex> ResourceDesc<'_, I> {
    #[inline]
    fn name(&self) -> &I {
        match self {
            ResourceDesc::StorageBuffer { name, .. }
            | ResourceDesc::DataBuffer { name, .. }
            | ResourceDesc::EmptyBuffer { name, .. }
            | ResourceDesc::TextureView { name, .. }
            | ResourceDesc::Sampler { name, .. } => name,
        }
    }

    #[inline]
    fn index(&self) -> usize {
        self.name().index() as usize
    }

    #[inline]
    fn buffer_name(&self) -> Option<String> {
        if cfg!(debug_assertions) {
            Some(format!("{:?} Uniform Buffer", self.name()))
        } else {
            None
        }
    }

    #[inline]
    fn entry<'a>(&'a self, buffers: &'a UniformBufferStorage<I>) -> BindGroupEntry<'a> {
        let resource = match self {
            ResourceDesc::DataBuffer { .. }
            | ResourceDesc::StorageBuffer { .. }
            | ResourceDesc::EmptyBuffer { .. } =>
                buffers.buffers[self.index()].as_ref().expect("Resource should be registered as a buffer").as_entire_binding(),
            ResourceDesc::TextureView { view, .. } => BindingResource::TextureView(view),
            ResourceDesc::Sampler { sampler, .. } => BindingResource::Sampler(sampler),
        };

        BindGroupEntry {
            binding: self.index() as u32,
            resource,
        }
    }

    #[inline]
    fn make_buffer(&self, device: &Device) -> Option<Buffer> {
        match self {
            ResourceDesc::StorageBuffer { data, .. } => {
                Some(device.create_buffer_init(&BufferInitDescriptor {
                    label: self.buffer_name().as_deref(),
                    contents: data,
                    usage: BufferUsages::STORAGE,
                }))
            }
            ResourceDesc::DataBuffer { data, .. } => {
                Some(device.create_buffer_init(&BufferInitDescriptor {
                    label: self.buffer_name().as_deref(),
                    contents: data,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }))
            }
            ResourceDesc::EmptyBuffer { size, map, .. } => {
                Some(device.create_buffer(&BufferDescriptor {
                    label: self.buffer_name().as_deref(),
                    size: *size as BufferAddress,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: *map,
                }))
            }
            ResourceDesc::TextureView { .. } | ResourceDesc::Sampler { .. } => None,
        }
    }
}
