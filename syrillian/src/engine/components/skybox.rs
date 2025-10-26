use crate::{proxy_data_mut, World};
use crate::components::Component;
use crate::core::GameObjectId;
use crate::engine::assets::HCubemap;
use crate::engine::rendering::CPUDrawCtx;
use crate::rendering::proxies::SceneProxy;
use crate::rendering::proxies::SkyboxProxy;
use nalgebra::Quaternion;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkyboxType {
    Cubemap(HCubemap),
}

impl Default for SkyboxType {
    fn default() -> Self {
        SkyboxType::Cubemap(HCubemap::FALLBACK_CUBEMAP)
    }
}

pub struct SkyboxComponent {
    parent: GameObjectId,
    skybox_type: SkyboxType,
    intensity: f32,
    rotation: Quaternion<f32>,
    enabled: bool,
    dirty_skybox: bool,
    was_enabled: bool,
}

impl Component for SkyboxComponent {
    fn new(parent: GameObjectId) -> Self {
        SkyboxComponent {
            parent,
            skybox_type: SkyboxType::default(),
            intensity: 1.0,
            rotation: Quaternion::identity(),
            enabled: true,
            dirty_skybox: false,
            was_enabled: true,
        }
    }

    fn create_render_proxy(&mut self, _world: &World) -> Option<Box<dyn SceneProxy>> {
        let SkyboxType::Cubemap(cubemap_handle) = self.skybox_type;
        Some(Box::new(SkyboxProxy::new(cubemap_handle)))
    }

    fn update_proxy(&mut self, _world: &World, mut ctx: CPUDrawCtx) {
        if self.enabled != self.was_enabled {
          if self.enabled {
              ctx.enable_proxy();
          } else {
              ctx.disable_proxy();
          }
          self.was_enabled = self.enabled;
      }

      if self.dirty_skybox {
          if self.enabled {
              let SkyboxType::Cubemap(cubemap) = self.skybox_type;
              ctx.send_proxy_update(move |sc| {
                  let data: &mut SkyboxProxy = proxy_data_mut!(sc);
                  data.cubemap = cubemap;
              });
          }
          self.dirty_skybox = false;
      }
    }

    fn parent(&self) -> GameObjectId {
        self.parent
    }
}

impl SkyboxComponent {
    pub fn set_skybox_type(&mut self, skybox_type: SkyboxType) {
        self.skybox_type = skybox_type;
        self.dirty_skybox = true;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.max(0.0);
    }

    pub fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        self.rotation = rotation;
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
        self.dirty_skybox = true;
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
        assert!(!component.dirty_skybox);
        assert!(component.was_enabled);

        let SkyboxType::Cubemap(handle) = component.skybox_type();
        assert_eq!(*handle, HCubemap::FALLBACK_CUBEMAP);
    }

    #[test]
    fn test_skybox_methods_work() {
        let parent_id = GameObjectId::from_ffi(1);
        let mut component = SkyboxComponent::new(parent_id);

        component.set_intensity(0.5);
        assert_eq!(component.intensity(), 0.5);

        component.set_intensity(-1.0);
        assert_eq!(component.intensity(), 0.0);

        assert!(component.is_enabled());
        component.toggle_enabled();
        assert!(!component.is_enabled());
        component.toggle_enabled();
        assert!(component.is_enabled());

        let new_rotation = Quaternion::new(
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        );
        component.set_rotation(new_rotation);
        assert_eq!(component.rotation(), &new_rotation);

        let cubemap = HCubemap::FALLBACK_CUBEMAP;
        let new_type = SkyboxType::Cubemap(cubemap);
        component.set_skybox_type(new_type);
        let SkyboxType::Cubemap(handle) = component.skybox_type();
        assert_eq!(*handle, cubemap);
    }

    #[test]
    fn test_dirty_flag_set_on_skybox_type_change() {
        let parent_id = GameObjectId::from_ffi(1);
        let mut component = SkyboxComponent::new(parent_id);
        let cubemap = HCubemap::FALLBACK_CUBEMAP;
        component.set_skybox_type(SkyboxType::Cubemap(cubemap));

        assert!(component.dirty_skybox);
    }

    #[test]
    fn test_enable_disable_functionality() {
        let parent_id = GameObjectId::from_ffi(1);
        let mut component = SkyboxComponent::new(parent_id);
        let cubemap = HCubemap::FALLBACK_CUBEMAP;
        component.set_skybox_type(SkyboxType::Cubemap(cubemap));
        component.toggle_enabled();
        component.toggle_enabled();

        assert!(component.is_enabled());
        assert!(component.dirty_skybox);
    }

    #[test]
    fn test_dirty_flags_with_setters() {
        let parent_id = GameObjectId::from_ffi(1);
        let mut component = SkyboxComponent::new(parent_id);

        assert!(!component.dirty_skybox);

        let cubemap = HCubemap::FALLBACK_CUBEMAP;
        component.set_skybox_type(SkyboxType::Cubemap(cubemap));

        assert!(component.dirty_skybox);

        component.dirty_skybox = false;
        component.toggle_enabled();

        assert!(component.dirty_skybox);
    }
}
