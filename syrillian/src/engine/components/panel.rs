use crate::World;
use crate::components::{Component, Image, NewComponent, Text2D, UiRect, UiRectLayout};
use crate::core::GameObjectId;
use nalgebra::Vector2;

/// Basic container for 2D UI elements.
#[derive(Debug)]
pub struct Panel {
    parent: GameObjectId,
    padding: Vector2<f32>,
    content_depth_bias: f32,
    draw_step: u32,
    depth_step: f32,
}

impl Panel {
    pub fn set_padding(&mut self, padding: Vector2<f32>) {
        self.padding = padding;
    }

    /// Positive values push children closer to the camera than the panel background.
    pub fn set_content_depth_bias(&mut self, bias: f32) {
        self.content_depth_bias = bias.max(0.01);
    }

    pub fn set_depth_step(&mut self, step: f32) {
        self.depth_step = step.max(0.0);
    }

    fn layout_child(&self, child: GameObjectId, layout: &UiRectLayout, world: &mut World) {
        if let Some(mut rect) = child.get_component::<UiRect>() {
            rect.apply_to_components(world, *layout);
        }
    }
}

impl NewComponent for Panel {
    fn new(parent: GameObjectId) -> Self {
        Panel {
            parent,
            padding: Vector2::new(5.0, 5.0),
            content_depth_bias: 0.01,
            draw_step: 1,
            depth_step: 0.0001,
        }
    }
}

impl Component for Panel {
    fn update(&mut self, world: &mut World) {
        let Some(rect) = self.parent.get_component::<UiRect>() else {
            return;
        };

        let Some(mut container_layout) = rect.layout(world) else {
            return;
        };

        container_layout.top_left_px += self.padding;

        let mut order: u32 = 0;

        // Panel root visual order
        if let Some(mut bg) = self.parent.get_component::<Image>() {
            bg.set_draw_order(order);
            order = order.saturating_add(self.draw_step);
        }
        if let Some(mut text) = self.parent.get_component::<Text2D>() {
            text.set_draw_order(order);
            order = order.saturating_add(self.draw_step);
        }

        self.layout_children(self.parent.children(), &container_layout, &mut order, world);
    }
}

impl Panel {
    fn layout_children(
        &self,
        children: &[GameObjectId],
        parent_layout: &UiRectLayout,
        order: &mut u32,
        world: &mut World,
    ) {
        for &child in children {
            let layout_from_rect = child.get_component::<UiRect>().and_then(|rect| {
                rect.layout_in_region(
                    parent_layout.top_left_px,
                    parent_layout.size_px,
                    parent_layout.screen,
                )
            });

            #[allow(clippy::unnecessary_lazy_evaluations)]
            let mut layout = layout_from_rect.unwrap_or_else(|| UiRectLayout {
                top_left_px: parent_layout.top_left_px,
                size_px: parent_layout.size_px,
                screen: parent_layout.screen,
                target: parent_layout.target,
                depth: parent_layout.depth,
                draw_order: *order,
            });

            layout.draw_order = *order;
            layout.depth =
                parent_layout.depth - self.content_depth_bias - (*order as f32 * self.depth_step);

            self.layout_child(child, &layout, world);

            *order = order.saturating_add(self.draw_step);

            if !child.children().is_empty() {
                self.layout_children(child.children(), &layout, order, world);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{ImageScalingMode, UiSize};
    use crate::engine::rendering::proxies::PROXY_PRIORITY_2D;
    use crate::engine::rendering::proxies::image::ImageSceneProxy;
    use crate::windowing::game_thread::RenderTargetId;
    use nalgebra::Vector2;
    use std::any::Any;
    use winit::dpi::PhysicalSize;

    fn world_with_viewport() -> Box<World> {
        let (mut world, _, _) = World::fresh();
        world.set_viewport_size(RenderTargetId::PRIMARY, PhysicalSize::new(800, 600));
        world
    }

    fn as_image_proxy(
        proxy: &mut Box<dyn crate::rendering::proxies::SceneProxy>,
    ) -> &mut ImageSceneProxy {
        (proxy.as_mut() as &mut dyn Any)
            .downcast_mut::<ImageSceneProxy>()
            .expect("image proxy")
    }

    #[test]
    fn panel_lays_out_children_with_depth_bias() {
        let mut world = world_with_viewport();

        let mut panel = world.new_object("panel");
        world.add_child(panel);

        let mut panel_rect = panel.add_component::<UiRect>();
        panel_rect.set_offset(Vector2::new(5.0, 10.0));
        panel_rect.set_size(UiSize::Pixels {
            width: 200.0,
            height: 100.0,
        });
        panel_rect.set_depth(0.2);

        let mut panel_image = panel.add_component::<Image>();
        let mut panel_text = panel.add_component::<Text2D>();
        let mut panel_comp = panel.add_component::<Panel>();

        let mut child = world.new_object("child");
        let mut child_rect = child.add_component::<UiRect>();
        child_rect.set_anchor(Vector2::new(0.5, 0.5));
        child_rect.set_pivot(Vector2::new(0.5, 0.5));
        child_rect.set_size(UiSize::Percent {
            width: 0.5,
            height: 0.5,
        });
        child_rect.set_offset(Vector2::new(10.0, -5.0));
        let mut child_image = child.add_component::<Image>();

        let mut grandchild = world.new_object("grandchild");
        let _grandchild_rect = grandchild.add_component::<UiRect>();
        let mut grandchild_image = grandchild.add_component::<Image>();

        panel.add_child(child);
        child.add_child(grandchild);

        panel_comp.update(&mut world);

        let mut panel_image_proxy = panel_image
            .create_render_proxy(&world)
            .expect("panel image proxy");
        assert_eq!(as_image_proxy(&mut panel_image_proxy).draw_order, 0);

        let text_priority = panel_text
            .create_render_proxy(&world)
            .expect("text proxy")
            .priority(world.assets.as_ref());
        assert_eq!(text_priority, PROXY_PRIORITY_2D + 1);

        let mut child_proxy_box = child_image
            .create_render_proxy(&world)
            .expect("child image proxy");
        let child_proxy = as_image_proxy(&mut child_proxy_box);
        match child_proxy.scaling {
            ImageScalingMode::Absolute {
                left,
                right,
                top,
                bottom,
            } => {
                assert_eq!((left, right, top, bottom), (70, 170, 565, 515));
            }
            _ => panic!("expected absolute scaling for child"),
        }
        assert_eq!(child_proxy.draw_order, 2);
        assert!((child_proxy.translation[(2, 3)] - 0.1898).abs() < 1e-6);

        let mut grandchild_proxy_box = grandchild_image
            .create_render_proxy(&world)
            .expect("grandchild image proxy");
        let grandchild_proxy = as_image_proxy(&mut grandchild_proxy_box);
        match grandchild_proxy.scaling {
            ImageScalingMode::Absolute {
                left,
                right,
                top,
                bottom,
            } => {
                assert_eq!((left, right, top, bottom), (70, 170, 565, 465));
            }
            _ => panic!("expected absolute scaling for grandchild"),
        }
        assert_eq!(grandchild_proxy.draw_order, 3);
        assert!((grandchild_proxy.translation[(2, 3)] - 0.1795).abs() < 1e-6);
    }
}
