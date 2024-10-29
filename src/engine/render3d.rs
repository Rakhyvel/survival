use crate::App;

use super::{
    aabb::AABB,
    bvh::BVH,
    camera::{Camera, ProjectionKind},
    frustrum::Frustrum,
    objects::*,
    physics::PositionComponent,
    shadow_map::DirectionalLightSource,
};

use gl::types::GLuint;
use hecs::{Entity, World};
use obj::{load_obj, Obj, TexturedVertex};

pub fn render_3d_models_system(
    world: &mut World,
    open_gl: &mut OpenGl,
    directional_light: &DirectionalLightSource,
    mesh_manager: &MeshManager,
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
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }

    let (light_view_matrix, light_proj_matrix) =
        directional_light.shadow_camera.view_proj_matrices();
    let u_light_matrix = Uniform::new(open_gl.program.id(), "light_mvp").unwrap();
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
        let model = world.get::<&ModelComponent>(model_id).unwrap();
        let mesh = mesh_manager.get_mesh(model.mesh_id).unwrap();
        let model_matrix = model.get_model_matrix();

        model.texture.activate(gl::TEXTURE0);
        model
            .texture
            .associate_uniform(open_gl.program.id(), 0, "texture0");

        directional_light.depth_map.activate(gl::TEXTURE1);
        directional_light
            .depth_map
            .associate_uniform(open_gl.program.id(), 1, "shadow_map");

        // let light_space_mvp = light_proj_view;

        let (view_matrix, proj_matrix) = open_gl.camera.view_proj_matrices();
        mesh.draw(open_gl, model_matrix, view_matrix, proj_matrix);
    }
    // println!("{:?}", rendered);
}

/// An actual model, with geometry, a position, scale, rotation, and texture.
pub struct ModelComponent {
    pub mesh_id: MeshId,
    position: nalgebra_glm::Vec3,
    scale: nalgebra_glm::Vec3,
    model_matrix: nalgebra_glm::Mat4,
    pub texture: Texture,
}

/// Contains a collection of meshes, and associates them with a MeshId.
pub struct MeshManager {
    meshes: Vec<Mesh>,
}

/// Opaque type used by a MeshManager to associate meshes.
#[derive(Copy, Clone)]
pub struct MeshId(usize);

/// Stores the geometry of a mesh. Meshes are registered in the MeshManager, and can be potentially shared across
/// multiple models.
pub struct Mesh {
    pub geometry: Vec<GeometryData>,
    indices: Vec<u32>,
    pub aabb: AABB,
}

/// Encapsulates stuff needed for rendering using opengl, including the camera and the shader program.
pub struct OpenGl {
    pub camera: Camera,
    program: Program,
}

/// Actual geometry data for a mesh.
pub struct GeometryData {
    ibo: Buffer<u32>,
    vbo: Buffer<f32>,
    vao: Vao,
    pub vertex_data: Vec<f32>,
}

enum GeometryDataIndex {
    Vertex = 0,
    Normal = 1,
    Texture = 2,
    Color = 3,
}

impl MeshManager {
    pub fn new() -> Self {
        Self { meshes: vec![] }
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshId {
        let id = self.meshes.len();
        self.meshes.push(mesh);
        MeshId::new(id)
    }

    pub fn get_mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(id.as_usize())
    }
}

