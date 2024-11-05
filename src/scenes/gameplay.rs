use core::f32;

use hecs::{Entity, World};
use rand::SeedableRng;
use sdl2::keyboard::Scancode;

use crate::{
    engine::{
        bvh::{BVHNodeId, BVH},
        camera::{Camera, ProjectionKind},
        chunked_map::ChunkedPerlinMap,
        objects::{create_program, Program, Texture},
        perlin::HeightMap,
        ray::Ray,
        render3d::{self, Mesh, MeshManager, ModelComponent, OpenGl, TextureManager},
        shadow_map::{self, DirectionalLightSource},
    },
    App, Scene,
};

const MAP_WIDTH: usize = 16384; // 16k is desireable!
const CHUNK_SIZE: usize = 16;
const UNIT_PER_METER: f32 = 0.05;
const MINUTES_PER_DAY: f32 = 10.0;
const TICKS_OFFSET: f32 = 0.0;

pub const QUAD_DATA: &[u8] = include_bytes!("../../res/quad.obj");
pub const CUBE_DATA: &[u8] = include_bytes!("../../res/cube.obj");
pub const CONE_DATA: &[u8] = include_bytes!("../../res/cone.obj");
pub const BUSH_DATA: &[u8] = include_bytes!("../../res/bush.obj");

struct Player {
    bvh_node_id: BVHNodeId,
}

pub struct Rock {}

pub struct Gameplay {
    world: World,
    open_gl: OpenGl,
    mesh_mgr: MeshManager,
    texture_mgr: TextureManager,
    map: ChunkedPerlinMap,
    bvh: BVH<Entity>,
    directional_light: DirectionalLightSource,
    outline_program: Program,

    // Player stuff
    position: nalgebra_glm::Vec3,
    velocity: nalgebra_glm::Vec3,

    prev_space_state: bool,
    debug: bool,

    update_swap: u32,
}

impl Scene for Gameplay {
    fn update(&mut self, app: &App) {
        self.directional_light.light_dir.z =
            (app.ticks as f32 / (60.0 * 60.0 * 0.5 * MINUTES_PER_DAY) + TICKS_OFFSET).cos();
        self.directional_light.light_dir.y =
            (app.ticks as f32 / (60.0 * 60.0 * 0.5 * MINUTES_PER_DAY) + TICKS_OFFSET).sin();
        self.map.check_chunks(
            self.position.xy(),
            &mut self.world,
            &mut self.bvh,
            &mut self.mesh_mgr,
            &self.texture_mgr,
        );
        self.update_view(app);
        self.update_clickers(app);
        self.update_swap += 1;
    }

    fn render(&mut self, app: &App) {
        // sky system
        let model_t = app.ticks as f32 / (60.0 * 60.0 * 0.5 * MINUTES_PER_DAY) + TICKS_OFFSET;
        unsafe {
            let day_color = nalgebra_glm::vec3(172.0, 205.0, 248.0);
            let night_color = nalgebra_glm::vec3(5.0, 6.0, 7.0);
            let red_color = nalgebra_glm::vec3(124.0, 102.0, 86.0);
            let do_color = if model_t.cos() > 0.0 {
                day_color
            } else {
                night_color
            };
            let dnf = model_t.sin().powf(100.0);
            let result = dnf * red_color + (1.0 - dnf) * do_color;
            gl::ClearColor(result.x / 255., result.y / 255., result.z / 255., 1.0);
        }

        shadow_map::directional_light_system(
            &mut self.directional_light,
            &mut self.world,
            &mut self.open_gl,
            &self.mesh_mgr,
            &self.texture_mgr,
            &self.bvh,
        );
        render3d::render_3d_models_system(
            &mut self.world,
            &mut self.open_gl,
            &self.directional_light,
            &self.mesh_mgr,
            &self.texture_mgr,
            &self.bvh,
            app.window_size,
            self.debug,
        );
        render3d::render_3d_outlines_system(
            &mut self.world,
            &mut self.open_gl,
            &self.outline_program,
            &self.mesh_mgr,
            &self.bvh,
        );
    }
}

