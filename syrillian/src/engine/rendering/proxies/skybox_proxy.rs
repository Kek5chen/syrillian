use crate::assets::{AssetStore, HCubemap, HShader};
use crate::rendering::proxies::SceneProxy;
use crate::rendering::uniform::ShaderUniform;
use crate::rendering::{GPUDrawCtx, Renderer};
use crate::{must_pipeline, proxy_data, proxy_data_mut};
use nalgebra::{Matrix4, Vector3};
use std::any::Any;
use syrillian_macros::UniformIndex;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{BindGroup, Buffer, BufferUsages};
use winit::window::Window;

#[repr(u8)]
#[derive(Copy, Clone, Debug, UniformIndex)]
pub enum SkyboxUniformIndex {
    Camera = 0,
}

const SKYBOX_VERTICES: &[Vector3<f32>] = &[
    // Front face
    Vector3::new(-1.0, -1.0, 1.0),
    Vector3::new(1.0, -1.0, 1.0),
    Vector3::new(1.0, 1.0, 1.0),
    Vector3::new(-1.0, -1.0, 1.0),
    Vector3::new(1.0, 1.0, 1.0),
    Vector3::new(-1.0, 1.0, 1.0),
    // Back face
    Vector3::new(1.0, -1.0, -1.0),
    Vector3::new(-1.0, -1.0, -1.0),
    Vector3::new(-1.0, 1.0, -1.0),
    Vector3::new(1.0, -1.0, -1.0),
    Vector3::new(-1.0, 1.0, -1.0),
    Vector3::new(1.0, 1.0, -1.0),
    // Left face
    Vector3::new(-1.0, -1.0, -1.0),
    Vector3::new(-1.0, -1.0, 1.0),
    Vector3::new(-1.0, 1.0, 1.0),
    Vector3::new(-1.0, -1.0, -1.0),
    Vector3::new(-1.0, 1.0, 1.0),
    Vector3::new(-1.0, 1.0, -1.0),
    // Right face
    Vector3::new(1.0, -1.0, 1.0),
    Vector3::new(1.0, -1.0, -1.0),
    Vector3::new(1.0, 1.0, -1.0),
    Vector3::new(1.0, -1.0, 1.0),
    Vector3::new(1.0, 1.0, -1.0),
    Vector3::new(1.0, 1.0, 1.0),
    // Top face
    Vector3::new(-1.0, 1.0, 1.0),
    Vector3::new(1.0, 1.0, 1.0),
    Vector3::new(1.0, 1.0, -1.0),
    Vector3::new(-1.0, 1.0, 1.0),
    Vector3::new(1.0, 1.0, -1.0),
    Vector3::new(-1.0, 1.0, -1.0),
    // Bottom face
    Vector3::new(-1.0, -1.0, -1.0),
    Vector3::new(1.0, -1.0, -1.0),
    Vector3::new(1.0, -1.0, 1.0),
    Vector3::new(-1.0, -1.0, -1.0),
    Vector3::new(1.0, -1.0, 1.0),
    Vector3::new(-1.0, -1.0, 1.0),
];

#[derive(Debug)]
pub struct RuntimeSkyboxData {
    pub cubemap_bind_group: Option<BindGroup>,
    pub uniform: ShaderUniform<SkyboxUniformIndex>,
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
}

#[derive(Debug, Clone)]
pub struct SkyboxProxy {
    pub cubemap: HCubemap,
}

impl SkyboxProxy {
    pub fn new(cubemap: HCubemap) -> Self {
        Self { cubemap }
    }
}

impl SceneProxy for SkyboxProxy {
    fn setup_render(
        &mut self,
        renderer: &Renderer,
        _local_to_world: &Matrix4<f32>,
    ) -> Box<dyn Any> {
        let device = &renderer.state.device;

        let camera_bgl = renderer.cache.bgl_render();
        let uniform = ShaderUniform::<SkyboxUniformIndex>::builder(&camera_bgl).build(device);

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Skybox Vertex Buffer"),
            contents: bytemuck::cast_slice(SKYBOX_VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let cubemap_bind_group = if let Some(cubemap_texture) = renderer.cache.cubemap(self.cubemap)
        {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Skybox Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Skybox Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Skybox Bind Group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&cubemap_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            }))
        } else {
            None
        };

        Box::new(RuntimeSkyboxData {
            cubemap_bind_group,
            uniform,
            vertex_buffer,
            vertex_count: SKYBOX_VERTICES.len() as u32,
        })
    }

    fn update_render(
        &mut self,
        _renderer: &Renderer,
        data: &mut dyn Any,
        _window: &Window,
        _local_to_world: &Matrix4<f32>,
    ) {
        let _data: &mut RuntimeSkyboxData = proxy_data_mut!(data);
    }

    fn render(
        &self,
        renderer: &Renderer,
        data: &dyn Any,
        ctx: &GPUDrawCtx,
        _local_to_world: &Matrix4<f32>,
    ) {
        let data: &RuntimeSkyboxData = proxy_data!(data);

        let Some(ref cubemap_bind_group) = data.cubemap_bind_group else {
            return;
        };

        let shader = renderer.cache.shader(HShader::SKYBOX_CUBEMAP);
        let mut pass = ctx.pass.write().unwrap();

        must_pipeline!(pipeline = shader, ctx.pass_type => return);
        pass.set_pipeline(pipeline);

        pass.set_vertex_buffer(0, data.vertex_buffer.slice(..));

        pass.set_bind_group(0, cubemap_bind_group, &[]);

        pass.set_bind_group(1, data.uniform.bind_group(), &[]);

        pass.draw(0..data.vertex_count, 0..1);
    }

    fn priority(&self, _store: &AssetStore) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::HCubemap;

    #[test]
    fn test_skybox_proxy_creation() {
        let proxy = SkyboxProxy::new(HCubemap::FALLBACK_CUBEMAP);
        assert_eq!(proxy.cubemap, HCubemap::FALLBACK_CUBEMAP);
    }
}
