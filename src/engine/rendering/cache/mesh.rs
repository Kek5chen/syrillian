use crate::engine::assets::Mesh;
use crate::engine::rendering::cache::AssetCache;
use crate::engine::rendering::cache::generic_cache::CacheType;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BufferUsages, Device, Queue};

#[derive(Debug)]
pub struct RuntimeMesh {
    pub(crate) vertices_buf: wgpu::Buffer,
    pub(crate) vertices_num: usize,
    pub(crate) indices_buf: Option<wgpu::Buffer>,
    pub(crate) indices_num: usize,
}

impl RuntimeMesh {
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices_num
    }

    #[inline]
    pub fn indices_count(&self) -> usize {
        self.indices_num
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
