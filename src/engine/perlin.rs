use std::cmp::Ordering;

use rand::{Rng, SeedableRng};

static HASH: [u8; 256] = [
    208, 34, 231, 213, 32, 248, 233, 56, 161, 78, 24, 140, 71, 48, 140, 254, 245, 255, 247, 247,
    40, 185, 248, 251, 245, 28, 124, 204, 204, 76, 36, 1, 107, 28, 234, 163, 202, 224, 245, 128,
    167, 204, 9, 92, 217, 54, 239, 174, 173, 102, 193, 189, 190, 121, 100, 108, 167, 44, 43, 77,
    180, 204, 8, 81, 70, 223, 11, 38, 24, 254, 210, 210, 177, 32, 81, 195, 243, 125, 8, 169, 112,
    32, 97, 53, 195, 13, 203, 9, 47, 104, 125, 117, 114, 124, 165, 203, 181, 235, 193, 206, 70,
    180, 174, 0, 167, 181, 41, 164, 30, 116, 127, 198, 245, 146, 87, 224, 149, 206, 57, 4, 192,
    210, 65, 210, 129, 240, 178, 105, 228, 108, 245, 148, 140, 40, 35, 195, 38, 58, 65, 207, 215,
    253, 65, 85, 208, 76, 62, 3, 237, 55, 89, 232, 50, 217, 64, 244, 157, 199, 121, 252, 90, 17,
    212, 203, 149, 152, 140, 187, 234, 177, 73, 174, 193, 100, 192, 143, 97, 53, 145, 135, 19, 103,
    13, 90, 135, 151, 199, 91, 239, 247, 33, 39, 145, 101, 120, 99, 3, 186, 86, 99, 41, 237, 203,
    111, 79, 220, 135, 158, 42, 30, 154, 120, 67, 87, 167, 135, 176, 183, 191, 253, 115, 184, 21,
    233, 58, 129, 233, 142, 39, 128, 211, 118, 137, 139, 255, 114, 20, 218, 113, 154, 27, 127, 246,
    250, 1, 8, 198, 250, 209, 92, 222, 173, 21, 88, 102, 219,
];

#[derive(Default)]
pub struct PerlinMap {
    cells: Vec<Cell>,
    map_width: usize,
}

#[derive(Default, Copy, Clone)]
struct Cell {
    pub height: f32,
    pub flow: f32,
}

struct Particle {
    pub age: usize,

    pub pos: nalgebra_glm::Vec3,
    pub vel: nalgebra_glm::Vec3,

    pub volume: f32,   // Total particle volume
    pub sediment: f32, // Fraction of volume that is sediment
    pub rain: f32,     // Separate
}

pub trait HeightMap {
    fn height_nearest(&self, p: nalgebra_glm::Vec2) -> f32;
    fn height_interpolated(&self, p: nalgebra_glm::Vec2) -> f32;
    fn normal(&self, p: nalgebra_glm::Vec2) -> nalgebra_glm::Vec3;
    fn flow(&self, p: nalgebra_glm::Vec2) -> f32;
}

impl Particle {
    fn new(pos: nalgebra_glm::Vec3) -> Self {
        Self {
            age: 0,
            pos,
            vel: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            volume: 1.0,
            sediment: 0.0,
            rain: 0.0,
        }
    }

    fn descend(&mut self, map: &mut PerlinMap) -> bool {
        loop {
            const MIN_VOLUME: f32 = 0.01;
            const DEPOSITION_RATE: f32 = 0.1;
            const EVAPORATION_RATE: f32 = 0.1;
            const MAX_AGE: usize = 100;
            const ENTRAINMENT: f32 = 1.0;

            if self.age > MAX_AGE {
                break;
            }

            let grad = map.get_normal(self.pos.xy());

            // Accelerate particle using classical mechanics
            let old_pos = self.pos;
            self.vel += grad / self.volume;
            self.vel += nalgebra_glm::vec3(0.0, 0.1, 0.0);
            if nalgebra_glm::length(&self.vel) > 0.0 {
                self.vel = (2.0 as f32).sqrt() * nalgebra_glm::normalize(&self.vel);
            }
            self.vel.z -= 1.0; // gravity
            self.pos += self.vel;
            let mut flying = false;
            if self.pos.z < map.height(self.pos.xy()) {
                self.pos.z = map.height(self.pos.xy());
                flying = false;
            }

            // Update flow, momentum
            if !flying {
                if self.pos.z >= 0.5 {
                    map.incr_flow(old_pos.xy(), self.rain);
                } else {
                    self.rain = 1.0;
                }
            }

            self.rain *= 1.0 - EVAPORATION_RATE;

            self.age += 1;
        }
        false
    }
}

