use std::cmp::max;

use super::{
    frustrum::{self, Frustrum},
    plane::Plane,
    ray::Ray,
    sphere::Sphere,
};

#[derive(Debug, Copy, Clone)]
#[allow(unused)]
pub struct AABB {
    pub min: nalgebra_glm::Vec3,
    pub max: nalgebra_glm::Vec3,
}

#[allow(unused)]
impl AABB {
    pub fn new() -> Self {
        Self {
            min: nalgebra_glm::vec3(f32::MAX, f32::MAX, f32::MAX),
            max: nalgebra_glm::vec3(f32::MIN, f32::MIN, f32::MIN),
        }
    }

    pub fn from_min_max(min: nalgebra_glm::Vec3, max: nalgebra_glm::Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: impl IntoIterator<Item = nalgebra_glm::Vec3>) -> Self {
        let mut retval = AABB::new();
        retval.expand_to_fit(points);
        retval
    }

    pub fn union(&self, b: AABB) -> AABB {
        AABB::from_min_max(
            nalgebra_glm::min2(&self.min, &b.min),
            nalgebra_glm::max2(&self.max, &b.max),
        )
    }

    pub fn area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    pub fn within_frustrum(&self, frustrum: &Frustrum, debug: bool) -> bool {
        let mut i = 0;
        for plane in frustrum.planes() {
            let vmax = self.get_furthest_corner(plane);
            let value = plane.normal().dot(&vmax) + plane.dist();
            // println!("{}", value);
            if value < 0.0 {
                let bounding_sphere = self.bounding_sphere();
                let in_sphere = bounding_sphere.within_frustrum(frustrum);
                if !in_sphere && debug {
                    println!(
                        "i:{}\nmax:{:?}\nmin:{:?}\nplane:{:?}\nsphere:{:?}",
                        i,
                        self.max,
                        self.min,
                        plane,
                        self.bounding_sphere()
                    );
                }
                return in_sphere;
            }
            i += 1;
        }
        true
    }

    pub fn within_sphere(&self, sphere: &Sphere) -> bool {
        let mut radius_squared = sphere.radius.powf(2.0);

        if (sphere.center.x < self.min.x) {
            radius_squared -= (sphere.center.x - self.min.x).powf(2.0);
        }
        if (sphere.center.x > self.max.x) {
            radius_squared -= (sphere.center.x - self.max.x).powf(2.0);
        }
        if (sphere.center.y < self.min.y) {
            radius_squared -= (sphere.center.y - self.min.y).powf(2.0);
        }
        if (sphere.center.y > self.max.y) {
            radius_squared -= (sphere.center.y - self.max.y).powf(2.0);
        }
        if (sphere.center.z < self.min.z) {
            radius_squared -= (sphere.center.z - self.min.z).powf(2.0);
        }
        if (sphere.center.z > self.max.z) {
            radius_squared -= (sphere.center.z - self.max.z).powf(2.0);
        }

        radius_squared > 0.0
    }

    pub fn raycast(&self, ray: &Ray) -> bool {
        let t_min = self.min - ray.origin;
        let t_max = self.max - ray.origin;
        let t1 = nalgebra_glm::min2(&t_min, &t_max);
        let t2 = nalgebra_glm::max2(&t_min, &t_max);
        let t_near = f32::max(f32::max(t1.x, t1.y), t1.z);
        let t_far = f32::min(f32::min(t1.x, t1.y), t1.z);
        t_near < t_far
    }

    pub fn bounding_sphere(&self) -> Sphere {
        let center = self.center();
        Sphere::new(center, nalgebra_glm::distance(&center, &self.min))
    }

    pub fn center(&self) -> nalgebra_glm::Vec3 {
        (self.max + self.min) * 0.5
    }

    pub fn translate(&self, center: nalgebra_glm::Vec3) -> Self {
        Self {
            min: self.min + center,
            max: self.max + center,
        }
    }

    pub fn scale(&self, factor: nalgebra_glm::Vec3) -> Self {
        Self {
            min: self.min.component_mul(&factor),
            max: self.max.component_mul(&factor),
        }
    }

    pub fn expand_to_fit(&mut self, points: impl IntoIterator<Item = nalgebra_glm::Vec3>) {
        for corner in points.into_iter() {
            self.min = nalgebra_glm::min2(&self.min, &corner.xyz());
            self.max = nalgebra_glm::max2(&self.max, &corner.xyz());
        }
    }

    pub fn pos_z_plane_midpoint(&self) -> nalgebra_glm::Vec4 {
        let bottom_left = nalgebra_glm::vec4(self.min.x, self.min.y, self.max.z, 1.0);
        let top_right = nalgebra_glm::vec4(self.max.x, self.max.y, self.max.z, 1.0);
        0.5 * (bottom_left + top_right)
    }

    pub fn transform(&mut self, matrix: nalgebra_glm::Mat4) {
        self.min = (matrix * nalgebra_glm::vec4(self.min.x, self.min.y, self.min.z, 1.0)).xyz();
        self.max = (matrix * nalgebra_glm::vec4(self.max.x, self.max.y, self.max.z, 1.0)).xyz();
    }

    pub fn intersect_z(&mut self, other: &AABB) {
        self.min.z = self.min.z.min(other.min.z);
        self.max.z = self.max.z.max(other.max.z);
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        // Check for separation in the x-axis
        if self.max.x < other.min.x || self.min.x > other.max.x {
            return false;
        }
        // Check for separation in the y-axis
        if self.max.y < other.min.y || self.min.y > other.max.y {
            return false;
        }
        // Check for separation in the z-axis
        if self.max.z < other.min.z || self.min.z > other.max.z {
            return false;
        }

        // No separation found, the AABBs intersect
        true
    }

    pub fn contains(&self, other: &AABB) -> bool {
        let mut result = true;
        result = result && self.min.x <= other.min.x;
        result = result && self.min.y <= other.min.y;
        result = result && self.min.z <= other.min.z;
        result = result && other.max.x <= self.max.x;
        result = result && other.max.y <= self.max.y;
        result = result && other.max.z <= self.max.z;
        result
    }

    pub fn corners(&self) -> [nalgebra_glm::Vec3; 8] {
        [
            nalgebra_glm::Vec3::new(self.min.x, self.min.y, self.min.z),
            nalgebra_glm::Vec3::new(self.max.x, self.min.y, self.min.z),
            nalgebra_glm::Vec3::new(self.min.x, self.max.y, self.min.z),
            nalgebra_glm::Vec3::new(self.max.x, self.max.y, self.min.z),
            nalgebra_glm::Vec3::new(self.min.x, self.min.y, self.max.z),
            nalgebra_glm::Vec3::new(self.max.x, self.min.y, self.max.z),
            nalgebra_glm::Vec3::new(self.min.x, self.max.y, self.max.z),
            nalgebra_glm::Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    fn get_furthest_corner(&self, plane: &Plane) -> nalgebra_glm::Vec3 {
        nalgebra_glm::vec3(
            if plane.normal().x > 0.0 {
                self.max.x
            } else {
                self.min.x
            },
            // Y axis
            if plane.normal().y > 0.0 {
                self.max.y
            } else {
                self.min.y
            },
            // Z axis
            if plane.normal().z > 0.0 {
                self.max.z
            } else {
                self.min.z
            },
        )
    }
}
