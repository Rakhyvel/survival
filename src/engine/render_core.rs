use std::{cell::RefCell, collections::HashMap, ops::Deref};

use gl::types::GLuint;
use obj::{load_obj, Obj, TexturedVertex};

use super::{
    aabb::AABB,
    camera::Camera,
    objects::{Buffer, Program, Texture, Uniform, Vao},
    text,
};

#[derive(Default)]
pub struct RenderContext {
    pub(crate) camera: RefCell<Camera>,
    pub(crate) program: RefCell<Option<ProgramId>>,
    mesh_manager: RefCell<ResourceManager<Mesh, MeshId>>,
    texture_manager: RefCell<ResourceManager<Texture, TextureId>>,
    program_manager: RefCell<ResourceManager<Program, ProgramId>>,
    // TODO: Move font manager in here, too
    pub(crate) int_screen_resolution: nalgebra_glm::I32Vec2,
}

struct ResourceManager<Resource, Id: OpaqueId> {
    resources: Vec<Resource>,
    keys: HashMap<&'static str, Id>,
}

trait OpaqueId: Copy {
    fn new(id: usize) -> Self;
    fn as_usize(&self) -> usize;
}

/// Opaque type used by the mesh manager to associate meshes.
#[derive(Copy, Clone)]
pub struct MeshId(usize);

/// Opaque type used by the texture manager to associate textures.
#[derive(Copy, Clone)]
pub struct TextureId(usize);

/// Opaque type used by the program manager to associate programs.
#[derive(Copy, Clone)]
pub struct ProgramId(usize);

/// An actual model, with geometry, a position, scale, rotation, and texture.
pub struct ModelComponent {
    pub mesh_id: MeshId,
    pub texture_id: TextureId,
    position: nalgebra_glm::Vec3,
    scale: nalgebra_glm::Vec3,
    model_matrix: nalgebra_glm::Mat4,
    pub shown: bool,
    pub outlined: bool,
}

/// Stores the geometry of a mesh. Meshes are registered in the mesh manager, and can be potentially shared across
/// multiple models.
pub(crate) struct Mesh {
    geometry: Vec<GeometryData>,
    indices: Vec<u32>,
    aabb: AABB,
}

/// Actual geometry data for a mesh.
struct GeometryData {
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

impl RenderContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_camera(&self, camera: Camera) {
        *self.camera.borrow_mut() = camera
    }

    pub fn set_program(&self, name: Option<&'static str>) {
        let manager = self.program_manager.borrow();
        if name.is_some() {
            let program_id = manager.get_id_from_name(name.unwrap()).unwrap();
            *self.program.borrow_mut() = Some(program_id);
            let program = manager.get_from_id(program_id).unwrap();
            program.set();
        } else {
            *self.program.borrow_mut() = None;
            unsafe {
                gl::UseProgram(0);
            }
        }
    }

    pub fn set_program_from_id(&self, program_id: ProgramId) {
        let manager = self.program_manager.borrow();
        *self.program.borrow_mut() = Some(program_id);
        let program = manager.get_from_id(program_id).unwrap();
        program.set();
    }

    pub fn add_mesh(&self, mesh: Mesh, name: Option<&'static str>) -> MeshId {
        self.mesh_manager.borrow_mut().add(mesh, name)
    }

    pub fn add_mesh_from_obj(&self, obj_file_data: &[u8], name: Option<&'static str>) -> MeshId {
        self.add_mesh(Mesh::from_obj(obj_file_data), name)
    }

    pub fn add_mesh_from_verts(
        &self,
        indices: Vec<u32>,
        datas: Vec<&Vec<f32>>,
        name: Option<&'static str>,
    ) -> MeshId {
        self.add_mesh(Mesh::new(indices, datas), name)
    }

    pub fn add_texture(&self, texture: Texture, name: Option<&'static str>) -> TextureId {
        self.texture_manager.borrow_mut().add(texture, name)
    }

    pub fn add_program(&self, program: Program, name: Option<&'static str>) -> ProgramId {
        self.program_manager.borrow_mut().add(program, name)
    }

    pub fn get_mesh(&self, name: &'static str) -> Option<std::cell::Ref<'_, Mesh>> {
        if let Some(id) = self.get_mesh_id_from_name(name) {
            self.get_mesh_from_id(id)
        } else {
            None
        }
    }

    pub fn get_texture(&self, name: &'static str) -> Option<std::cell::Ref<'_, Texture>> {
        if let Some(id) = self.get_texture_id_from_name(name) {
            self.get_texture_from_id(id)
        } else {
            None
        }
    }

    pub fn get_program(&self, name: &'static str) -> Option<std::cell::Ref<'_, Program>> {
        if let Some(id) = self.get_program_id_from_name(name) {
            self.get_program_from_id(id)
        } else {
            None
        }
    }

    pub fn get_mesh_from_id(&self, id: MeshId) -> Option<std::cell::Ref<'_, Mesh>> {
        let manager = self.mesh_manager.borrow();
        if let Some(mesh) = manager.get_from_id(id) {
            // Map the Ref<MeshManager> to Ref<Mesh>
            Some(std::cell::Ref::map(manager, |m| m.get_from_id(id).unwrap()))
        } else {
            None
        }
    }

    pub fn get_texture_from_id(&self, id: TextureId) -> Option<std::cell::Ref<'_, Texture>> {
        let manager = self.texture_manager.borrow();
        if let Some(texture) = manager.get_from_id(id) {
            // Map the Ref<TextureManager> to Ref<Texture>
            Some(std::cell::Ref::map(manager, |m| m.get_from_id(id).unwrap()))
        } else {
            None
        }
    }

    pub fn get_program_from_id(&self, id: ProgramId) -> Option<std::cell::Ref<'_, Program>> {
        let manager = self.program_manager.borrow();
        if let Some(program) = manager.get_from_id(id) {
            // Map the Ref<ProgramManager> to Ref<Program>
            Some(std::cell::Ref::map(manager, |m| m.get_from_id(id).unwrap()))
        } else {
            None
        }
    }

