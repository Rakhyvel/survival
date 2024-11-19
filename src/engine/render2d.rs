use std::borrow::Borrow;

use hecs::{Entity, World};

use super::{
    bvh::BVH,
    rectangle::Rectangle,
    render_core::{ModelComponent, RenderContext, TextureId},
};

pub struct NineSlice {
    pub texture: TextureId,
    pub border: f32,
}

impl RenderContext {
    pub fn render_nine_slice(&self, nine_slice: NineSlice, dest: Rectangle) {
        if dest.size.x < 2.0 * nine_slice.border || dest.size.y < 2.0 * nine_slice.border {
            panic!("Too small! {}", nine_slice.border)
        }

        let (nine_slice_width, nine_slice_height) = self
            .get_texture_from_id(nine_slice.texture)
            .unwrap()
            .get_dimensions()
            .unwrap();

        // TODO: Clean this up!
        // Coordinates for the texture destination
        let spritesheet_coords = [
            // top-left corner
            Rectangle::new(0.0, 0.0, nine_slice.border, nine_slice.border),
            // top edge
            Rectangle::new(
                nine_slice.border,
                0.0,
                nine_slice_width as f32 - 2.0 * nine_slice.border,
                nine_slice.border,
            ),
            // top-right corner
            Rectangle::new(
                nine_slice_width as f32 - nine_slice.border,
                0.0,
                nine_slice.border,
                nine_slice.border,
            ),
            // left edge
            Rectangle::new(
                0.0,
                nine_slice.border,
                nine_slice.border,
                nine_slice_height as f32 - 2.0 * nine_slice.border,
            ),
            // center
            Rectangle::new(
                nine_slice.border,
                nine_slice.border,
                nine_slice_width as f32 - 2.0 * nine_slice.border,
                nine_slice_height as f32 - 2.0 * nine_slice.border,
            ),
            // right edge
            Rectangle::new(
                nine_slice_width as f32 - nine_slice.border,
                nine_slice.border,
                nine_slice.border,
                nine_slice_height as f32 - 2.0 * nine_slice.border,
            ),
            // bottom-left corner
            Rectangle::new(
                0.0,
                nine_slice_height as f32 - nine_slice.border,
                nine_slice.border,
                nine_slice.border,
            ),
            // bottom edge
            Rectangle::new(
                nine_slice.border,
                nine_slice_height as f32 - nine_slice.border,
                nine_slice_width as f32 - 2.0 * nine_slice.border,
                nine_slice.border,
            ),
            // bottom-right corner
            Rectangle::new(
                nine_slice_width as f32 - nine_slice.border,
                nine_slice_height as f32 - nine_slice.border,
                nine_slice.border,
                nine_slice.border,
            ),
        ];

        let dest_coords = [
            // top-left corner
            Rectangle::new(dest.pos.x, dest.pos.y, nine_slice.border, nine_slice.border),
            // top edge
            Rectangle::new(
                dest.pos.x + nine_slice.border,
                dest.pos.y,
                dest.size.x - 2.0 * nine_slice.border,
                nine_slice.border,
            ),
            // top-right corner
            Rectangle::new(
                dest.pos.x + dest.size.x - nine_slice.border,
                dest.pos.y,
                nine_slice.border,
                nine_slice.border,
            ),
            // left edge
            Rectangle::new(
                dest.pos.x,
                dest.pos.y + nine_slice.border,
                nine_slice.border,
                dest.size.y - 2.0 * nine_slice.border,
            ),
            // center
            Rectangle::new(
                dest.pos.x + nine_slice.border,
                dest.pos.y + nine_slice.border,
                dest.size.x - 2.0 * nine_slice.border,
                dest.size.y - 2.0 * nine_slice.border,
            ),
            // right edge
            Rectangle::new(
                dest.pos.x + dest.size.x - nine_slice.border,
                dest.pos.y + nine_slice.border,
                nine_slice.border,
                dest.size.y - 2.0 * nine_slice.border,
            ),
            // bottom-left corner
            Rectangle::new(
                dest.pos.x,
                dest.pos.y + dest.size.y - nine_slice.border,
                nine_slice.border,
                nine_slice.border,
            ),
            // bottom edge
            Rectangle::new(
                dest.pos.x + nine_slice.border,
                dest.pos.y + dest.size.y - nine_slice.border,
                dest.size.x - 2.0 * nine_slice.border,
                nine_slice.border,
            ),
            // bottom-right corner
            Rectangle::new(
                dest.pos.x + dest.size.x - nine_slice.border,
                dest.pos.y + dest.size.y - nine_slice.border,
                nine_slice.border,
                nine_slice.border,
            ),
        ];

        for i in 0..9 {
            let dest = dest_coords[i];
            let texture_dest = spritesheet_coords[i];

            self.copy_texture(dest, nine_slice.texture, texture_dest);
        }
    }

    pub fn draw_text(&self, pos: nalgebra_glm::Vec2, text: &str) {
        let font = self.get_font_from_id(self.font.borrow().unwrap()).unwrap();
        font.draw(pos, text, self);
    }

    // TODO: Rename `copy_texture` or something, implement `fill_rect` with 2d-color.frag shader
    pub fn copy_texture(&self, dest: Rectangle, texture_id: TextureId, texture_dest: Rectangle) {
        let res = self.int_screen_resolution.borrow();
        unsafe {
            gl::Viewport(0, 0, res.x, res.y);
            gl::Disable(gl::DEPTH_TEST); // Disable depth test for 2D rendering
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.set_program_from_id(self.get_program_id_from_name("2d").unwrap());

        let (view_matrix, proj_matrix) = self.camera_2d.view_proj_matrices();
        let model_matrix: nalgebra_glm::Mat4 = nalgebra_glm::scale(
            &nalgebra_glm::translate(
                &nalgebra_glm::one(),
                &nalgebra_glm::vec3(
                    1.0 - 2.0 * dest.pos.x / res.x as f32 - dest.size.x / res.x as f32,
                    1.0 - 2.0 * dest.pos.y / res.y as f32 - dest.size.y / res.y as f32,
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

    pub fn fill_rect(&self, dest: Rectangle) {
        let res = self.int_screen_resolution.borrow();
        unsafe {
            gl::Viewport(0, 0, res.x, res.y);
            gl::Disable(gl::DEPTH_TEST); // Disable depth test for 2D rendering
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.set_program_from_id(self.get_program_id_from_name("2d-solid").unwrap());

        let u_color = self.get_program_uniform("u_color").unwrap();
        unsafe {
            gl::Uniform4f(
                u_color.id,
                self.color.borrow().x,
                self.color.borrow().y,
                self.color.borrow().z,
                self.color.borrow().w,
            );
        }

        let (view_matrix, proj_matrix) = self.camera_2d.view_proj_matrices();
        let model_matrix: nalgebra_glm::Mat4 = nalgebra_glm::scale(
            &nalgebra_glm::translate(
                &nalgebra_glm::one(),
                &nalgebra_glm::vec3(
                    1.0 - 2.0 * dest.pos.x / res.x as f32 - dest.size.x / res.x as f32,
                    1.0 - 2.0 * dest.pos.y / res.y as f32 - dest.size.y / res.y as f32,
                    3.0,
                ),
            ),
            &nalgebra_glm::vec3(dest.size.x / res.x as f32, dest.size.y / res.y as f32, 0.1),
        );

        let quad_mesh = self
            .get_mesh_from_id(self.get_mesh_id_from_name("quad-xy").unwrap())
            .unwrap();
        self.draw(quad_mesh.borrow(), model_matrix, view_matrix, proj_matrix);
    }
}
