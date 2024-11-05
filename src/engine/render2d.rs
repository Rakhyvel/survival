use hecs::{Entity, World};

use super::{
    bvh::BVH,
    render_core::{MeshManager, ModelComponent, OpenGl, TextureManager},
};

pub fn render_2d_models_system(
    world: &mut World,
    open_gl: &mut OpenGl,
    mesh_manager: &MeshManager,
    texture_manager: &TextureManager,
    int_screen_resolution: nalgebra_glm::I32Vec2,
) {
    let screen_resolution: nalgebra_glm::Vec2 = int_screen_resolution.cast();

    unsafe {
        gl::Viewport(0, 0, int_screen_resolution.x, int_screen_resolution.x);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

    let camera_frustrum = &open_gl.camera.frustum();

    for (_, model) in &mut world.query::<&ModelComponent>() {
        let mesh = mesh_manager.get_mesh_from_id(model.mesh_id).unwrap();
        let texture = texture_manager
            .get_texture_from_id(model.texture_id)
            .unwrap();
        let model_matrix = model.get_model_matrix();

        texture.activate(gl::TEXTURE0);
        texture.associate_uniform(open_gl.program(), 0, "texture0");

        let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
        mesh.draw(open_gl, model_matrix, view_matrix, proj_matrix);
    }
}
