use std::collections::HashMap;

use crate::App;

use super::{
    aabb::AABB,
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    frustrum::Frustrum,
    objects::*,
    physics::PositionComponent,
    render_core::{MeshManager, ModelComponent, OpenGl, TextureManager},
    shadow_map::DirectionalLightSource,
    text,
};

use gl::types::GLuint;
use hecs::{Entity, World};
use obj::{load_obj, Obj, TexturedVertex};

pub fn render_3d_models_system(
    world: &mut World,
    open_gl: &mut OpenGl,
    directional_light: &DirectionalLightSource,
    mesh_manager: &MeshManager,
    texture_manager: &TextureManager,
    bvh: &BVH<Entity>,
    int_screen_resolution: nalgebra_glm::I32Vec2,
    debug: bool,
) {
    let screen_resolution: nalgebra_glm::Vec2 = int_screen_resolution.cast();
    open_gl.set_shader_uniforms(
        directional_light.light_dir,
        nalgebra_glm::vec2(screen_resolution.x, screen_resolution.x),
    );

    unsafe {
        gl::Viewport(0, 0, int_screen_resolution.x, int_screen_resolution.x);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::Enable(gl::DEPTH_TEST);
        gl::StencilOp(gl::KEEP, gl::REPLACE, gl::REPLACE);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
    }

    let (light_view_matrix, light_proj_matrix) =
        directional_light.shadow_camera.view_proj_matrices();
    let u_light_matrix = Uniform::new(open_gl.program(), "light_mvp").unwrap();
    let light_proj_view = light_proj_matrix * light_view_matrix;
    unsafe {
        gl::UniformMatrix4fv(
            u_light_matrix.id,
            1,
            gl::FALSE,
            &light_proj_view.columns(0, 4)[0],
        );
    }

    let camera_frustrum = &open_gl.camera.frustum();
    let mut rendered = 0;

    for model_id in bvh.iter_frustrum(camera_frustrum, debug) {
        rendered += 1;
        let mut model = world.get::<&mut ModelComponent>(model_id).unwrap();
        let mesh = mesh_manager.get_mesh_from_id(model.mesh_id).unwrap();
        let texture = texture_manager
            .get_texture_from_id(model.texture_id)
            .unwrap();
        let model_matrix = model.get_model_matrix();

        if model.outlined {
            unsafe {
                gl::StencilFunc(gl::ALWAYS, 1, 0xFF);
                gl::StencilMask(0xFF);
            }
        } else {
            unsafe {
                gl::StencilMask(0x00);
            }
        }

        texture.activate(gl::TEXTURE0);
        texture.associate_uniform(open_gl.program(), 0, "texture0");

        directional_light.depth_map.activate(gl::TEXTURE1);
        directional_light
            .depth_map
            .associate_uniform(open_gl.program(), 1, "shadow_map");

        let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
        mesh.draw(open_gl, model_matrix, view_matrix, proj_matrix);
    }
    // println!("{:?}", rendered);
}

pub fn render_3d_outlines_system(
    world: &mut World,
    open_gl: &mut OpenGl,
    outline_program: &Program,
    mesh_manager: &MeshManager,
    bvh: &BVH<Entity>,
) {
    unsafe {
        gl::StencilFunc(gl::NOTEQUAL, 1, 0xFF);
        gl::StencilMask(0x00);
        gl::Enable(gl::STENCIL_TEST);
        gl::Disable(gl::DEPTH_TEST);
    }
    outline_program.set();
    let camera_frustrum = &open_gl.camera.frustum();

    for model_id in bvh.iter_frustrum(camera_frustrum, false) {
        let mut model = world.get::<&mut ModelComponent>(model_id).unwrap();
        if !model.outlined {
            continue;
        }
        let mesh = mesh_manager.get_mesh_from_id(model.mesh_id).unwrap();

        let old_scale = model.get_scale();
        model.set_scale(old_scale * 1.2);
        let model_matrix = model.get_model_matrix();
        model.set_scale(old_scale);

        let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
        mesh.draw(open_gl, model_matrix, view_matrix, proj_matrix);
    }

    unsafe {
        gl::StencilMask(0xFF);
        gl::Enable(gl::STENCIL_TEST);
        gl::StencilFunc(gl::ALWAYS, 1, 0xFF);
    }
}
