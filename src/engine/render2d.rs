use hecs::{Entity, World};

use super::{
    bvh::BVH,
    render_core::{MeshManager, ModelComponent, OpenGl, TextureId, TextureManager},
};

pub fn render_rectangle(
    rect: Rectangle,
    open_gl: &mut OpenGl,
    mesh_manager: &MeshManager,
    texture_manager: &TextureManager,
    int_screen_resolution: nalgebra_glm::I32Vec2,
) {
    open_gl.set_program();
    unsafe {
        gl::Viewport(0, 0, int_screen_resolution.x, int_screen_resolution.y);
        gl::Disable(gl::DEPTH_TEST); // Disable depth test for 2D rendering
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

    let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
    let model_matrix: nalgebra_glm::Mat4 = nalgebra_glm::scale(
        &nalgebra_glm::translate(
            &nalgebra_glm::one(),
            &nalgebra_glm::vec3(
                1.0 - 2.0 * rect.pos.x / int_screen_resolution.x as f32,
                1.0 - 2.0 * rect.pos.y / int_screen_resolution.y as f32,
                3.0,
            ),
        ),
        &nalgebra_glm::vec3(
            rect.size.x / int_screen_resolution.x as f32,
            rect.size.y / int_screen_resolution.y as f32,
            0.1,
        ),
    );

    let texture = texture_manager
        .get_texture_from_id(rect.texture_id)
        .unwrap();
    let (texture_width, texture_height) = texture.get_dimensions().unwrap();
    texture.activate(gl::TEXTURE0);
    texture.associate_uniform(open_gl.program(), 0, "texture0");
    let u_texture_top_left = open_gl.get_uniform("u_texture_top_left").unwrap();
    unsafe {
        gl::Uniform2f(
            u_texture_top_left.id,
            rect.uv.x / (texture_width as f32),
            rect.uv.y / (texture_height as f32),
        );
    }
    let u_texture_size = open_gl.get_uniform("u_texture_size").unwrap();
    unsafe {
        gl::Uniform2f(
            u_texture_size.id,
            rect.uv_size.x / (texture_width as f32),
            rect.uv_size.y / (texture_height as f32),
        );
    }

    let quad_mesh = mesh_manager
        .get_mesh_from_id(mesh_manager.get_id_from_name("quad-xy").unwrap())
        .unwrap();
    quad_mesh.draw(open_gl, model_matrix, view_matrix, proj_matrix);
}

pub struct Rectangle {
    pub pos: nalgebra_glm::Vec2,
    pub size: nalgebra_glm::Vec2,

    pub texture_id: TextureId,
    pub uv: nalgebra_glm::Vec2,
    pub uv_size: nalgebra_glm::Vec2,
}
