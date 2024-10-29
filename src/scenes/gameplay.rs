use core::f32;

use hecs::{Entity, World};
use rand::{Rng, SeedableRng};
use sdl2::keyboard::Scancode;

use crate::{
    engine::{
        bvh::{BVHNodeId, BVH},
        camera::{Camera, ProjectionKind},
        objects::{create_program, Texture},
        perlin::PerlinMap,
        render3d::{self, Mesh, MeshManager, ModelComponent, OpenGl},
        shadow_map::{self, DirectionalLightSource},
        sphere::Sphere,
    },
    App, Scene,
};

const MAP_WIDTH: usize = 512;
const CHUNK_SIZE: usize = 16;
const UNIT_PER_METER: f32 = 0.05;

pub const QUAD_DATA: &[u8] = include_bytes!("../../res/quad.obj");
pub const CONE_DATA: &[u8] = include_bytes!("../../res/cone.obj");
pub const CUBE_DATA: &[u8] = include_bytes!("../../res/cube.obj");
pub const BUSH_DATA: &[u8] = include_bytes!("../../res/bush.obj");

struct Player {
    bvh_node_id: BVHNodeId,
}

struct Rock {}

pub struct Gameplay {
    world: World,
    open_gl: OpenGl,
    mesh_mgr: MeshManager,
    map: PerlinMap,
    bvh: BVH<Entity>,
    directional_light: DirectionalLightSource,

    // Player stuff
    position: nalgebra_glm::Vec3,
    velocity: nalgebra_glm::Vec3,

    prev_space_state: bool,
    debug: bool,

    swap: u32,
}

impl Scene for Gameplay {
    fn update(&mut self, app: &App) {
        const MINUTES_PER_DAY: f32 = 1.0;
        const TICKS_OFFSET: f32 = -1.5;
        self.directional_light.light_dir.z =
            (app.ticks as f32 / (60.0 * 60.0 * 0.5 * MINUTES_PER_DAY) + TICKS_OFFSET).cos();
        self.directional_light.light_dir.y =
            (app.ticks as f32 / (60.0 * 60.0 * 0.5 * MINUTES_PER_DAY) + TICKS_OFFSET).sin();
        self.update_view(app);
    }

    fn render(&mut self, app: &App) {
        if self.swap % 2 == 0 {
            // ok this is funny, just don't render the sun shadows so much! lmao
            shadow_map::directional_light_system(
                &mut self.directional_light,
                &mut self.world,
                &mut self.open_gl,
                &self.mesh_mgr,
                &self.bvh,
            );
        }
        self.swap += 1;
        render3d::render_3d_models_system(
            &mut self.world,
            &mut self.open_gl,
            &self.directional_light,
            &self.mesh_mgr,
            &self.bvh,
            app.window_size,
            self.debug,
        );
    }
}

