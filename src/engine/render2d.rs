use std::borrow::Borrow;

use hecs::{Entity, World};

use super::{
    bvh::BVH,
    rectangle::Rectangle,
    render_core::{ModelComponent, RenderContext, TextureId},
};

impl RenderContext {
    pub fn render_rectangle(
        &self,
        dest: Rectangle,
        texture_id: TextureId,
        texture_dest: Rectangle,
    ) {
        let res = self.int_screen_resolution.borrow();
        unsafe {
            gl::Viewport(0, 0, res.x, res.y);
            gl::Disable(gl::DEPTH_TEST); // Disable depth test for 2D rendering
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        }

        let (view_matrix, proj_matrix) = self.camera.borrow().view_proj_matrices();
        let model_matrix: nalgebra_glm::Mat4 = nalgebra_glm::scale(
            &nalgebra_glm::translate(
                &nalgebra_glm::one(),
                &nalgebra_glm::vec3(
                    1.0 - 2.0 * dest.pos.x / res.x as f32,
                    1.0 - 2.0 * dest.pos.y / res.y as f32,
                    3.0,
                ),
            ),
            &nalgebra_glm::vec3(dest.size.x / res.x as f32, dest.size.y / res.y as f32, 0.1),
        );

        let texture = self.get_texture_from_id(texture_id).unwrap();
        let (texture_width, texture_height) = texture.get_dimensions().unwrap();
        texture.activate(gl::TEXTURE0);
        texture.associate_uniform(self.get_current_program_id(), 0, "texture0");
        let u_sprite_offset = self.get_program_uniform("u_sprite_offset").unwrap();
        unsafe {
            gl::Uniform2f(
                u_sprite_offset.id,
                texture_dest.pos.x / texture_width as f32,
                texture_dest.pos.y / texture_width as f32,
            );
        }
        let u_sprite_size = self.get_program_uniform("u_sprite_size").unwrap();
        unsafe {
            gl::Uniform2f(
                u_sprite_size.id,
                texture_dest.size.x / texture_width as f32,
                texture_dest.size.y / texture_height as f32,
            );
        }

        let quad_mesh = self
            .get_mesh_from_id(self.get_mesh_id_from_name("quad-xy").unwrap())
            .unwrap();
        self.draw(quad_mesh.borrow(), model_matrix, view_matrix, proj_matrix);
    }
}
