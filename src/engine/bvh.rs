use std::{array::IntoIter, cmp::max};

use obj::Obj;
use rand::{rngs::StdRng, Rng, SeedableRng};

use super::{aabb::AABB, frustrum::Frustrum, ray::Ray, sphere::Sphere};

/// A Bounding Volume Hierarchy implementation. I'd recommend using BVH<Entity> if using hecs, for faster iteration.
pub struct BVH<Object: Copy + Clone> {
    nodes: Vec<BVHNode<Object>>,
    root_id: BVHNodeId,
    rng: StdRng,
}

pub type BVHNodeId = u32;

pub const INVALID_BVH_NODE_ID: BVHNodeId = !0u32;

// TODO: Implement ray tracing query
// TODO: Implement removing elements
// TODO: Implement proxy volumes and movement
struct BVHNode<Object: Copy + Clone> {
    volume: AABB, // TODO: Could this be generalized?
    left: BVHNodeId,
    right: BVHNodeId,
    parent_id: BVHNodeId,
    object: Option<Object>,
    height: i32,
}

pub struct BVHFrustrumIterator<'a, Object: Copy + Clone> {
    bvh: &'a BVH<Object>, // Reference to the tree
    frustrum: &'a Frustrum,
    stack: Vec<BVHNodeId>,
    debug: bool,
}

pub struct BVHSphereIterator<'a, Object: Copy + Clone> {
    bvh: &'a BVH<Object>, // Reference to the tree
    sphere: &'a Sphere,
    stack: Vec<BVHNodeId>,
}

pub struct BVHRayIterator<'a, Object: Copy + Clone> {
    bvh: &'a BVH<Object>, // Reference to the tree
    ray: &'a Ray,
    stack: Vec<BVHNodeId>,
}

impl<Object: Copy + Clone> BVH<Object> {
    const AABB_EXTENSION: f32 = 0.1;
    const AABB_MULTIPLIER: f32 = 2.0;