impl Gameplay {
    pub fn new() -> Self {
        let mut world = World::new();

        println!("Setting up island...");
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut map = PerlinMap::new(MAP_WIDTH, 0.03, rand::Rng::gen(&mut rng), 1.0);

        map.normalize();
        map.create_bulge();
        map.create_shelf(0.6, 0.4);

        println!("Eroding...");
        let start = std::time::Instant::now();
        map.erode(MAP_WIDTH, rand::Rng::gen(&mut rng));
        println!("Erode time: {:?}", start.elapsed());

        // map.create_shelf(0.6, 0.4);

        // Setup the mesh manager
        let mut mesh_mgr = MeshManager::new();
        let quad_mesh = mesh_mgr.add_mesh(Mesh::from_obj(QUAD_DATA));
        let tree_mesh = mesh_mgr.add_mesh(Mesh::from_obj(CONE_DATA));
        let cube_mesh = mesh_mgr.add_mesh(Mesh::from_obj(CUBE_DATA));
        let bush_mesh = mesh_mgr.add_mesh(Mesh::from_obj(BUSH_DATA));

        // Setup the BVH
        let mut bvh = BVH::<Entity>::new();

        // bvh.walk_tree();

        let spawn_point =
            nalgebra_glm::vec3(MAP_WIDTH as f32 / 2.0 + 1.0, MAP_WIDTH as f32 / 2.0, 2.5);

        let player_entity = world.spawn((
            ModelComponent::new(
                cube_mesh,
                spawn_point,
                nalgebra_glm::vec3(0.4, 0.4, 1.0),
                Texture::from_png("res/grass.png"),
            ),
            Rock {},
        ));
        let player_node_id = bvh.insert(
            player_entity,
            mesh_mgr
                .get_mesh(cube_mesh)
                .unwrap()
                .aabb
                .scale(nalgebra_glm::vec3(0.4, 0.4, 1.0))
                .translate(spawn_point),
        );
        world
            .insert(
                player_entity,
                (Player {
                    bvh_node_id: player_node_id,
                },),
            )
            .unwrap();

        for _ in 0..MAP_WIDTH * 1 {
            // Add all the rocks
            let mut position = nalgebra_glm::vec3(
                rng.gen_range(0..MAP_WIDTH) as f32,
                rng.gen_range(0..MAP_WIDTH) as f32,
                0.0,
            );
            let scale = 0.2;
            if bvh.iter_sphere(&Sphere::new(position, 1.0)).count() == 0 {
                position.z = map.get_z_interpolated(position.xy());
                let rock_entity = world.spawn((
                    ModelComponent::new(
                        cube_mesh,
                        position,
                        nalgebra_glm::vec3(scale, scale, scale),
                        Texture::from_png("res/grass.png"),
                    ),
                    Rock {},
                ));
                bvh.insert(
                    rock_entity,
                    mesh_mgr
                        .get_mesh(cube_mesh)
                        .unwrap()
                        .aabb
                        .translate(position),
                );
            }
        }
        for _ in 0..(MAP_WIDTH) {
            // Add all the trees
            let mut attempts = 0;
            loop {
                let pos = nalgebra_glm::vec2(
                    rng.gen::<f32>() * (MAP_WIDTH as f32 - 1.0),
                    rng.gen::<f32>() * (MAP_WIDTH as f32 - 1.0),
                );
                let height = map.get_z_interpolated(pos);
                let dot_prod = map.get_dot_prod(pos).abs();
                let variation = rng.gen_range(0.0..1.0);
                let scale = (30.0 + 70.0 * variation) * UNIT_PER_METER;
                let scale_vec = nalgebra_glm::vec3(scale, scale, scale * 0.8);
                let position = nalgebra_glm::vec3(pos.x, pos.y, height);
                if height >= 1.0
                    && dot_prod > 0.99
                    // && map.flow(pos) > 8.0
                    && bvh.iter_sphere(&Sphere::new(position, scale)).count() == 0
                {
                    let tree_entity = world.spawn((ModelComponent::new(
                        tree_mesh,
                        position,
                        scale_vec,
                        Texture::from_png("res/tree.png"),
                    ),));
                    bvh.insert(
                        tree_entity,
                        mesh_mgr
                            .get_mesh(cube_mesh)
                            .unwrap()
                            .aabb
                            .scale(scale_vec)
                            .translate(position),
                    );
                    // bvh.remove(tree_bvh_id);
                    break;
                }
                if attempts > 100 {
                    break;
                }
                attempts += 1;
            }
        }
        for _ in 0..(MAP_WIDTH) {
            // Add all the bushes
            let mut attempts = 0;
            loop {
                let pos = nalgebra_glm::vec2(
                    rng.gen::<f32>() * (MAP_WIDTH as f32 - 1.0),
                    rng.gen::<f32>() * (MAP_WIDTH as f32 - 1.0),
                );
                let height = map.get_z_interpolated(pos);
                let dot_prod = map.get_dot_prod(pos).abs();
                let variation = rng.gen_range(0.0..1.0);
                let position = nalgebra_glm::vec3(pos.x, pos.y, height);
                let scale = (3.5 + 7.0 * variation) * UNIT_PER_METER;
                if height >= 1.0 && dot_prod >= 0.8
                // && dot_prod <= 0.9
                // && bvh.iter_sphere(&Sphere::new(position, scale)).count() == 0
                //  && map.flow(pos) > 6.0
                {
                    let bush_entity = world.spawn((ModelComponent::new(
                        bush_mesh,
                        position,
                        nalgebra_glm::vec3(scale, scale, scale * 0.8),
                        Texture::from_png("res/tree.png"),
                    ),));
                    bvh.insert(
                        bush_entity,
                        mesh_mgr
                            .get_mesh(cube_mesh)
                            .unwrap()
                            .aabb
                            .translate(position),
                    );
                    break;
                }
                if attempts > 100 {
                    break;
                }
                attempts += 1;
            }
        }

        // Add the chunks
        for chunk_y in (0..(MAP_WIDTH)).step_by(CHUNK_SIZE) {
            for chunk_x in (0..(MAP_WIDTH)).step_by(CHUNK_SIZE) {
                let (i, v, n, u) = create_mesh(&map, chunk_x, chunk_y);
                let grass_mesh = mesh_mgr.add_mesh(Mesh::new(i, vec![&v, &n, &u]));
                let chunk_position = nalgebra_glm::vec3(chunk_x as f32, chunk_y as f32, 0.0);
                let chunk_entity = world.spawn((ModelComponent::new(
                    grass_mesh,
                    chunk_position,
                    nalgebra_glm::vec3(1.0, 1.0, 1.0),
                    Texture::from_png("res/grass.png"),
                ),));
                bvh.insert(
                    chunk_entity,
                    mesh_mgr
                        .get_mesh(grass_mesh)
                        .unwrap()
                        .aabb
                        .translate(chunk_position),
                );
            }
        }

        let scale_vec = nalgebra_glm::vec3(1000.0, 1000.0, 1000.0);
        let water_entity = world.spawn((ModelComponent::new(
            quad_mesh,
            nalgebra_glm::vec3(0.0, 0.0, 0.5),
            scale_vec,
            Texture::from_png("res/water.png"),
        ),));
        bvh.insert(
            water_entity,
            mesh_mgr
                .get_mesh(quad_mesh)
                .unwrap()
                .aabb
                .scale(scale_vec)
                .translate(nalgebra_glm::vec3(0.0, 0.0, 0.5)),
        );

        Self {
            world,
            open_gl: OpenGl::new(
                Camera::new(
                    spawn_point,
                    nalgebra_glm::vec3(MAP_WIDTH as f32 / 2.0, MAP_WIDTH as f32 / 2.0, 0.5),
                    nalgebra_glm::vec3(0.0, 0.0, 1.0),
                    ProjectionKind::Perspective { fov: 0.65 },
                ),
                create_program(
                    include_str!("../shaders/3d.vert"),
                    include_str!("../shaders/3d.frag"),
                )
                .unwrap(),
            ),
            bvh,
            mesh_mgr,
            map,
            directional_light: DirectionalLightSource::new(
                Camera::new(
                    nalgebra_glm::vec3(MAP_WIDTH as f32 / -2.0, 0.0, 2.0),
                    nalgebra_glm::vec3(MAP_WIDTH as f32 / 2.0, MAP_WIDTH as f32 / 2.0, 0.5),
                    nalgebra_glm::vec3(0.0, 0.0, 1.0),
                    ProjectionKind::Orthographic {
                        // These do not matter for now, they're reset later
                        left: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        top: 0.0,
                        near: 0.0,
                        far: 0.0,
                    },
                ),
                create_program(
                    include_str!("../shaders/shadow.vert"),
                    include_str!("../shaders/shadow.frag"),
                )
                .unwrap(),
                nalgebra_glm::vec3(-0.1, 0.0, 0.86),
                8192,
            ),
            position: spawn_point,
            velocity: nalgebra_glm::vec3(0.0, 0.0, 0.0),

            prev_space_state: false,
            debug: false,
            swap: 0,
        }
    }