impl PerlinMap {
    pub fn new(map_width: usize) -> Self {
        let mut retval = Self::default();

        retval.map_width = map_width;

        retval
    }

    pub fn generate(
        &mut self,
        level_of_detail: f32,
        depth: i32,
        seed: i32,
        amplitude: f32,
        offset: nalgebra_glm::Vec2,
    ) {
        assert!(self.cells.len() == 0);
        for y in 0..self.map_width {
            for x in 0..self.map_width {
                self.cells.push(Cell {
                    height: perlin2d(
                        x as f32 + offset.x,
                        y as f32 + offset.y,
                        level_of_detail,
                        depth,
                        seed,
                    ) * amplitude,
                    flow: 0.0,
                });
            }
        }
    }

    pub fn erode(&mut self, total_particles: usize, seed: u64) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let mut checkpoint = total_particles / 10;
        for i in 0..total_particles {
            if i > checkpoint {
                checkpoint += total_particles / 10;
                // println!(
                //     " - {}%",
                //     (i as f32 / total_particles as f32 * 100.0) as usize
                // );
            }

            let mut drop = Particle::new(nalgebra_glm::vec3(
                rng.gen_range(0.0..self.map_width as f32),
                rng.gen_range(0.0..self.map_width as f32),
                10.0,
            ));
            if self.height(drop.pos.xy()) < 0.5 {
                continue;
            }
            while drop.descend(self) {}
        }
    }

    pub fn cascade(&mut self, pos: nalgebra_glm::Vec2) {
        const MAX_DIFF: f32 = 0.9;
        const SETTLING: f32 = 0.8;

        let neighbors = [
            nalgebra_glm::vec2(-1.0, -1.0),
            nalgebra_glm::vec2(-1.0, 0.0),
            nalgebra_glm::vec2(-1.0, 1.0),
            nalgebra_glm::vec2(0.0, -1.0),
            nalgebra_glm::vec2(0.0, 1.0),
            nalgebra_glm::vec2(1.0, -1.0),
            nalgebra_glm::vec2(1.0, 0.0),
            nalgebra_glm::vec2(1.0, 1.0),
        ];

        let mut in_bound_neighbors: Vec<nalgebra_glm::Vec3> = vec![];
        for neighbor in neighbors {
            let npos = neighbor + pos;
            if self.oob(npos) {
                continue;
            } else {
                in_bound_neighbors.push(nalgebra_glm::vec3(npos.x, npos.y, self.height(npos)))
            }
        }

        in_bound_neighbors.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(Ordering::Greater));

        // Iterate over all sorted neighbors
        for i in 0..in_bound_neighbors.len() {
            let npos = in_bound_neighbors[i];

            // Full height-different between positions
            let diff = self.height(pos) - in_bound_neighbors[i].z;
            if diff == 0.0 {
                continue;
            }

            // The amount of excess difference
            let excess = if in_bound_neighbors[i].z > 0.1 {
                diff.abs() - MAX_DIFF
            } else {
                diff.abs()
            };
            if excess <= 0.0 {
                continue;
            }

            // Actual amount transferred
            let transfer = SETTLING * excess / 2.0;

            // Cap by maximum transferrable amount
            if diff > 0.0 {
                self.incr_height(pos, -transfer);
                self.incr_height(npos.xy(), transfer);
            } else {
                self.incr_height(pos, transfer);
                self.incr_height(npos.xy(), -transfer);
            }
        }
    }

    pub fn height(&self, p: nalgebra_glm::Vec2) -> f32 {
        if self.oob(p) {
            return 0.0;
        }
        self.cells[p.x as usize + p.y as usize * self.map_width].height
    }

    fn incr_height(&mut self, p: nalgebra_glm::Vec2, val: f32) {
        if self.oob(p) {
            return;
        }
        self.cells[p.x as usize + p.y as usize * self.map_width].height += val
    }

    pub fn flow(&self, p: nalgebra_glm::Vec2) -> f32 {
        self.cells[p.x as usize + p.y as usize * self.map_width].flow
    }

    fn incr_flow(&mut self, p: nalgebra_glm::Vec2, val: f32) {
        if self.oob(p) {
            return;
        }
        self.cells[p.x as usize + p.y as usize * self.map_width].flow += val
    }

    pub fn get_z_interpolated(&self, p: nalgebra_glm::Vec2) -> f32 {
        assert!(!p.x.is_nan());
        // The coordinates of the tile's origin (bottom left corner)
        let origin = nalgebra_glm::floor(&p);

        // Coordinates inside the tile. [0,1]
        let offset = p - origin;

        let ray_origin = nalgebra_glm::vec3(p.x, p.y, 10000.0);
        let ray_direction = nalgebra_glm::vec3(0.0, 0.0, -1.0);

        let offsets = if offset.y <= 1.0 - offset.x {
            // In bottom triangle
            vec![
                nalgebra_glm::vec2(0.0, 0.0), // Contains the origin
                nalgebra_glm::vec2(1.0, 0.0),
                nalgebra_glm::vec2(0.0, 1.0),
            ]
        } else {
            // In top triangle
            vec![
                nalgebra_glm::vec2(1.0, 0.0),
                nalgebra_glm::vec2(1.0, 1.0), // Contains the anti-origin
                nalgebra_glm::vec2(0.0, 1.0),
            ]
        };
        let offsets: Vec<nalgebra_glm::Vec3> = offsets
            .iter()
            .map(|o| nalgebra_glm::vec3(origin.x + o.x, origin.y + o.y, self.height(origin + o)))
            .collect();

        let (retval, _t) = intersect(
            offsets[0],
            offsets[1],
            offsets[2],
            ray_origin,
            ray_direction,
        )
        .unwrap();
        retval.z
    }

    fn oob(&self, p: nalgebra_glm::Vec2) -> bool {
        p.x < 0.0 || p.y < 0.0 || p.x >= self.map_width as f32 || p.y >= self.map_width as f32
    }

    pub fn get_normal(&self, p: nalgebra_glm::Vec2) -> nalgebra_glm::Vec3 {
        assert!(!p.x.is_nan());
        // The coordinates of the tile's origin (bottom left corner)
        let origin = nalgebra_glm::floor(&p);

        // Coordinates inside the tile. [0,1]
        let offset = p - origin;

        let offsets = if offset.y <= 1.0 - offset.x {
            // In bottom triangle
            [
                nalgebra_glm::vec2(0.0, 0.0), // Contains the origin
                nalgebra_glm::vec2(1.0, 0.0),
                nalgebra_glm::vec2(0.0, 1.0),
            ]
        } else {
            // In top triangle
            [
                nalgebra_glm::vec2(1.0, 0.0),
                nalgebra_glm::vec2(1.0, 1.0), // Contains the anti-origin
                nalgebra_glm::vec2(0.0, 1.0),
            ]
        };
        let offsets: Vec<nalgebra_glm::Vec3> = offsets
            .iter()
            .map(|o| nalgebra_glm::vec3(origin.x + o.x, origin.y + o.y, self.height(origin + o)))
            .collect();

        tri_normal(offsets[0], offsets[1], offsets[2])
    }

    pub fn get_dot_prod(&self, p: nalgebra_glm::Vec2) -> f32 {
        assert!(!p.x.is_nan());

        nalgebra_glm::dot(&self.get_normal(p), &nalgebra_glm::vec3(0.0, 0.0, 1.0))
    }

    pub fn create_bulge(&mut self) {
        for y in 0..self.map_width {
            for x in 0..self.map_width {
                let z = self.cells[x + y * self.map_width].height;
                let xo = (x as f32) - (self.map_width as f32) / 2.0;
                let yo = (y as f32) - (self.map_width as f32) / 2.0;
                let d = ((xo * xo + yo * yo) as f32).sqrt();
                let shoreline = 0.5 * self.map_width as f32;
                let bulge = -(d - shoreline) / shoreline;
                self.cells[x + y * self.map_width].height = (((z - 0.5).powf(3.0) * 150.0) + 0.55);
            }
        }
    }

    pub fn create_shelf(&mut self, shelf_height: f32, shelf_depth: f32) {
        for y in 0..self.map_width {
            for x in 0..self.map_width {
                if self.cells[x + y * self.map_width].height <= shelf_height {
                    self.cells[x + y * self.map_width].height =
                        self.cells[x + y * self.map_width].height.min(shelf_depth)
                }
            }
        }
    }

    pub fn normalize(&mut self) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;

        for i in 0..self.cells.len() {
            min = self.cells[i].height.min(min);
            max = self.cells[i].height.max(max);
        }

        // stretch to min/max
        for i in 0..self.cells.len() {
            self.cells[i].height = (self.cells[i].height - min) / (max - min);
        }
    }
}

