use crate::World;
use crate::components::{Component, Image, ImageScalingMode, NewComponent, Text2D};
use crate::core::GameObjectId;
use crate::windowing::game_thread::RenderTargetId;
use nalgebra::{Vector2, Vector3};

#[derive(Debug, Clone, Copy)]
pub enum UiSize {
    Pixels { width: f32, height: f32 },
    Percent { width: f32, height: f32 },
}

impl UiSize {
    pub fn resolve(&self, screen: Vector2<f32>) -> Vector2<f32> {
        match *self {
            UiSize::Pixels { width, height } => Vector2::new(width.max(0.0), height.max(0.0)),
            UiSize::Percent { width, height } => {
                Vector2::new((width * screen.x).max(0.0), (height * screen.y).max(0.0))
            }
        }
    }
}

#[derive(Debug)]
pub struct UiRect {
    anchor: Vector2<f32>,
    pivot: Vector2<f32>,
    offset: Vector2<f32>,
    size: UiSize,
    depth: f32,
    render_target: RenderTargetId,
    parent: GameObjectId,
}

impl UiRect {
    pub fn anchor(&self) -> Vector2<f32> {
        self.anchor
    }

    pub fn set_anchor(&mut self, anchor: Vector2<f32>) {
        self.anchor = anchor;
    }

    pub fn pivot(&self) -> Vector2<f32> {
        self.pivot
    }

    pub fn set_pivot(&mut self, pivot: Vector2<f32>) {
        self.pivot = pivot;
    }

    pub fn offset(&self) -> Vector2<f32> {
        self.offset
    }

    pub fn set_offset(&mut self, offset: Vector2<f32>) {
        self.offset = offset;
    }

    pub fn size(&self) -> UiSize {
        self.size
    }

    pub fn set_size(&mut self, size: UiSize) {
        self.size = size;
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }

    pub fn render_target(&self) -> RenderTargetId {
        self.render_target
    }

    pub fn set_render_target(&mut self, target: RenderTargetId) {
        self.render_target = target;
    }

    fn apply_to_components(
        &mut self,
        world: &mut World,
        top_left_px: Vector2<f32>,
        size_px: Vector2<f32>,
    ) {
        self.parent.transform.set_local_position_vec(Vector3::new(
            top_left_px.x,
            top_left_px.y,
            self.depth,
        ));

        if let Some(mut image) = self.parent.get_component::<Image>()
            && let Some(screen) = world.viewport_size(self.render_target)
        {
            let screen_h = screen.height.max(1) as f32;

            let left = top_left_px.x.max(0.0).floor() as u32;
            let right = (top_left_px.x + size_px.x).max(0.0).ceil() as u32;

            let bottom = (screen_h - (top_left_px.y + size_px.y)).max(0.0).floor() as u32;
            let top = (screen_h - top_left_px.y).max(0.0).ceil() as u32;

            if top > bottom && right > left {
                image.set_scaling_mode(ImageScalingMode::Absolute {
                    left,
                    right,
                    top,
                    bottom,
                });
            }
        }

        if let Some(mut text) = self.parent.get_component::<Text2D>() {
            text.set_position_vec(top_left_px);
        }
    }
}

impl NewComponent for UiRect {
    fn new(parent: GameObjectId) -> Self {
        Self {
            parent,
            anchor: Vector2::new(0.0, 0.0),
            pivot: Vector2::new(0.0, 0.0),
            offset: Vector2::zeros(),
            size: UiSize::Pixels {
                width: 100.0,
                height: 100.0,
            },
            depth: 0.0,
            render_target: RenderTargetId::PRIMARY,
        }
    }
}

impl Component for UiRect {
    fn update(&mut self, world: &mut World) {
        if !self.parent.exists() {
            return;
        }

        let Some(screen) = world.viewport_size(self.render_target) else {
            return;
        };

        let screen_vec = Vector2::new(screen.width as f32, screen.height as f32);
        let size_px = self.size.resolve(screen_vec);

        let anchor_px = Vector2::new(self.anchor.x * screen_vec.x, self.anchor.y * screen_vec.y);

        let pivot_offset = Vector2::new(self.pivot.x * size_px.x, self.pivot.y * size_px.y);
        let top_left_px = anchor_px + self.offset - pivot_offset;

        self.apply_to_components(world, top_left_px, size_px);
    }
}