    pub fn get_mesh_id_from_name(&self, name: &'static str) -> Option<MeshId> {
        self.mesh_manager.borrow().get_id_from_name(name)
    }

    pub fn get_texture_id_from_name(&self, name: &'static str) -> Option<TextureId> {
        self.texture_manager.borrow().get_id_from_name(name)
    }

    pub fn get_program_id_from_name(&self, name: &'static str) -> Option<ProgramId> {
        self.program_manager.borrow().get_id_from_name(name)
    }

    pub fn get_mesh_aabb(&self, mesh_id: MeshId) -> AABB {
        self.mesh_manager
            .borrow()
            .get_from_id(mesh_id)
            .unwrap()
            .aabb
    }

    pub fn get_model_aabb(&self, model: &ModelComponent) -> AABB {
        self.mesh_manager
            .borrow()
            .get_from_id(model.mesh_id)
            .unwrap()
            .aabb
            .scale(model.scale)
            .translate(model.position)
    }

    pub fn get_current_program_id(&self) -> u32 {
        if self.program.borrow().is_some() {
            let program = self
                .get_program_from_id(self.program.borrow().unwrap())
                .unwrap();
            program.id()
        } else {
            0
        }
    }

    pub fn get_program_uniform(&self, uniform_name: &str) -> Result<Uniform, &'static str> {
        Uniform::new(self.get_current_program_id(), uniform_name)
    }

    pub fn draw(
        &self,
        mesh: &Mesh,
        model_matrix: nalgebra_glm::Mat4,
        view_matrix: nalgebra_glm::Mat4,
        proj_matrix: nalgebra_glm::Mat4,
    ) {
        let u_model_matrix: Uniform = self.get_program_uniform("u_model_matrix").unwrap();
        let u_view_matrix = self.get_program_uniform("u_view_matrix").unwrap();
        let u_proj_matrix = self.get_program_uniform("u_proj_matrix").unwrap();
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
            for i in 0..mesh.geometry.len() {
                mesh.geometry[i].vbo.bind();
                mesh.geometry[i].ibo.bind();
                mesh.geometry[i].vao.enable(i as u32);
            }

            // Make the render call!
            gl::DrawElements(
                gl::TRIANGLES,
                mesh.indices.len() as i32,
                gl::UNSIGNED_INT,
                0 as *const _,
            );
        }
    }
}

impl<Resource, Id: OpaqueId> ResourceManager<Resource, Id> {
    pub fn new() -> Self {
        Self {
            resources: vec![],
            keys: HashMap::new(),
        }
    }

    pub fn add(&mut self, res: Resource, name: Option<&'static str>) -> Id {
        let id = Id::new(self.resources.len());
        self.resources.push(res);
        if name.is_some() {
            self.keys.insert(name.unwrap(), id);
        }
        id
    }

    pub fn get_from_id(&self, id: Id) -> Option<&Resource> {
        self.resources.get(id.as_usize())
    }

    pub fn get_id_from_name(&self, name: &'static str) -> Option<Id> {
        self.keys.get(name).copied()
    }

    pub fn get(&self, name: &'static str) -> Option<&Resource> {
        self.get_from_id(self.get_id_from_name(name).unwrap())
    }
}

impl<Resource, Id: OpaqueId> Default for ResourceManager<Resource, Id> {
    fn default() -> Self {
        Self {
            resources: vec![],
            keys: HashMap::new(),
        }
    }
}

impl OpaqueId for MeshId {
    fn new(id: usize) -> Self {
        MeshId(id)
    }

    fn as_usize(&self) -> usize {
        self.0
    }
}

impl OpaqueId for TextureId {
    fn new(id: usize) -> Self {
        TextureId(id)
    }

    fn as_usize(&self) -> usize {
        self.0
    }
}

impl OpaqueId for ProgramId {
    fn new(id: usize) -> Self {
        ProgramId(id)
    }

    fn as_usize(&self) -> usize {
        self.0
    }
}

impl ModelComponent {
    pub fn new(
        mesh_id: MeshId,
        texture_id: TextureId,
        position: nalgebra_glm::Vec3,
        scale: nalgebra_glm::Vec3,
    ) -> Self {
        Self {
            mesh_id,
            texture_id,
            position,
            scale,
            model_matrix: Self::construct_model_matrix(&position, &scale),
            shown: true,
            outlined: false,
        }
    }

    pub fn set_position(&mut self, position: nalgebra_glm::Vec3) {
        self.position = position;
        self.regen_model_matrix();
    }

    pub fn get_position(&self) -> nalgebra_glm::Vec3 {
        self.position
    }

    pub fn set_scale(&mut self, scale: nalgebra_glm::Vec3) {
        self.scale = scale;
        self.regen_model_matrix();
    }

    pub fn get_scale(&self) -> nalgebra_glm::Vec3 {
        self.scale
    }

    pub fn get_model_matrix(&self) -> nalgebra_glm::Mat4 {
        self.model_matrix
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

        // Allocate VRAM buffers (this is slow!)
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