fn perlin2d(x: f32, y: f32, freq: f32, depth: i32, seed: i32) -> f32 {
    let mut xa = x * freq;
    let mut ya = y * freq;
    let mut amp: f32 = 1.0;
    let mut fin: f32 = 0.0;
    let mut div: f32 = 256.0;

    for _ in 0..depth {
        fin += noise2d(xa, ya, seed) * amp;
        xa *= 2.0;
        ya *= 2.0;
        amp *= 0.5;
        div += 256.0 * amp;
    }

    fin / div
}

fn noise2d(x: f32, y: f32, seed: i32) -> f32 {
    let x_int = x as i32;
    let y_int = y as i32;
    let x_frac: f32 = x - (x_int as f32);
    let y_frac: f32 = y - (y_int as f32);

    // Calculate noise values once
    let s = noise2(x_int, y_int, seed);
    let t = noise2(x_int + 1, y_int, seed);
    let u = noise2(x_int, y_int + 1, seed);
    let v = noise2(x_int + 1, y_int + 1, seed);

    // Calculate x smothing
    let low = smooth_inter(s, t, x_frac);
    let high = smooth_inter(u, v, x_frac);

    // Return y smoothing
    smooth_inter(low, high, y_frac)
}

fn noise2(x: i32, y: i32, seed: i32) -> f32 {
    let tmp = HASH[((y + seed) & 0xFF) as usize];
    HASH[((tmp as i32 + x) & 0xFF) as usize] as f32
}

