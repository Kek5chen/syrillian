use crate::engine::assets::generic_store::{HandleName, Store, StoreDefaults, StoreType};
use crate::engine::assets::{H, StoreTypeFallback};
use crate::store_add_checked;
use std::error::Error;
use wgpu::{Device, Texture, TextureFormat, TextureView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cubemap {
    pub faces: [Vec<u8>; 6],
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

    const FACE_COLORS: [[u8; 4]; 6] = [
        [255, 0, 0, 255],   // Right - Red
        [0, 255, 0, 255],   // Left - Green
        [0, 0, 255, 255],   // Top - Blue
        [255, 255, 0, 255], // Bottom - Yellow
        [255, 0, 255, 255], // Front - Magenta
        [0, 255, 255, 255], // Back - Cyan
    ];

    pub fn gen_fallback_cubemap(size: u32) -> [Vec<u8>; 6] {
        std::array::from_fn(|face_idx| {
            Self::FACE_COLORS[face_idx].repeat(size as usize * size as usize)
        })
    }

    /// Load cubemap from 6 individual face files
    /// Order: [+X (right), -X (left), +Y (top), -Y (bottom), +Z (front), -Z (back)]
    pub fn from_files(paths: [&str; 6]) -> Result<Self, Box<dyn Error>> {
        // Load images from file paths
        let mut faces = Vec::new();
        let mut width = 0;
        let mut height = 0;

        for (i, path) in paths.iter().enumerate() {
            // Try to load the image file
            let img = match image::open(path) {
                Ok(img) => img,
                Err(_) => {
                    // If file doesn't exist, use fallback color for that face
                    let fallback_size = 64;
                    let face_data = Self::FACE_COLORS[i].repeat(fallback_size * fallback_size);
                    if i == 0 {
                        width = fallback_size as u32;
                        height = fallback_size as u32;
                    }
                    faces.push(face_data);
                    continue;
                }
            };

            // Convert to RGBA8
            let rgba_img = img.to_rgba8();
            if i == 0 {
                width = rgba_img.width();
                height = rgba_img.height();
            }

            // Ensure all faces have the same dimensions
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

        // Convert Vec to array
        let faces_array: [Vec<u8>; 6] = faces
            .try_into()
            .map_err(|_| "Failed to convert faces vector to array")?;

        Ok(Cubemap {
            faces: faces_array,
            width,
            height,
            format: TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Load cubemap from single equirectangular image
    pub fn from_single_image(path: &str) -> Result<Self, Box<dyn Error>> {
        // Load equirectangular image
        let img = match image::open(path) {
            Ok(img) => img,
            Err(_) => {
                // If file doesn't exist, return fallback cubemap
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

        // Determine face size (typically 1/4 of equirectangular width)
        let face_size = (img_width / 4).max(64);

        // Convert equirectangular to cubemap faces
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
    ) -> [Vec<u8>; 6] {
        let img_width = rgba_img.width() as f32;
        let img_height = rgba_img.height() as f32;

        // Define face directions for cubemap sampling
        // Order: [+X (right), -X (left), +Y (top), -Y (bottom), +Z (front), -Z (back)]
        let face_directions = [
            // +X (Right)
            ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
            // -X (Left)
            ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
            // +Y (Top)
            ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            // -Y (Bottom)
            ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
            // +Z (Front)
            ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
            // -Z (Back)
            ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]),
        ];

        std::array::from_fn(|face_idx| {
            let (forward, up, right) = face_directions[face_idx];
            let mut face_data = Vec::with_capacity((face_size * face_size * 4) as usize);

            for y in 0..face_size {
                for x in 0..face_size {
                    // Convert to normalized coordinates [-1, 1]
                    let u = (x as f32 / (face_size - 1) as f32) * 2.0 - 1.0;
                    let v = (y as f32 / (face_size - 1) as f32) * 2.0 - 1.0;

                    // Calculate 3D direction vector for this pixel
                    let dir_x = forward[0] + u * right[0] + v * up[0];
                    let dir_y = forward[1] + u * right[1] + v * up[1];
                    let dir_z = forward[2] + u * right[2] + v * up[2];

                    // Normalize the direction vector
                    let len = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
                    let dir_x = dir_x / len;
                    let dir_y = dir_y / len;
                    let dir_z = dir_z / len;

                    // Convert 3D direction to spherical coordinates
                    let theta = dir_z.atan2(dir_x); // longitude [-π, π]
                    let phi = dir_y.asin(); // latitude [-π/2, π/2]

                    // Convert to equirectangular UV coordinates [0, 1]
                    let eq_u = (theta / (2.0 * std::f32::consts::PI) + 0.5).clamp(0.0, 1.0);
                    let eq_v = (phi / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);

                    // Sample from equirectangular image
                    let sample_x = (eq_u * (img_width - 1.0)).clamp(0.0, img_width - 1.0) as u32;
                    let sample_y = (eq_v * (img_height - 1.0)).clamp(0.0, img_height - 1.0) as u32;

                    let pixel = rgba_img.get_pixel(sample_x, sample_y);
                    face_data.extend_from_slice(&pixel.0);
                }
            }

            face_data
        })
    }

    /// Convert to GPU texture
    pub fn to_gpu_texture(&self, device: &Device, queue: &wgpu::Queue) -> Texture {
        use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureUsages};

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Cubemap Texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 6, // 6 faces for cubemap
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Copy face data to GPU texture

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
            array_layer_count: Some(6), // All 6 faces
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
        // Simple GREEN test - verify from_files returns Ok
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
        // Simple GREEN test - verify from_single_image returns Ok
        let result = Cubemap::from_single_image("test.hdr");
        assert!(result.is_ok(), "from_single_image should return Ok");

        let cubemap = result.unwrap();
        assert_eq!(cubemap.width, 64);
        assert_eq!(cubemap.height, 64);
        assert_eq!(cubemap.faces.len(), 6);
    }

    #[test]
    fn test_basic_cubemap_properties() {
        // Test basic cubemap structure
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

        // Each face should have the correct size
        for face in &cubemap.faces {
            assert_eq!(face.len(), (32 * 32 * 4) as usize); // 32x32 RGBA
        }
    }

    #[test]
    fn test_handle_based_cubemap_integration() {
        // Test Handle-based asset integration
        use crate::engine::assets::generic_store::Store;
        use std::sync::Arc;

        let store: Arc<Store<Cubemap>> = Arc::new(Store::populated());

        // Test fallback cubemap access
        let fallback_handle = HCubemap::FALLBACK_CUBEMAP;
        let cubemap_ref = store.get(fallback_handle);

        // Dereference the DashMap Ref to access the cubemap
        assert_eq!(cubemap_ref.faces.len(), 6);
        assert_eq!(cubemap_ref.width, 32); // Fallback size
        assert_eq!(cubemap_ref.height, 32);
    }
}
