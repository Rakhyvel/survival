#[derive(Debug, Copy, Clone)]
pub struct Plane {
    normal: nalgebra_glm::Vec3,
    pub dist: f32,
}

impl Plane {
    pub fn new(normal: nalgebra_glm::Vec3, dist: f32) -> Self {
        Self { normal, dist }
    }

    pub fn from_center_normal(center: nalgebra_glm::Vec3, normal: nalgebra_glm::Vec3) -> Self {
        let normal_normal = normal.normalize();
        Self {
            normal: normal_normal,
            dist: -normal_normal.dot(&center),
        }
    }

    pub fn normal(&self) -> nalgebra_glm::Vec3 {
        self.normal
    }

    pub fn dist(&self) -> f32 {
        self.dist
    }
}
