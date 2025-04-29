use std::error::Error;
use std::rc::Rc;
use wgpu::{
    Adapter, CompositeAlphaMode, Device, DeviceDescriptor, Extent3d, Features, Instance, Limits, MemoryHints, PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[allow(unused)]
pub struct State {
    pub(crate) instance: Instance,
    pub(crate) surface: Surface<'static>,
    pub(crate) device: Rc<Device>,
    pub(crate) queue: Rc<Queue>,
    pub(crate) config: SurfaceConfiguration,
    pub(crate) size: PhysicalSize<u32>,
    pub(crate) depth_texture: Texture,
}

impl State {
    fn setup_instance() -> Instance {
        Instance::default()
    }

    fn setup_surface(instance: &Instance, window: &Window) -> Result<Surface<'static>, Box<dyn Error>> {
        unsafe {
            // We are creating a 'static lifetime out of a local reference
            // VERY UNSAFE: Make absolutely sure `window` lives as long as `surface`
            let surface = instance.create_surface(window)?;
            Ok(std::mem::transmute::<Surface, Surface<'static>>(surface))
        }
    }

    async fn setup_adapter(instance: &Instance, surface: &Surface<'_>) -> Adapter {
        instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                ..RequestAdapterOptions::default()
            })
            .await
            .expect(
                "Couldn't find anything that supports rendering stuff. How are you reading this..?",
            )
    }

    async fn get_device_and_queue(adapter: &Adapter) -> Result<(Rc<Device>, Rc<Queue>), Box<dyn Error>> {
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Renderer Hardware"),
                    required_features: Features::default() 
                        | Features::POLYGON_MODE_LINE
                        | Features::BUFFER_BINDING_ARRAY,
                    required_limits: Limits {
                        max_bind_groups: 5,
                        ..Limits::default()
                    },
                    memory_hints: MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await?;
        Ok((Rc::new(device), Rc::new(queue)))
    }

    fn configure_surface(
        size: &PhysicalSize<u32>,
        surface: &Surface,
        adapter: &Adapter,
        device: &Device,
    ) -> Result<SurfaceConfiguration, Box<dyn Error>> {
        let caps = surface.get_capabilities(adapter);

        let present_mode = if caps.present_modes.contains(&PresentMode::Mailbox) {
            PresentMode::Mailbox
        } else if caps.present_modes.contains(&PresentMode::Immediate) {
            PresentMode::Immediate
        } else {
            caps.present_modes.first().cloned().unwrap_or(PresentMode::Fifo)
        };

        if !caps.formats.contains(&TextureFormat::Bgra8UnormSrgb) {
            return Err(format!("Can only run on Bgra8UnormSrgb currently, but it's not supported by your GPU. Available: {:?}", caps.formats).into());
        }

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(device, &config);
        Ok(config)
    }

    fn setup_depth_texture(size: &PhysicalSize<u32>, device: &Device) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth Texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[TextureFormat::Depth32Float],
        })
    }

    pub async fn new(window: &Window) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let size = PhysicalSize {
            height: size.height.max(1),
            width: size.width.max(1),
        };

        let instance = Self::setup_instance();
        let surface = Self::setup_surface(&instance, window)?;
        let adapter = Self::setup_adapter(&instance, &surface).await;
        let (device, queue) = Self::get_device_and_queue(&adapter).await?;
        let config = Self::configure_surface(&size, &surface, &adapter, &device)?;

        let depth_texture = Self::setup_depth_texture(&size, &device);

        Ok(State {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
        })
    }

    pub fn resize(&mut self, mut new_size: PhysicalSize<u32>) {
        new_size.height = new_size.height.max(1);
        new_size.width = new_size.width.max(1);
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.recreate_surface();
    }

    pub fn recreate_surface(&mut self) {
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = Self::setup_depth_texture(&self.size, &self.device);
    }

    pub fn update(&mut self) {
        // TODO
    }
}
