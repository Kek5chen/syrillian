use crossbeam_channel::bounded;
use image::{ColorType, ImageFormat};
use snafu::Snafu;
use std::path::Path;
use wgpu::{
    BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, Device, Extent3d, MapMode,
    Origin3d, PollType, Queue, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureFormat,
};

#[derive(Debug, Snafu)]
pub enum TextureExportError {
    #[snafu(display("Unsupported texture format {:?} for export", format))]
    UnsupportedFormat { format: TextureFormat },

    #[snafu(display("Cannot export empty texture: {width}x{height}"))]
    InvalidDimensions { width: u32, height: u32 },

    #[snafu(display("Failed to map export buffer: {source:?}"))]
    Map { source: wgpu::BufferAsyncError },

    #[snafu(display("Failed to map export buffer: channel closed"))]
    MapChannelClosed,

    #[snafu(display("Failed to write image: {source}"))]
    Image { source: image::ImageError },

    #[snafu(display("Export source unavailable: {reason}"))]
    Unavailable { reason: &'static str },
}

fn is_supported(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb
    )
}

/// Reads a texture into an RGBA8 buffer (no gamma conversion) and strips row padding.
pub fn read_texture_rgba(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    format: TextureFormat,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, TextureExportError> {
    if width == 0 || height == 0 {
        return Err(TextureExportError::InvalidDimensions { width, height });
    }

    if !is_supported(format) {
        return Err(TextureExportError::UnsupportedFormat { format });
    }

    let bytes_per_pixel: u32 = 4;
    let bytes_per_row = bytes_per_pixel * width;
    let padded_bytes_per_row =
        bytes_per_row.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;

    let buffer_size = padded_bytes_per_row as u64 * height as u64;

    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Texture Export Buffer"),
        size: buffer_size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture Export Encoder"),
    });

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = bounded(1);
    slice.map_async(MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(PollType::wait_indefinitely());

    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(source)) => return Err(TextureExportError::Map { source }),
        Err(_) => return Err(TextureExportError::MapChannelClosed),
    }

    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);

    for row in 0..height as usize {
        let start = row * padded_bytes_per_row as usize;
        let end = start + bytes_per_row as usize;
        pixels.extend_from_slice(&data[start..end]);
    }

    drop(data);
    buffer.unmap();

    if matches!(
        format,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
    ) {
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
    }

    Ok(pixels)
}

pub fn save_texture_to_png(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    format: TextureFormat,
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> Result<(), TextureExportError> {
    let pixels = read_texture_rgba(device, queue, texture, format, width, height)?;

    image::save_buffer_with_format(
        path,
        &pixels,
        width,
        height,
        ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|source| TextureExportError::Image { source })
}
