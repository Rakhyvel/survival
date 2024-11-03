use hecs::{Entity, World};
use rand::{Rng, SeedableRng};

use super::{
    bvh::BVH,
    perlin::{HeightMap, PerlinMap},
    render3d::{Mesh, MeshManager, ModelComponent, TextureId, TextureManager},
    sphere::Sphere,
};

#[derive(Default)]
pub struct Chunk {
    map: PerlinMap,
    hydration: PerlinMap,
    chunk_width: usize,
    pos: nalgebra_glm::Vec2,
    generated: bool,

    level_of_detail: f32,
    seed: i32,
    amplitude: f32,
}

#[derive(Default)]
pub struct ChunkedPerlinMap {
    chunks: Vec<Chunk>,
    map_width: usize,
    chunk_width: usize,

    level_of_detail: f32,
    seed: i32,
    amplitude: f32,
}

struct Tree {}

impl Chunk {
    pub fn new(
        chunk_width: usize,
        pos: nalgebra_glm::Vec2,
        level_of_detail: f32,
        seed: i32,
        amplitude: f32,
    ) -> Self {
        Self {
            map: PerlinMap::new(chunk_width + 1),
            hydration: PerlinMap::new(chunk_width + 1),
            chunk_width,
            pos,
            generated: false,
            level_of_detail,
            seed,
            amplitude,
        }
    }

    pub fn generate(
        &mut self,
        world: &mut World,
        bvh: &mut BVH<Entity>,
        mesh_mgr: &mut MeshManager,
        texture_mgr: &TextureManager,
    ) {
        if !self.generated {
            self.map.generate(
                self.level_of_detail,
                10,
                self.seed,
                self.amplitude,
                self.pos,
            );
            self.hydration
                .generate(self.level_of_detail, 2, self.seed, self.amplitude, self.pos);

            self.map.create_bulge();
            self.map.create_shelf(0.6, 0.4);

            let mut rng = rand::rngs::StdRng::from_entropy();
            // self.map.erode(64, rand::Rng::gen(&mut rng));

            let grass_texture = texture_mgr.get_id_from_name("grass").unwrap();
            let tree_texture = texture_mgr.get_id_from_name("tree").unwrap();
            let rock_texture = texture_mgr.get_id_from_name("rock").unwrap();
            let tree_mesh = mesh_mgr.get_id_from_name("tree").unwrap();
            let bush_mesh = mesh_mgr.get_id_from_name("bush").unwrap();
            let cube_mesh = mesh_mgr.get_id_from_name("cube").unwrap();

            let pos_with_z = nalgebra_glm::vec3(self.pos.x, self.pos.y, 0.0);
            let (i, v, n, u) = self.create_mesh();
            let grass_mesh = mesh_mgr.add_mesh(Mesh::new(i, vec![&v, &n, &u]), None);
            let chunk_entity = world.spawn((ModelComponent::new(
                grass_mesh,
                grass_texture,
                pos_with_z,
                nalgebra_glm::vec3(1.0, 1.0, 1.0),
            ),));
            bvh.insert(
                chunk_entity,
                mesh_mgr
                    .get_mesh_from_id(grass_mesh)
                    .unwrap()
                    .aabb
                    .translate(pos_with_z),
            );

            // TODO: This should be OUT!

            for _ in 0..2 {
                // Add all the rocks
                let mut position = nalgebra_glm::vec3(
                    rng.gen_range(0..self.chunk_width) as f32,
                    rng.gen_range(0..self.chunk_width) as f32,
                    0.0,
                );
                let scale = 0.2;
                position.z = self.map.get_z_interpolated(position.xy());
                if position.z < 1.0 {
                    continue;
                }
                position.x += self.pos.x;
                position.y += self.pos.y;
                let rock_entity = world.spawn((ModelComponent::new(
                    cube_mesh,
                    rock_texture,
                    position,
                    nalgebra_glm::vec3(scale, scale, scale),
                ),));
                bvh.insert(
                    rock_entity,
                    mesh_mgr
                        .get_mesh_from_id(cube_mesh)
                        .unwrap()
                        .aabb
                        .translate(position),
                );
            }

            for _ in 0..16 {
                // Add all the trees
                let pos = nalgebra_glm::vec2(
                    rng.gen::<f32>() * (self.chunk_width as f32 - 1.0),
                    rng.gen::<f32>() * (self.chunk_width as f32 - 1.0),
                );
                let height = self.map.get_z_interpolated(pos);
                let hydro_normal = self.hydration.get_normal(pos);
                let variation: f32 = rng.gen_range(0.0..1.0);
                let scale = (1.4 + 1.0 * variation);
                let scale_vec = nalgebra_glm::vec3(scale, scale, scale * 0.8);
                let position = nalgebra_glm::vec3(pos.x + self.pos.x, pos.y + self.pos.y, height);
                if height >= 1.0
                    && hydro_normal.y > 0.0
                    && 0.5 < self.hydration.height(pos)
                    && bvh
                        .iter_sphere(&Sphere::new(position, scale))
                        .filter(|entity| world.get::<&Tree>(*entity).is_ok())
                        .count()
                        == 0
                {
                    let tree_entity = world.spawn((
                        ModelComponent::new(tree_mesh, tree_texture, position, scale_vec),
                        Tree {},
                    ));
                    bvh.insert(
                        tree_entity,
                        mesh_mgr
                            .get_mesh_from_id(cube_mesh)
                            .unwrap()
                            .aabb
                            .scale(scale_vec)
                            .translate(position),
                    );
                }
            }

            for _ in 0..16 {
                // Add all the bushes
                let pos = nalgebra_glm::vec2(
                    rng.gen::<f32>() * (self.chunk_width as f32 - 1.0),
                    rng.gen::<f32>() * (self.chunk_width as f32 - 1.0),
                );
                let height = self.map.get_z_interpolated(pos);
                let hydro_normal = self.hydration.get_normal(pos);
                let variation: f32 = rng.gen_range(0.0..1.0);
                let scale = (0.4 + 1.0 * variation);
                let scale_vec = nalgebra_glm::vec3(scale, scale, scale * 0.8);
                let position = nalgebra_glm::vec3(pos.x + self.pos.x, pos.y + self.pos.y, height);
                if height >= 1.0
                    && variation < (hydro_normal.y + 0.5) * 1.0
                    && bvh
                        .iter_sphere(&Sphere::new(position, scale))
                        .filter(|entity| world.get::<&Tree>(*entity).is_ok())
                        .count()
                        == 0
                {
                    let tree_entity = world.spawn((ModelComponent::new(
                        bush_mesh,
                        tree_texture,
                        position,
                        scale_vec,
                    ),));
                    bvh.insert(
                        tree_entity,
                        mesh_mgr
                            .get_mesh_from_id(bush_mesh)
                            .unwrap()
                            .aabb
                            .scale(scale_vec)
                            .translate(position),
                    );
                }
            }

            self.generated = true;
        }
    }