impl Gameplay {
    pub fn new() -> Self {
        let mut world = World::new();

        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut map =
            ChunkedPerlinMap::new(MAP_WIDTH, CHUNK_SIZE, 0.01, rand::Rng::gen(&mut rng), 1.0);

        // Setup the mesh manager
        let mut mesh_mgr = MeshManager::new();
        let quad_mesh = mesh_mgr.add_mesh(Mesh::from_obj(QUAD_DATA), Some("quad"));
        let cube_mesh = mesh_mgr.add_mesh(Mesh::from_obj(CUBE_DATA), Some("cube"));
        mesh_mgr.add_mesh(Mesh::from_obj(CONE_DATA), Some("tree"));
        mesh_mgr.add_mesh(Mesh::from_obj(BUSH_DATA), Some("bush"));

        // Setup the texture manager
        let mut texture_mgr = TextureManager::new();
        let grass_texture = texture_mgr.add_texture(Texture::from_png("grass.png"), "grass");
        let water_texture = texture_mgr.add_texture(Texture::from_png("water.png"), "water");
        texture_mgr.add_texture(Texture::from_png("tree.png"), "tree");
        texture_mgr.add_texture(Texture::from_png("rock.png"), "rock");

        let mut bvh = BVH::<Entity>::new();

        let spawn_point =
            nalgebra_glm::vec3(MAP_WIDTH as f32 / 2.0 + 1.0, MAP_WIDTH as f32 / 2.0, 2.5);
        loop {
            if map.chunkless_height(spawn_point.xy()) > 0.74 {
                break;
            }
            map = ChunkedPerlinMap::new(MAP_WIDTH, CHUNK_SIZE, 0.01, rand::Rng::gen(&mut rng), 1.0);
        }

        // Add player
        let scale_vec = nalgebra_glm::vec3(0.2, 0.2, 1.0);
        let player_entity = world.spawn((ModelComponent::new(
            cube_mesh,
            grass_texture,
            spawn_point,
            scale_vec,
        ),));
        let player_node_id = bvh.insert(
            player_entity,
            mesh_mgr
                .get_mesh("cube")
                .unwrap()
                .aabb
                .scale(scale_vec)
                .translate(spawn_point),
        );
        println!(
            "{:?}",
            mesh_mgr
                .get_mesh("cube")
                .unwrap()
                .aabb
                .scale(scale_vec)
                .translate(spawn_point)
        );
        world
            .insert(
                player_entity,
                (Player {
                    bvh_node_id: player_node_id,
                },),
            )
            .unwrap();

        // Add water plane
        let scale_vec = nalgebra_glm::vec3(MAP_WIDTH as f32, MAP_WIDTH as f32, MAP_WIDTH as f32);
        let water_entity = world.spawn((ModelComponent::new(
            quad_mesh,
            water_texture,
            nalgebra_glm::vec3(0.0, 0.0, 0.5),
            scale_vec,
        ),));
        bvh.insert(
            water_entity,
            mesh_mgr
                .get_mesh("quad")
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
            texture_mgr,
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
                MAP_WIDTH as i32,
            ),
            outline_program: create_program(
                include_str!("../shaders/3d.vert"),
                include_str!("../shaders/3d-color.frag"),
            )
            .unwrap(),

            position: spawn_point,
            velocity: nalgebra_glm::vec3(0.0, 0.0, 0.0),

            prev_space_state: false,
            debug: false,
            update_swap: 0,
        }
    }

    fn update_view(&mut self, app: &App) {
        let mut player_entt: Option<Entity> = None;
        for (entt, _) in &mut self.world.query::<&Player>() {
            player_entt = Some(entt);
            break;
        }
        let zoom = 1.0;

        let curr_w_state = app.keys[Scancode::W as usize];
        let curr_s_state = app.keys[Scancode::S as usize];
        let curr_a_state = app.keys[Scancode::A as usize];
        let curr_d_state = app.keys[Scancode::D as usize];
        let curr_space_state = app.keys[Scancode::Space as usize];
        let walking = curr_w_state || curr_s_state || curr_a_state || curr_d_state;
        let walk_speed: f32 = 1.6 * 2.5 * zoom;
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
        self.position.z = self.map.height_interpolated(self.position.xy());

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
            .set_position(self.position + nalgebra_glm::vec3(13.85, 0.0, 8.00) * zoom);
        self.open_gl
            .camera
            .set_lookat(self.position + nalgebra_glm::vec3(0.0, 0.0, 1.00));
    }

    fn update_clickers(&mut self, app: &App) {
        if app.mouse_left_clicked {
            println!("{:?} {:?}", app.mouse_x, app.mouse_y);
        }
        let inv_aspect = app.window_size.y as f32 / app.window_size.x as f32;
        let ndc_x = (2.0 * app.mouse_x as f32) / app.window_size.x as f32 - 1.0;
        let ndc_y = 1.0
            - (2.0
                * (app.mouse_y as f32 * inv_aspect
                    + app.window_size.y as f32 * (1.0 - inv_aspect)))
                / app.window_size.y as f32;

        let clip_coordinates = nalgebra_glm::vec4(ndc_x, ndc_y, -0.0, 1.0);

        let (inv_proj, inv_view) = self.open_gl.camera.inv_proj_and_view();
        let mut eye_coords = inv_proj * clip_coordinates;
        eye_coords /= eye_coords.w;

        let world_coords = inv_view * eye_coords;
        let dir = (world_coords.xyz() - self.open_gl.camera.position()).normalize();

        let ray = Ray {
            dir,
            origin: self.open_gl.camera.position(),
        };

        // Set all outlines to false
        for (_, model) in &mut self.world.query::<&mut ModelComponent>() {
            model.outlined = false;
        }

        // Set hovered outlines to true
        let hovereds: Vec<Entity> = self
            .bvh
            .iter_ray(&ray)
            .filter(|entity| self.world.get::<&Rock>(*entity).is_ok())
            .collect();
        for entity in hovereds {
            self.world
                .get::<&mut ModelComponent>(entity)
                .unwrap()
                .outlined = true;
            if app.mouse_left_clicked {
                println!("{:?}", entity);
            }
        }
    }
}