#[inline]
fn smooth_inter(x: f32, y: f32, s: f32) -> f32 {
    lin_inter(x, y, s * s * (3.0 - 2.0 * s))
}

#[inline]
fn lin_inter(x: f32, y: f32, s: f32) -> f32 {
    x + s * (y - x)
}

fn intersect(
    v0: nalgebra_glm::Vec3,
    v1: nalgebra_glm::Vec3,
    v2: nalgebra_glm::Vec3,
    ray_origin: nalgebra_glm::Vec3,
    ray_direction: nalgebra_glm::Vec3,
) -> Option<(nalgebra_glm::Vec3, f32)> {
    const EPSILON: f32 = 0.0000001;
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = nalgebra_glm::cross(&ray_direction, &edge2);
    let a = nalgebra_glm::dot(&edge1, &h);

    if a.abs() < EPSILON {
        return None; // Ray is parallel to the triangle
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * nalgebra_glm::dot(&s, &h);

    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = nalgebra_glm::cross(&s, &edge1);
    let v = f * nalgebra_glm::dot(&ray_direction, &q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * nalgebra_glm::dot(&edge2, &q);

    let intersection_point = ray_origin + t * ray_direction;
    Some((intersection_point, t))
}

fn tri_normal(
    v0: nalgebra_glm::Vec3,
    v1: nalgebra_glm::Vec3,
    v2: nalgebra_glm::Vec3,
) -> nalgebra_glm::Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let normal = nalgebra_glm::cross(&edge1, &edge2).normalize();
    normal
}