    fn update_view(&mut self, app: &App) {
        let mut player_entt: Option<Entity> = None;
        for (entt, _) in &mut self.world.query::<&Player>() {
            player_entt = Some(entt);
            break;
        }

        let curr_w_state = app.keys[Scancode::W as usize];
        let curr_s_state = app.keys[Scancode::S as usize];
        let curr_a_state = app.keys[Scancode::A as usize];
        let curr_d_state = app.keys[Scancode::D as usize];
        let curr_space_state = app.keys[Scancode::Space as usize];
        let walking = curr_w_state || curr_s_state || curr_a_state || curr_d_state;
        let walk_speed: f32 = 1.6 * 2.5;
        let facing_vec = nalgebra_glm::vec3(1.0, 0.0, 0.0);
        let sideways_vec = nalgebra_glm::vec3(0.0, 1.0, 0.0);
        let mut player_vel_vec: nalgebra_glm::Vec3 = nalgebra_glm::zero();
        if curr_w_state {
            player_vel_vec += -facing_vec;
        }
        if curr_s_state {
            player_vel_vec += facing_vec;
        }
        if curr_a_state {
            player_vel_vec += -sideways_vec;
        }
        if curr_d_state {
            player_vel_vec += sideways_vec;
        }
        self.debug = false;
        if curr_space_state && !self.prev_space_state {
            self.debug = true;
            println!(
                "{:?} {:?}",
                self.open_gl.camera.position(),
                self.open_gl.camera.lookat()
            );
        } else if walking {
            // Move the player, this way moving diagonal isn't faster
            self.velocity +=
                player_vel_vec.normalize() * walk_speed * 4.317 * UNIT_PER_METER / 62.5;
        }
        self.prev_space_state = curr_space_state;
        self.position += self.velocity;
        self.position.z = self.map.get_z_interpolated(self.position.xy());

        let mut model = self
            .world
            .get::<&mut ModelComponent>(player_entt.unwrap())
            .unwrap();
        model.set_position(self.position);
        let player_bvh_node_id = self
            .world
            .get::<&Player>(player_entt.unwrap())
            .unwrap()
            .bvh_node_id;
        self.bvh.move_obj(
            player_bvh_node_id,
            &model.get_aabb(&self.mesh_mgr),
            &self.velocity,
        );
        self.velocity *= 0.8; // friction

        self.open_gl
            .camera
            .set_position(self.position + nalgebra_glm::vec3(138.5, 0.0, 80.0));
        self.open_gl.camera.set_lookat(self.position);
    }
}

