#[derive(Default, Copy, Clone)]
pub struct Rectangle {
    pub pos: nalgebra_glm::Vec2,
    pub size: nalgebra_glm::Vec2,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            pos: nalgebra_glm::vec2(x, y),
            size: nalgebra_glm::vec2(w, h),
        }
    }
}