    pub fn new() -> Self {
        Self {
            nodes: vec![],
            root_id: INVALID_BVH_NODE_ID,
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    pub fn insert(&mut self, object: Object, aabb: AABB) -> BVHNodeId {
        let proxy_id = self.allocate_node(object, aabb);

        // Fatten the aabb.
        let r = nalgebra_glm::vec3(
            Self::AABB_EXTENSION,
            Self::AABB_EXTENSION,
            Self::AABB_EXTENSION,
        );
        let mut new_aabb = self.get_volume(proxy_id);
        new_aabb.min -= r;
        new_aabb.max += r;
        self.set_volume(proxy_id, new_aabb);

        self.insert_leaf(proxy_id);

        proxy_id
    }

    pub fn remove(&mut self, node_id: BVHNodeId) {
        assert!((node_id as usize) < self.nodes.len());
        assert!(self.node_at(node_id).is_leaf());

        self.remove_leaf(node_id);
        // TODO: Free the node id
    }

    pub fn move_obj(
        &mut self,
        proxy_id: BVHNodeId,
        aabb: &AABB,
        displacement: &nalgebra_glm::Vec3,
    ) -> bool {
        assert!((proxy_id as usize) < self.nodes.len());
        assert!(self.node_at(proxy_id).is_leaf());

        if self.get_volume(proxy_id).intersects(aabb) {
            return false;
        }

        self.remove_leaf(proxy_id);

        // Extend AABB.
        let mut new_aabb = *aabb;
        let r = nalgebra_glm::vec3(
            Self::AABB_EXTENSION,
            Self::AABB_EXTENSION,
            Self::AABB_EXTENSION,
        );
        new_aabb.min -= r;
        new_aabb.max += r;

        // Predict AABB displacement.
        let d = Self::AABB_MULTIPLIER * displacement;

        if d.x < 0.0 {
            new_aabb.min.x += d.x;
        } else {
            new_aabb.max.x += d.x;
        }
        if d.y < 0.0 {
            new_aabb.min.y += d.y;
        } else {
            new_aabb.max.y += d.y;
        }
        if d.z < 0.0 {
            new_aabb.min.z += d.z;
        } else {
            new_aabb.max.z += d.z;
        }
        self.set_volume(proxy_id, new_aabb);

        self.insert_leaf(proxy_id);

        true
    }

    fn insert_leaf(&mut self, new_node: BVHNodeId) {
        if new_node == self.root_id {
            return;
        }

        // Find the best sibling for the new leaf
        let aabb = self.get_volume(new_node);
        let mut index = self.root_id;
        let mut loop_counter = 0;
        while !self.node_at(index).is_leaf() {
            loop_counter += 1;
            let left = self.node_at(index).left;
            let right = self.node_at(index).right;
            let area = self.node_at(index).volume.area();

            let combined_aabb = self.node_at(index).volume.union(aabb);
            let combined_area = combined_aabb.area();

            // Cost of creating a new parent for this node and the new leaf
            let cost = 2.0 * combined_area;

            // Minimum cost of pushing the leaf further down the tree
            let inheritance_cost = 2.0 * (combined_area - area);

            // Cost of descending into left child
            let left_cost = if self.node_at(left).is_leaf() {
                aabb.union(self.node_at(left).volume).area()
            } else {
                let aabb2 = aabb.union(self.node_at(left).volume);
                let old_area = self.node_at(left).volume.area();
                let new_area = aabb2.area();
                (new_area - old_area)
            } + inheritance_cost;

            // Cost of descending into right child
            let right_cost = if self.node_at(right).is_leaf() {
                aabb.union(self.node_at(right).volume).area()
            } else {
                let aabb2 = aabb.union(self.node_at(right).volume);
                let old_area = self.node_at(right).volume.area();
                let new_area = aabb2.area();
                (new_area - old_area)
            } + inheritance_cost;

            // Descend according to the minimum cost
            if (cost < left_cost && cost < right_cost) {
                break;
            }

            // Descend
            if (left_cost < right_cost) {
                index = left;
            } else {
                index = right;
            }
        }
        let best_sibling: BVHNodeId = index;

        // Create a new parent
        let old_parent = self.node_at(best_sibling).parent_id;
        let new_parent = self.new_internal_node(
            old_parent,
            self.node_at(best_sibling).height + 1,
            new_node,
            best_sibling,
        );
        if (old_parent != INVALID_BVH_NODE_ID) {
            // The sibling was not the root
            if (self.node_at(old_parent).left == best_sibling) {
                self.node_at_mut(old_parent).left = new_parent;
            } else {
                assert_eq!(self.node_at_mut(old_parent).right, best_sibling);
                self.node_at_mut(old_parent).right = new_parent;
            }
        } else {
            // The sibling was the root
            self.root_id = new_parent
        }
        self.node_at_mut(best_sibling).parent_id = new_parent;
        self.node_at_mut(new_node).parent_id = new_parent;

        self.adjust_bounds(new_parent);
    }

    pub fn remove_leaf(&mut self, leaf: BVHNodeId) {
        if leaf == self.root_id {
            self.root_id = INVALID_BVH_NODE_ID;
            return;
        }

        let parent = self.get_parent_id(leaf);
        let grand_parent = self.get_parent_id(parent);
        let sibling = if self.get_left(parent) == leaf {
            self.get_right(parent)
        } else {
            self.get_left(parent)
        };

        if grand_parent != INVALID_BVH_NODE_ID {
            // Destroy parent and connect sibling to grand parent
            if self.get_left(grand_parent) == parent {
                self.set_left(grand_parent, sibling);
            } else {
                self.set_right(grand_parent, sibling);
            }
            self.set_parent(sibling, grand_parent);
            // TODO: Implement free-list, add parent to free-list
            self.adjust_bounds(grand_parent);
        } else {
            self.root_id = sibling;
            self.set_parent(leaf, INVALID_BVH_NODE_ID);
            // TODO: Implement free-list, add parent to free-list
        }
    }

    pub fn iter_frustrum<'a>(
        &'a self,
        frustrum: &'a Frustrum,
        debug: bool,
    ) -> BVHFrustrumIterator<'a, Object> {
        let mut stack = Vec::new();

        if self.root_id != INVALID_BVH_NODE_ID {
            stack.push(self.root_id);
        }

        BVHFrustrumIterator {
            bvh: self,
            frustrum,
            stack,
            debug,
        }
    }

    pub fn iter_sphere<'a>(&'a self, sphere: &'a Sphere) -> BVHSphereIterator<'a, Object> {
        let mut stack = Vec::new();

        if self.root_id != INVALID_BVH_NODE_ID {
            stack.push(self.root_id);
        }

        BVHSphereIterator {
            bvh: self,
            sphere,
            stack,
        }
    }

    pub fn iter_ray<'a>(&'a self, ray: &'a Ray) -> BVHRayIterator<'a, Object> {
        let mut stack = Vec::new();

        if self.root_id != INVALID_BVH_NODE_ID {
            stack.push(self.root_id);
        }

        BVHRayIterator {
            bvh: self,
            ray,
            stack,
        }
    }

    pub fn walk_tree(&self) {
        let mut stack = vec![];
        stack.push(self.root_id);

        while stack.len() > 0 {
            let index = stack.pop().unwrap();
            let node = self.node_at(index);
            println!("{} [label=\"{:?}\"]", index, node.volume);

            if node.left != INVALID_BVH_NODE_ID {
                println!("{} -> {}", index, node.left);
                stack.push(node.left);
            }
            if node.right != INVALID_BVH_NODE_ID {
                println!("{} -> {}", index, node.right);
                stack.push(node.right);
            }
        }
        println!("\n");
    }