fn create_mesh(
    map: &PerlinMap,
    chunk_x: usize,
    chunk_y: usize,
) -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut indices = Vec::<u32>::new();
    let mut vertices = Vec::<f32>::new();
    let mut normals = Vec::<f32>::new();
    let mut uv = Vec::<f32>::new();

    let mut i = 0;
    for y in 0..CHUNK_SIZE {
        let y = y + chunk_y;
        for x in 0..CHUNK_SIZE {
            let x = x + chunk_x;
            // Left triangle |\
            let offsets = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
            add_triangle(
                map,
                &mut indices,
                &mut vertices,
                &mut normals,
                &mut uv,
                x as f32,
                y as f32,
                chunk_x as f32,
                chunk_y as f32,
                &offsets,
                &mut i,
            );

            // Right triangle \|
            let offsets = vec![(1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            add_triangle(
                map,
                &mut indices,
                &mut vertices,
                &mut normals,
                &mut uv,
                x as f32,
                y as f32,
                chunk_x as f32,
                chunk_y as f32,
                &offsets,
                &mut i,
            );
        }
    }

    (indices, vertices, normals, uv)
}

fn add_triangle(
    tiles: &PerlinMap,
    indices: &mut Vec<u32>,
    vertices: &mut Vec<f32>,
    normals: &mut Vec<f32>,
    uv: &mut Vec<f32>,
    x: f32,
    y: f32,
    chunk_x: f32,
    chunk_y: f32,
    offsets: &Vec<(f32, f32)>,
    i: &mut u32,
) {
    let mut sum_z = 0.0;
    let tri_verts: Vec<nalgebra_glm::Vec3> = offsets
        .iter()
        .map(|(xo, yo)| {
            let z = tiles.height(nalgebra_glm::vec2(x + xo, y + yo));
            let mapval = nalgebra_glm::vec3(x + xo, y + yo, z);
            sum_z += tiles.height(nalgebra_glm::vec2(x + xo, y + yo));
            add_vertex(vertices, x + xo - chunk_x, y + yo - chunk_y, z);
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
    let uv_offset: f32 = if avg_z < 0.5 || (avg_z < 0.9 * dot_prod && 0.9 < dot_prod) {
        3.0 / 9.0
    } else if dot_prod < 0.9 {
        5.0 / 9.0
    } else {
        0.0
    };
    for _ in 0..3 {
        add_uv(uv, uv_offset, 0.0);
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
