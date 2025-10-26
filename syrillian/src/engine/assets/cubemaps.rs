use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, StoreTypeFallback};
use crate::store_add_checked;
use std::error::Error;
use wgpu::{Device, Texture, TextureFormat, TextureView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubemapFaces {
    pub positive_x: Vec<u8>,
    pub negative_x: Vec<u8>,
    pub positive_y: Vec<u8>,
    pub negative_y: Vec<u8>,
    pub positive_z: Vec<u8>,
    pub negative_z: Vec<u8>,
}

impl CubemapFaces {
    pub fn new(
        positive_x: Vec<u8>,
        negative_x: Vec<u8>,
        positive_y: Vec<u8>,
        negative_y: Vec<u8>,
        positive_z: Vec<u8>,
        negative_z: Vec<u8>,
    ) -> Self {
        Self {
            positive_x,
            negative_x,
            positive_y,
            negative_y,
            positive_z,
            negative_z,
        }
    }

    fn from_colors(colors: [FaceColor; 6], size: u32) -> Self {
        let pixel_count = size as usize * size as usize;
        Self::new(
            Into::<[u8; 4]>::into(colors[0]).repeat(pixel_count),
            Into::<[u8; 4]>::into(colors[1]).repeat(pixel_count),
            Into::<[u8; 4]>::into(colors[2]).repeat(pixel_count),
            Into::<[u8; 4]>::into(colors[3]).repeat(pixel_count),
            Into::<[u8; 4]>::into(colors[4]).repeat(pixel_count),
            Into::<[u8; 4]>::into(colors[5]).repeat(pixel_count),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<u8>> {
        [
            &self.positive_x,
            &self.negative_x,
            &self.positive_y,
            &self.negative_y,
            &self.positive_z,
            &self.negative_z,
        ]
        .into_iter()
    }

    pub fn len(&self) -> usize {
        6
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FaceColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl FaceColor {
    const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    #[allow(dead_code)]
    const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<FaceColor> for [u8; 4] {
    #[inline]
    fn from(color: FaceColor) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceDirection {
    forward: [f32; 3],
    up: [f32; 3],
    right: [f32; 3],
}

impl FaceDirection {
    const fn new(forward: [f32; 3], up: [f32; 3], right: [f32; 3]) -> Self {
        Self { forward, up, right }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cubemap {
    pub faces: CubemapFaces,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

pub type HCubemap = H<Cubemap>;

impl H<Cubemap> {
    const FALLBACK_CUBEMAP_ID: u32 = 0;
    const MAX_BUILTIN_ID: u32 = 0;

    pub const FALLBACK_CUBEMAP: H<Cubemap> = H::new(Self::FALLBACK_CUBEMAP_ID);
}

impl Cubemap {
    const BYTES_PER_PIXEL: u32 = 4; // RGBA

    const FACE_COLORS: [FaceColor; 6] = [
        FaceColor::new(255, 0, 0, 255),   // Right - Red
        FaceColor::new(0, 255, 0, 255),   // Left - Green
        FaceColor::new(0, 0, 255, 255),   // Top - Blue
        FaceColor::new(255, 255, 0, 255), // Bottom - Yellow
        FaceColor::new(255, 0, 255, 255), // Front - Magenta
        FaceColor::new(0, 255, 255, 255), // Back - Cyan
    ];

    pub fn gen_fallback_cubemap(size: u32) -> CubemapFaces {
        CubemapFaces::from_colors(Self::FACE_COLORS, size)
    }

    // Load cubemap from 6 individual face files
    pub fn from_files(paths: [&str; 6]) -> Result<Self, Box<dyn Error>> {
        let mut faces = Vec::new();
        let mut width = 0;
        let mut height = 0;

        for (i, path) in paths.iter().enumerate() {
            let img = match image::open(path) {
                Ok(img) => img,
                Err(_) => {
                    let fallback_size = 64;
                    let face_data = Into::<[u8; 4]>::into(Self::FACE_COLORS[i]).repeat(fallback_size * fallback_size);
                    if i == 0 {
                        width = fallback_size as u32;
                        height = fallback_size as u32;
                    }
                    faces.push(face_data);
                    continue;
                }
            };

            let rgba_img = img.to_rgba8();
            if i == 0 {
                width = rgba_img.width();
                height = rgba_img.height();
            }

            let resized_img = if rgba_img.width() != width || rgba_img.height() != height {
                image::imageops::resize(
                    &rgba_img,
                    width,
                    height,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                rgba_img
            };

            faces.push(resized_img.into_raw());
        }

        let faces_array: [Vec<u8>; 6] = faces
            .try_into()
            .map_err(|_| "Failed to convert faces vector to array")?;

        Ok(Cubemap {
            faces: CubemapFaces::new(
                faces_array[0].clone(),
                faces_array[1].clone(),
                faces_array[2].clone(),
                faces_array[3].clone(),
                faces_array[4].clone(),
                faces_array[5].clone(),
            ),
            width,
            height,
            format: TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Load cubemap from single equirectangular image
    pub fn from_single_image(path: &str) -> Result<Self, Box<dyn Error>> {
        let img = match image::open(path) {
            Ok(img) => img,
            Err(_) => {
                return Ok(Cubemap {
                    faces: Self::gen_fallback_cubemap(64),
                    width: 64,
                    height: 64,
                    format: TextureFormat::Rgba8UnormSrgb,
                });
            }
        };

        let rgba_img = img.to_rgba8();
        let img_width = rgba_img.width();
        let _img_height = rgba_img.height();

        let face_size = (img_width / 4).max(64);
        let faces = Self::equirectangular_to_cubemap(&rgba_img, face_size);

        Ok(Cubemap {
            faces,
            width: face_size,
            height: face_size,
            format: TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Convert equirectangular image to cubemap faces
    fn equirectangular_to_cubemap(
        rgba_img: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        face_size: u32,
    ) -> CubemapFaces {
        let img_width = rgba_img.width() as f32;
        let img_height = rgba_img.height() as f32;

        let face_directions = [
            // +X (Right)
            FaceDirection::new([1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
            // -X (Left)
            FaceDirection::new([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
            // +Y (Top)
            FaceDirection::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            // -Y (Bottom)
            FaceDirection::new([0.0, -1.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
            // +Z (Front)
            FaceDirection::new([0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
            // -Z (Back)
            FaceDirection::new([0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]),
        ];

        let faces_array: [Vec<u8>; 6] = std::array::from_fn(|face_idx| {
            let direction = &face_directions[face_idx];
            let mut face_data = Vec::with_capacity((face_size * face_size * 4) as usize);

            for y in 0..face_size {
                for x in 0..face_size {
                    let u = (x as f32 / (face_size - 1) as f32) * 2.0 - 1.0;
                    let v = (y as f32 / (face_size - 1) as f32) * 2.0 - 1.0;

                    let dir_x = direction.forward[0] + u * direction.right[0] + v * direction.up[0];
                    let dir_y = direction.forward[1] + u * direction.right[1] + v * direction.up[1];
                    let dir_z = direction.forward[2] + u * direction.right[2] + v * direction.up[2];

                    let len = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
                    let dir_x = dir_x / len;
                    let dir_y = dir_y / len;
                    let dir_z = dir_z / len;

                    let theta = dir_z.atan2(dir_x);
                    let phi = dir_y.asin();

                    let eq_u = (theta / (2.0 * std::f32::consts::PI) + 0.5).clamp(0.0, 1.0);
                    let eq_v = (phi / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);

                    let sample_x = (eq_u * (img_width - 1.0)).clamp(0.0, img_width - 1.0) as u32;
                    let sample_y = (eq_v * (img_height - 1.0)).clamp(0.0, img_height - 1.0) as u32;

                    let pixel = rgba_img.get_pixel(sample_x, sample_y);
                    face_data.extend_from_slice(&pixel.0);
                }
            }

            face_data
        });

        CubemapFaces::new(
            faces_array[0].clone(),
            faces_array[1].clone(),
            faces_array[2].clone(),
            faces_array[3].clone(),
            faces_array[4].clone(),
            faces_array[5].clone(),
        )
    }

    /// Convert to GPU texture
    pub fn to_gpu_texture(&self, device: &Device, queue: &wgpu::Queue) -> Texture {
        use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureUsages};

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Cubemap Texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (face_idx, face_data) in self.faces.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: face_idx as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                face_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * Self::BYTES_PER_PIXEL),
                    rows_per_image: Some(self.height),
                },
                Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        texture
    }

    /// Create texture view for GPU usage
    pub fn create_view(&self, texture: &Texture) -> TextureView {
        use wgpu::{TextureViewDescriptor, TextureViewDimension};

        texture.create_view(&TextureViewDescriptor {
            label: Some("Cubemap Texture View"),
            format: Some(self.format),
            dimension: Some(TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
            usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
        })
    }
}

impl StoreDefaults for Cubemap {
    fn populate(store: &mut Store<Self>) {
        const FALLBACK_SIZE: u32 = 32;
        store_add_checked!(
            store,
            HCubemap::FALLBACK_CUBEMAP_ID,
            Cubemap {
                faces: Self::gen_fallback_cubemap(FALLBACK_SIZE),
                width: FALLBACK_SIZE,
                height: FALLBACK_SIZE,
                format: TextureFormat::Bgra8UnormSrgb
            }
        );
    }
}

impl StoreType for Cubemap {
    #[inline]
    fn name() -> &'static str {
        "Cubemap"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        match handle.id() {
            HCubemap::FALLBACK_CUBEMAP_ID => HandleName::Static("Cubemap Fallback"),
            _ => HandleName::Id(handle),
        }
    }

    fn is_builtin(handle: H<Self>) -> bool {
        handle.id() == H::<Self>::MAX_BUILTIN_ID
    }
}

impl StoreTypeFallback for Cubemap {
    fn fallback() -> H<Self> {
        HCubemap::FALLBACK_CUBEMAP
    }
}

impl Store<Cubemap> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_files_returns_ok() {
        let paths = [
            "test1.png",
            "test2.png",
            "test3.png",
            "test4.png",
            "test5.png",
            "test6.png",
        ];

        let result = Cubemap::from_files(paths);
        assert!(result.is_ok(), "from_files should return Ok");

        let cubemap = result.unwrap();
        assert_eq!(cubemap.width, 64);
        assert_eq!(cubemap.height, 64);
        assert_eq!(cubemap.faces.len(), 6);
    }

    #[test]
    fn test_from_single_image_returns_ok() {
        let result = Cubemap::from_single_image("test.hdr");
        assert!(result.is_ok(), "from_single_image should return Ok");

        let cubemap = result.unwrap();
        assert_eq!(cubemap.width, 64);
        assert_eq!(cubemap.height, 64);
        assert_eq!(cubemap.faces.len(), 6);
    }

    #[test]
    fn test_basic_cubemap_properties() {
        let cubemap = Cubemap {
            faces: Cubemap::gen_fallback_cubemap(32),
            width: 32,
            height: 32,
            format: TextureFormat::Rgba8UnormSrgb,
        };

        assert_eq!(cubemap.width, 32);
        assert_eq!(cubemap.height, 32);
        assert_eq!(cubemap.faces.len(), 6);
        assert_eq!(cubemap.format, TextureFormat::Rgba8UnormSrgb);

        for face in cubemap.faces.iter() {
            assert_eq!(face.len(), (32 * 32 * 4) as usize);
        }
    }

    #[test]
    fn test_handle_based_cubemap_integration() {
        use crate::engine::assets::generic_store::Store;
        use std::sync::Arc;

        let store: Arc<Store<Cubemap>> = Arc::new(Store::populated());

        let fallback_handle = HCubemap::FALLBACK_CUBEMAP;
        let cubemap_ref = store.get(fallback_handle);

        assert_eq!(cubemap_ref.faces.len(), 6);
        assert_eq!(cubemap_ref.width, 32);
        assert_eq!(cubemap_ref.height, 32);
    }
}
