use super::{frustrum::Frustrum, plane::Plane};

#[derive(Debug, Copy, Clone)]
pub enum ProjectionKind {
    Perspective {
        fov: f32,
    },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

impl Default for ProjectionKind {
    fn default() -> Self {
        Self::Perspective { fov: 3.5 }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Camera {
    position: nalgebra_glm::Vec3,
    lookat: nalgebra_glm::Vec3,
    up: nalgebra_glm::Vec3,
    pub projection_kind: ProjectionKind,
    aspect_ratio: f32,

    view_matrix: nalgebra_glm::Mat4,
    proj_matrix: nalgebra_glm::Mat4,
}

impl Camera {
    pub fn new(
        position: nalgebra_glm::Vec3,
        lookat: nalgebra_glm::Vec3,
        up: nalgebra_glm::Vec3,
        projection_kind: ProjectionKind,
    ) -> Self {
        let mut retval = Self {
            position,
            lookat,
            up,
            projection_kind,
            aspect_ratio: 1.0,
            view_matrix: nalgebra_glm::identity(),
            proj_matrix: nalgebra_glm::identity(),
        };
        retval.regen_view_proj_matrices();
        retval
    }

    pub fn view_proj_matrices(&self) -> (nalgebra_glm::Mat4, nalgebra_glm::Mat4) {
        (self.view_matrix, self.proj_matrix)
    }

    pub fn regen_view_proj_matrices(&mut self) {
        let view_matrix = nalgebra_glm::look_at(&self.position, &self.lookat, &self.up);
        let proj_matrix = match self.projection_kind {
            ProjectionKind::Perspective { fov } => {
                // TODO: Take in aspect, though I don't really care!
                nalgebra_glm::perspective(800.0 / 600.0, fov, 0.1, 1000.0)
            }
            ProjectionKind::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => nalgebra_glm::ortho(left, right, bottom, top, near, far),
        };

        self.view_matrix = view_matrix;
        self.proj_matrix = proj_matrix;
    }

    pub fn inv_proj_view(&self) -> nalgebra_glm::Mat4 {
        let proj_view_matrix = self.proj_matrix * self.view_matrix;
        nalgebra_glm::inverse(&proj_view_matrix)
    }

    pub fn inv_proj_and_view(&self) -> (nalgebra_glm::Mat4, nalgebra_glm::Mat4) {
        (
            // TODO: Store these, probably
            nalgebra_glm::inverse(&self.proj_matrix),
            nalgebra_glm::inverse(&self.view_matrix),
        )
    }

    pub fn frustum(&self) -> Frustrum {
        Frustrum::from_inv_proj_view(self.inv_proj_view(), false)
    }

    pub fn set_position(&mut self, position: nalgebra_glm::Vec3) {
        self.position = position;
        self.regen_view_proj_matrices()
    }

    pub fn set_lookat(&mut self, lookat: nalgebra_glm::Vec3) {
        self.lookat = lookat;
        self.regen_view_proj_matrices()
    }

    pub fn position(&self) -> nalgebra_glm::Vec3 {
        self.position
    }

    pub fn lookat(&self) -> nalgebra_glm::Vec3 {
        self.lookat
    }

    pub fn up(&self) -> nalgebra_glm::Vec3 {
        self.up
    }

    fn get_forward_right_up(&self) -> (nalgebra_glm::Vec3, nalgebra_glm::Vec3, nalgebra_glm::Vec3) {
        let forward = (self.lookat - self.position).normalize();
        let right = -forward.cross(&self.up).normalize();
        let up = right.cross(&forward).normalize();
        (forward, right, up)
    }
}
