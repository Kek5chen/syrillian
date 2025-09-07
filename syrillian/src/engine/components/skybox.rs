use crate::World;
use crate::components::Component;
use crate::core::GameObjectId;
use crate::engine::assets::HCubemap;
use nalgebra::{Quaternion, Vector3};

/// Color representation for skybox parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const CYAN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
}

/// Different types of skybox implementations
#[derive(Debug, Clone, PartialEq)]
pub enum SkyboxType {
    /// Cubemap-based skybox with 6 face textures
    Cubemap(HCubemap),
    /// Simple gradient from top to bottom color
    Gradient {
        top_color: Color,
        bottom_color: Color,
    },
    /// Procedural skybox with dynamic parameters
    Procedural {
        cloud_density: f32,
        sun_position: Vector3<f32>,
        time: f32,
    },
    /// Pixel art style skybox with limited color palette
    Pixel {
        palette: Vec<Color>,
        resolution: u32,
        dither_strength: f32,
    },
    /// Physically-based atmospheric simulation
    Realistic {
        sun_position: Vector3<f32>,
        atmosphere_density: f32,
        time_of_day: f32,
    },
}

impl Default for SkyboxType {
    fn default() -> Self {
        SkyboxType::Gradient {
            top_color: Color::BLUE,
            bottom_color: Color::CYAN,
        }
    }
}

/// SkyboxComponent for managing 3D scene backgrounds
pub struct SkyboxComponent {
    parent: GameObjectId,
    skybox_type: SkyboxType,
    intensity: f32,
    rotation: Quaternion<f32>,
    enabled: bool,
}

impl Component for SkyboxComponent {
    fn new(parent: GameObjectId) -> Self {
        SkyboxComponent {
            parent,
            skybox_type: SkyboxType::default(),
            intensity: 1.0,
            rotation: Quaternion::identity(),
            enabled: true,
        }
    }

    fn update(&mut self, _world: &mut World) {
        // Update logic will be implemented when rendering system is integrated
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl SkyboxComponent {
    pub fn set_skybox_type(&mut self, skybox_type: SkyboxType) {
        self.skybox_type = skybox_type;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.max(0.0);
    }

    pub fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        self.rotation = rotation;
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn skybox_type(&self) -> &SkyboxType {
        &self.skybox_type
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn rotation(&self) -> &Quaternion<f32> {
        &self.rotation
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GameObjectId;

    #[test]
    fn test_skybox_component_new_should_succeed() {
        let parent_id = GameObjectId::from_ffi(1);
        let component = SkyboxComponent::new(parent_id);

        assert_eq!(component.parent(), parent_id);
        assert_eq!(component.intensity(), 1.0);
        assert!(component.is_enabled());
        assert_eq!(component.rotation(), &Quaternion::identity());

        // Check default skybox type
        match component.skybox_type() {
            SkyboxType::Gradient {
                top_color,
                bottom_color,
            } => {
                assert_eq!(*top_color, Color::BLUE);
                assert_eq!(*bottom_color, Color::CYAN);
            }
            _ => panic!("Expected default gradient"),
        }
    }

    #[test]
    fn test_skybox_type_default() {
        let default_type = SkyboxType::default();
        match default_type {
            SkyboxType::Gradient {
                top_color,
                bottom_color,
            } => {
                assert_eq!(top_color, Color::BLUE);
                assert_eq!(bottom_color, Color::CYAN);
            }
            _ => panic!("Expected gradient default"),
        }
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::BLUE.b, 1.0);
    }

    #[test]
    fn test_skybox_methods_work() {
        let parent_id = GameObjectId::from_ffi(1);
        let mut component = SkyboxComponent::new(parent_id);

        // Test set_intensity
        component.set_intensity(0.5);
        assert_eq!(component.intensity(), 0.5);

        // Test intensity clamping
        component.set_intensity(-1.0);
        assert_eq!(component.intensity(), 0.0);

        // Test toggle_enabled
        assert!(component.is_enabled());
        component.toggle_enabled();
        assert!(!component.is_enabled());
        component.toggle_enabled();
        assert!(component.is_enabled());

        // Test set_rotation
        let new_rotation = Quaternion::new(
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        );
        component.set_rotation(new_rotation);
        assert_eq!(component.rotation(), &new_rotation);

        // Test set_skybox_type
        let cubemap = HCubemap::FALLBACK_CUBEMAP;
        let new_type = SkyboxType::Cubemap(cubemap);
        component.set_skybox_type(new_type.clone());
        match component.skybox_type() {
            SkyboxType::Cubemap(handle) => {
                assert_eq!(*handle, cubemap);
            }
            _ => panic!("Expected cubemap type"),
        }
    }
}
