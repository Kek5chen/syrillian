use crate::engine::assets::Mesh;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::cache::generic_cache::CacheType;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BufferUsages, Device, Queue};

#[derive(Debug)]
pub struct RuntimeMesh {
    vertices_buf: wgpu::Buffer,
    vertices_num: usize,
    indices_buf: Option<wgpu::Buffer>,
    indices_num: usize,
}

impl RuntimeMesh {
    pub fn set_vertex_buffer(&mut self, buffer: wgpu::Buffer, vertex_count: usize) {
        self.vertices_buf = buffer;
        self.vertices_num = vertex_count;
    }

    pub fn set_index_buffer(&mut self, buffer: Option<wgpu::Buffer>, indices_count: usize) {
        self.indices_num = if buffer.is_some() { indices_count } else { 0 };
        self.indices_buf = buffer;
    }

    #[inline]
    pub fn vertex_count(&self) -> u32 {
        self.vertices_num as u32
    }

    #[inline]
    pub fn indices_count(&self) -> u32 {
        self.indices_num as u32
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertices_buf
    }

    pub fn indices_buffer(&self) -> Option<&wgpu::Buffer> {
        self.indices_buf.as_ref()
    }
}

impl CacheType for Mesh {
    type Hot = RuntimeMesh;

    fn upload(&self, device: &Device, _queue: &Queue, _cache: &AssetCache) -> Self::Hot {
        let vertices_num = self.vertex_count();
        let indices_num = self.indices_count();

        let vertices_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("3D Object Vertex Buffer"),
            contents: bytemuck::cast_slice(self.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let indices_buf = self.indices().map(|indices| {
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("3D Object Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: BufferUsages::INDEX,
            })
        });

        Self::Hot {
            vertices_buf,
            vertices_num,
            indices_buf,
            indices_num,
        }
    }
}