    fn allocate_node(&mut self, object: Object, aabb: AABB) -> BVHNodeId {
        let node_index = self.nodes.len() as u32;
        let new_node = BVHNode::<Object> {
            volume: aabb,
            left: INVALID_BVH_NODE_ID,
            right: INVALID_BVH_NODE_ID,
            parent_id: INVALID_BVH_NODE_ID,
            object: Some(object),
            height: 0,
        };
        self.nodes.push(new_node);
        if self.root_id == INVALID_BVH_NODE_ID {
            self.root_id = node_index
        }
        node_index
    }

    fn new_internal_node(
        &mut self,
        parent: BVHNodeId,
        height: i32,
        left_id: BVHNodeId,
        right_id: BVHNodeId,
    ) -> BVHNodeId {
        let node_index = self.nodes.len() as u32;
        let x = self.rng.gen_range(0.0..1.0);
        let new_node = BVHNode::<Object> {
            volume: AABB::new(),
            left: if x < 0.5 { left_id } else { right_id },
            right: if x < 0.5 { right_id } else { left_id },
            parent_id: parent,
            object: None,
            height,
        };
        self.nodes.push(new_node);
        node_index
    }

    fn adjust_bounds(&mut self, mut index: BVHNodeId) {
        while index != INVALID_BVH_NODE_ID {
            let left = self.get_left(index);
            let right = self.get_right(index);

            assert_ne!(left, INVALID_BVH_NODE_ID);
            assert_ne!(right, INVALID_BVH_NODE_ID);

            let left_volume = self.get_volume(left);
            let right_volume = self.get_volume(right);
            self.node_at_mut(index).volume = left_volume.union(right_volume);

            index = self.get_parent_id(index);
        }
    }

    fn node_at(&self, id: BVHNodeId) -> &BVHNode<Object> {
        &self.nodes[id as usize]
    }

    fn node_at_mut(&mut self, id: BVHNodeId) -> &mut BVHNode<Object> {
        &mut self.nodes[id as usize]
    }

    fn get_left(&self, id: BVHNodeId) -> BVHNodeId {
        self.node_at(id).left
    }

    fn set_left(&mut self, id: BVHNodeId, left: BVHNodeId) {
        self.node_at_mut(id).left = left
    }

    fn get_right(&self, id: BVHNodeId) -> BVHNodeId {
        self.node_at(id).right
    }

    fn set_right(&mut self, id: BVHNodeId, right: BVHNodeId) {
        self.node_at_mut(id).right = right
    }

    fn get_parent_id(&self, id: BVHNodeId) -> BVHNodeId {
        self.node_at(id).parent_id
    }

    fn set_parent(&mut self, id: BVHNodeId, parent_id: BVHNodeId) {
        self.node_at_mut(id).parent_id = parent_id
    }

    fn get_volume(&self, id: BVHNodeId) -> AABB {
        self.node_at(id).volume
    }

    fn set_volume(&mut self, id: BVHNodeId, volume: AABB) {
        self.node_at_mut(id).volume = volume
    }

    fn get_height(&self, id: BVHNodeId) -> i32 {
        self.node_at(id).height
    }

    fn set_height(&mut self, id: BVHNodeId, height: i32) {
        self.node_at_mut(id).height = height
    }
}

impl<Object: Copy + Clone> BVHNode<Object> {
    fn is_leaf(&self) -> bool {
        self.left == INVALID_BVH_NODE_ID
    }
}

impl<'a, Object: Copy + Clone> Iterator for BVHFrustrumIterator<'a, Object> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_id) = self.stack.pop() {
            let current_node = self.bvh.node_at(current_id);
            if !current_node
                .volume
                .within_frustrum(self.frustrum, self.debug)
            {
                continue;
            }

            if current_node.left != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.left);
            }
            if current_node.right != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.right);
            }
            if let Some(object) = current_node.object {
                return Some(object);
            }
        }
        None
    }
}

impl<'a, Object: Copy + Clone> Iterator for BVHSphereIterator<'a, Object> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_id) = self.stack.pop() {
            let current_node = self.bvh.node_at(current_id);
            if !current_node.volume.within_sphere(self.sphere) {
                continue;
            }

            if current_node.left != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.left);
            }
            if current_node.right != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.right);
            }
            if let Some(object) = current_node.object {
                return Some(object);
            }
        }
        None
    }
}

impl<'a, Object: Copy + Clone> Iterator for BVHRayIterator<'a, Object> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_id) = self.stack.pop() {
            let current_node = self.bvh.node_at(current_id);
            if !current_node.volume.raycast(self.ray) {
                continue;
            }

            if current_node.left != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.left);
            }
            if current_node.right != INVALID_BVH_NODE_ID {
                self.stack.push(current_node.right);
            }
            if let Some(object) = current_node.object {
                return Some(object);
            }
        }
        None
    }
}
