use hecs::{Entity, World};

use super::{
    aabb::AABB,
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    frustrum::Frustrum,
    objects::{Fbo, Program, Texture},
    render3d::{MeshManager, ModelComponent, OpenGl, TextureManager},
};

#[derive(Default)]
pub struct DirectionalLightSource {
    pub shadow_camera: Camera,
    pub shadow_program: Program,
    pub frame_buffer: Fbo,
    frame_buffer_size: i32,
    pub depth_map: Texture,
    pub light_dir: nalgebra_glm::Vec3,
}

impl DirectionalLightSource {
    pub fn new(
        shadow_camera: Camera,
        shadow_program: Program,
        light_dir: nalgebra_glm::Vec3,
        frame_buffer_size: i32,
    ) -> Self {
        let depth_map = Texture::new();
        depth_map.load_depth_buffer(frame_buffer_size, frame_buffer_size);
        let frame_buffer = Fbo::new();
        frame_buffer.bind();
        depth_map.post_bind();
        Self {
            shadow_camera,
            shadow_program,
            frame_buffer,
            frame_buffer_size,
            depth_map,
            light_dir: light_dir.normalize(),
        }
    }
}

// pub struct ShadowSystem;
// impl<'a> System<'a> for ShadowSystem {
//     type SystemData = (
//         ReadStorage<'a, MeshComponent>,
//         ReadStorage<'a, PositionComponent>,
//         ReadStorage<'a, CastsShadowComponent>,
//         Read<'a, MeshMgrResource>,
//         Read<'a, OpenGlResource>,
//         Write<'a, SunResource>,
//     );

pub fn directional_light_system(
    directional_light: &mut DirectionalLightSource,
    world: &mut World,
    open_gl: &mut OpenGl,
    mesh_manager: &MeshManager,
    texture_manager: &TextureManager,
    bvh: &BVH<Entity>,
) {
    directional_light.frame_buffer.bind();
    unsafe {
        gl::Viewport(
            0,
            0,
            directional_light.frame_buffer_size,
            directional_light.frame_buffer_size,
        );
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::FRONT);
        gl::Clear(gl::DEPTH_BUFFER_BIT)
    }

    // Use a simple depth shader program
    directional_light.shadow_program.set();

    // Compute the camera frustrum corners
    let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
    let inv_proj_view = nalgebra_glm::inverse(&(proj_matrix * view_matrix));
    let screen_frustrum = Frustrum::from_inv_proj_view(inv_proj_view, false);

    // Transform the screen-world-frustrum corners to light-view-space (1st time)
    // Move shadow camera to world-space origin (kinda arbitrary)
    directional_light
        .shadow_camera
        .set_position(nalgebra_glm::zero());
    // Have it point along the world-space light direction
    directional_light
        .shadow_camera
        .set_lookat(directional_light.shadow_camera.position() - directional_light.light_dir);
    // Calculate the view and proj matrices for this
    let (light_view_matrix, light_proj_view_matrix) =
        directional_light.shadow_camera.view_proj_matrices();
    // Transform the world-space screen frustum into light-view-space
    let light_view_frustrum = screen_frustrum.transform(light_view_matrix);

    // Calculate an AABB for the light-view-space frustrum
    let aabb_light_space = AABB::from_points(light_view_frustrum.corners());

    // Calculate a light-space AABB for the world
    // let mut world_aabb_light_space = AABB::new();
    // world_aabb_light_space.transform(light_view_matrix);
    // aabb_light_space.intersect_z(&world_aabb_light_space);

    // Calculate the mid-point of the near-plane on the light-view-frustrum
    let light_pos_light_space = aabb_light_space.pos_z_plane_midpoint();
    let light_pos_world_space = (nalgebra_glm::inverse(&light_view_matrix)) * light_pos_light_space;

    // Transform the screen-world-frustrum to light-space (2nd time)
    directional_light
        .shadow_camera
        .set_position(light_pos_world_space.xyz());
    directional_light
        .shadow_camera
        .set_lookat(directional_light.shadow_camera.position() - directional_light.light_dir);
    let (light_view_matrix, light_proj_matrix) =
        directional_light.shadow_camera.view_proj_matrices();
    let light_view_frustrum = screen_frustrum.transform(light_view_matrix);

    // Create an Orthographic Projection around the light-space AABB
    let aabb_light_space = AABB::from_points(light_view_frustrum.corners());
    directional_light.shadow_camera.projection_kind = ProjectionKind::Orthographic {
        left: aabb_light_space.min.x,
        right: aabb_light_space.max.x,
        bottom: aabb_light_space.min.y,
        top: aabb_light_space.max.y,
        near: aabb_light_space.min.z,
        far: 800.0,
    };

    let frustum2 =
        Frustrum::from_inv_proj_view(directional_light.shadow_camera.inv_proj_view(), false);

    let mut rendered = 0;
    for model_id in bvh.iter_frustrum(&frustum2, false) {
        rendered += 1;
        let model = world.get::<&ModelComponent>(model_id).unwrap();
        let mesh = mesh_manager.get_mesh_from_id(model.mesh_id).unwrap();
        let texture = texture_manager
            .get_texture_from_id(model.texture_id)
            .unwrap();
        let model_matrix = model.get_model_matrix();

        texture.activate(gl::TEXTURE0);
        texture.associate_uniform(open_gl.program(), 0, "texture0");
        mesh.draw(open_gl, model_matrix, light_view_matrix, light_proj_matrix)
    }
    // println!("rendered: {}", rendered);

    directional_light.frame_buffer.unbind();
}