    fn pos(&self) -> nalgebra_glm::Vec2 {
        self.pos
    }

    fn height_nearest(&self, p: nalgebra_glm::Vec2) -> f32 {
        self.map.height(p)
    }

    fn height_interpolated(&self, p: nalgebra_glm::Vec2) -> f32 {
        self.map.get_z_interpolated(p)
    }

    fn normal(&self, p: nalgebra_glm::Vec2) -> nalgebra_glm::Vec3 {
        self.map.get_normal(p)
    }

    fn flow(&self, p: nalgebra_glm::Vec2) -> f32 {
        self.map.flow(p)
    }

    fn create_mesh(&self) -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>) {
        let mut indices = Vec::<u32>::new();
        let mut vertices = Vec::<f32>::new();
        let mut normals = Vec::<f32>::new();
        let mut uv = Vec::<f32>::new();

        let mut i = 0;
        for y in 0..self.chunk_width {
            let y = y;
            for x in 0..self.chunk_width {
                let x = x;
                // Left triangle |\
                let offsets = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
                self.add_triangle(
                    &mut indices,
                    &mut vertices,
                    &mut normals,
                    &mut uv,
                    x as f32,
                    y as f32,
                    &offsets,
                    &mut i,
                );

                // Right triangle \|
                let offsets = vec![(1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
                self.add_triangle(
                    &mut indices,
                    &mut vertices,
                    &mut normals,
                    &mut uv,
                    x as f32,
                    y as f32,
                    &offsets,
                    &mut i,
                );
            }
        }

        (indices, vertices, normals, uv)
    }

    fn add_triangle(
        &self,
        indices: &mut Vec<u32>,
        vertices: &mut Vec<f32>,
        normals: &mut Vec<f32>,
        uv: &mut Vec<f32>,
        x: f32,
        y: f32,
        offsets: &Vec<(f32, f32)>,
        i: &mut u32,
    ) {
        let mut sum_z = 0.0;
        let tri_verts: Vec<nalgebra_glm::Vec3> = offsets
            .iter()
            .map(|(xo, yo)| {
                let z = self.height_nearest(nalgebra_glm::vec2(x + xo, y + yo));
                let mapval = nalgebra_glm::vec3(x + xo, y + yo, z);
                sum_z += self.height_nearest(nalgebra_glm::vec2(x + xo, y + yo));
                add_vertex(vertices, x + xo, y + yo, z);
                indices.push(*i);
                *i += 1;
                mapval
            })
            .collect();

        let edge1 = tri_verts[1] - tri_verts[0];
        let edge2 = tri_verts[2] - tri_verts[0];
        let normal = nalgebra_glm::cross(&edge1, &edge2).normalize();
        for _ in 0..3 {
            normals.push(normal.x);
            normals.push(normal.y);
            normals.push(normal.z);
        }
        // 0 = steep
        // 1 = flat
        let dot_prod = nalgebra_glm::dot(&normal, &nalgebra_glm::vec3(0.0, 0.0, 1.0));

        let avg_z = sum_z / 3.0;
        let u_offset: f32 = if avg_z < 0.5 || (avg_z < 0.9 * dot_prod && 0.9 < dot_prod) {
            3.0 / 9.0
        } else if dot_prod < 0.9 {
            5.0 / 9.0
        } else {
            0.0
        };
        let v_offset = 0.0;
        for _ in 0..3 {
            add_uv(uv, u_offset, v_offset);
        }
    }
}

fn add_vertex(vertices: &mut Vec<f32>, x: f32, y: f32, z: f32) {
    vertices.push(x);
    vertices.push(y);
    vertices.push(z);
}

fn add_uv(uv: &mut Vec<f32>, x: f32, y: f32) {
    uv.push(x);
    uv.push(y);
    uv.push(0.0);
}

impl ChunkedPerlinMap {
    pub fn new(
        map_width: usize,
        chunk_width: usize,
        level_of_detail: f32,
        seed: i32,
        amplitude: f32,
    ) -> Self {
        let mut chunks: Vec<Chunk> = vec![];
        let side_chunks = map_width / chunk_width;
        for y in 0..side_chunks {
            for x in 0..side_chunks {
                chunks.push(Chunk::new(
                    chunk_width,
                    nalgebra_glm::vec2((x * chunk_width) as f32, (y * chunk_width) as f32),
                    level_of_detail,
                    seed,
                    amplitude,
                ));
            }
        }
        Self {
            chunks,
            map_width,
            chunk_width,
            level_of_detail,
            seed,
            amplitude,
        }
    }

    pub fn check_chunks(
        &mut self,
        p: nalgebra_glm::Vec2,
        world: &mut World,
        bvh: &mut BVH<Entity>,
        mesh_mgr: &mut MeshManager,
        texture_mgr: &TextureManager,
    ) {
        let chunk_offsets = [
            nalgebra_glm::vec2(-1.0, 1.0),
            nalgebra_glm::vec2(-1.0, 0.0),
            nalgebra_glm::vec2(-1.0, -1.0),
            nalgebra_glm::vec2(0.0, 1.0),
            nalgebra_glm::vec2(0.0, 0.0),
            nalgebra_glm::vec2(0.0, -1.0),
            nalgebra_glm::vec2(1.0, 1.0),
            nalgebra_glm::vec2(1.0, 0.0),
            nalgebra_glm::vec2(1.0, -1.0),
        ];
        for chunk_offset in chunk_offsets {
            let chunk_pos = chunk_offset * (self.chunk_width as f32) + p;
            let chunk = self.chunk_at_mut(chunk_pos);
            chunk.generate(world, bvh, mesh_mgr, texture_mgr);
        }
    }

    fn chunk_at(&self, p: nalgebra_glm::Vec2) -> &Chunk {
        let side_chunks = self.map_width / self.chunk_width;
        let chunk_p = p / self.chunk_width as f32;
        &self.chunks[chunk_p.y as usize * side_chunks + chunk_p.x as usize]
    }

    fn chunk_at_mut(&mut self, p: nalgebra_glm::Vec2) -> &mut Chunk {
        let side_chunks = self.map_width / self.chunk_width;
        let chunk_p = p / self.chunk_width as f32;
        &mut self.chunks[chunk_p.y as usize * side_chunks + chunk_p.x as usize]
    }
}

impl HeightMap for ChunkedPerlinMap {
    fn height_nearest(&self, p: nalgebra_glm::Vec2) -> f32 {
        let chunk = self.chunk_at(p);
        chunk.height_nearest(p - chunk.pos())
    }

    fn height_interpolated(&self, p: nalgebra_glm::Vec2) -> f32 {
        let chunk = self.chunk_at(p);
        chunk.height_interpolated(p - chunk.pos())
    }

    fn normal(&self, p: nalgebra_glm::Vec2) -> nalgebra_glm::Vec3 {
        let chunk = self.chunk_at(p);
        chunk.normal(p - chunk.pos())
    }

    fn flow(&self, p: nalgebra_glm::Vec2) -> f32 {
        let chunk = self.chunk_at(p);
        chunk.flow(p - chunk.pos())
    }
}
