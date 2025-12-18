use crate::strobe::UiDrawContext;

pub trait UiElement: Send + Sync + 'static {
    fn draw_order(&self) -> u32;
    fn render(&self, ctx: &mut UiDrawContext);
}