impl MeshId {
    pub fn new(id: usize) -> Self {
        MeshId(id)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl ModelComponent {
    pub fn new(
        mesh_id: MeshId,
        position: nalgebra_glm::Vec3,
        scale: nalgebra_glm::Vec3,
        texture: Texture,
    ) -> Self {
        Self {
            mesh_id,
            position,
            scale,
            texture,
            model_matrix: Self::construct_model_matrix(&position, &scale),
        }
    }

    pub fn set_position(&mut self, position: nalgebra_glm::Vec3) {
        self.position = position;
        self.regen_model_matrix();
    }

    pub fn set_scale(&mut self, scale: nalgebra_glm::Vec3) {
        self.scale = scale;
        self.regen_model_matrix();
    }

    pub fn get_model_matrix(&self) -> nalgebra_glm::Mat4 {
        self.model_matrix
    }

    pub fn get_aabb(&self, mesh_manager: &MeshManager) -> AABB {
        mesh_manager
            .get_mesh(self.mesh_id)
            .unwrap()
            .aabb
            .scale(self.scale)
            .translate(self.position)
    }

    fn regen_model_matrix(&mut self) {
        self.model_matrix = Self::construct_model_matrix(&self.position, &self.scale);
    }

    fn construct_model_matrix(
        position: &nalgebra_glm::Vec3,
        scale: &nalgebra_glm::Vec3,
    ) -> nalgebra_glm::Mat4 {
        nalgebra_glm::scale(
            &nalgebra_glm::translate(&nalgebra_glm::one(), position),
            scale,
        )
    }
}

impl Mesh {
    pub fn new(indices: Vec<u32>, datas: Vec<&Vec<f32>>) -> Self {
        let geometry: Vec<GeometryData> = datas
            .iter()
            .map(|data| GeometryData {
                ibo: Buffer::<u32>::gen(gl::ELEMENT_ARRAY_BUFFER),
                vao: Vao::gen(),
                vbo: Buffer::<f32>::gen(gl::ARRAY_BUFFER),
                vertex_data: data.to_vec(),
            })
            .collect();

        for i in 0..geometry.len() {
            geometry[i].vbo.set_data(&geometry[i].vertex_data);
            geometry[i].ibo.set_data(&indices);
            geometry[i].vao.set(i as u32)
        }

        let aabb = AABB::from_points(
            geometry[GeometryDataIndex::Vertex as usize]
                .vertex_data
                .chunks(3)
                .map(|p| nalgebra_glm::vec3(p[0], p[1], p[2])),
        );

        Mesh {
            geometry,
            indices,
            aabb,
        }
    }

    pub fn from_obj(obj_file_data: &[u8]) -> Self {
        let obj: Obj<TexturedVertex> = load_obj(&obj_file_data[..]).unwrap();
        let vb: Vec<TexturedVertex> = obj.vertices;

        let indices = vec_u32_from_vec_u16(&obj.indices);
        let vertices = flatten_positions(&vb);
        let normals = flatten_normals(&vb);
        let uv = flatten_uv(&vb);

        let data = vec![&vertices, &normals, &uv];

        Self::new(indices, data)
    }

    pub fn draw(
        &self,
        open_gl: &OpenGl,
        model_matrix: nalgebra_glm::Mat4,
        view_matrix: nalgebra_glm::Mat4,
        proj_matrix: nalgebra_glm::Mat4,
    ) {
        let u_model_matrix = open_gl.get_uniform("u_model_matrix").unwrap();
        let u_view_matrix = open_gl.get_uniform("u_view_matrix").unwrap();
        let u_proj_matrix = open_gl.get_uniform("u_proj_matrix").unwrap();
        unsafe {
            gl::UniformMatrix4fv(
                u_model_matrix.id,
                1,
                gl::FALSE,
                &model_matrix.columns(0, 4)[0],
            );
            gl::UniformMatrix4fv(
                u_view_matrix.id,
                1,
                gl::FALSE,
                &view_matrix.columns(0, 4)[0],
            );
            gl::UniformMatrix4fv(
                u_proj_matrix.id,
                1,
                gl::FALSE,
                &proj_matrix.columns(0, 4)[0],
            );

            // Setup geometry for rendering
            for i in 0..self.geometry.len() {
                self.geometry[i].vbo.bind();
                self.geometry[i].ibo.bind();
                self.geometry[i].vao.enable(i as u32);
            }

            // Make the render call!
            gl::DrawElements(
                gl::TRIANGLES,
                self.indices.len() as i32,
                gl::UNSIGNED_INT,
                0 as *const _,
            );
        }
    }
}

impl OpenGl {
    pub fn new(camera: Camera, program: Program) -> Self {
        Self { camera, program }
    }

    pub fn set_shader_uniforms(&self, sun_dir: nalgebra_glm::Vec3, resolution: nalgebra_glm::Vec2) {
        self.program.set();
        let u_resolution = self.get_uniform("u_resolution").unwrap();
        let u_sun_dir = self.get_uniform("u_sun_dir").unwrap();
        unsafe {
            gl::Uniform2f(u_resolution.id, resolution.x, resolution.y);
            gl::Uniform3f(u_sun_dir.id, sun_dir.x, sun_dir.y, sun_dir.z);
        }
    }

    pub fn get_uniform(&self, uniform_name: &str) -> Result<Uniform, &'static str> {
        Uniform::new(self.program.id(), uniform_name)
    }

    pub fn program(&self) -> GLuint {
        self.program.id()
    }
}

fn flatten_positions(vertices: &Vec<TexturedVertex>) -> Vec<f32> {
    let mut retval = vec![];
    for vertex in vertices {
        retval.push(vertex.position[0]);
        retval.push(vertex.position[1]);
        retval.push(vertex.position[2]);
    }
    retval
}

fn flatten_normals(vertices: &Vec<TexturedVertex>) -> Vec<f32> {
    let mut retval = vec![];
    for vertex in vertices {
        retval.push(vertex.normal[0]);
        retval.push(vertex.normal[1]);
        retval.push(vertex.normal[2]);
    }
    retval
}

fn flatten_uv(vertices: &Vec<TexturedVertex>) -> Vec<f32> {
    let mut retval = vec![];
    for vertex in vertices {
        retval.push(vertex.texture[0]);
        retval.push(vertex.texture[1]);
        retval.push(vertex.texture[2]);
    }
    retval
}

fn vec_u32_from_vec_u16(input: &Vec<u16>) -> Vec<u32> {
    let mut retval = vec![];
    for x in input {
        retval.push(*x as u32);
    }
    retval
}