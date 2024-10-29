mod engine;
mod scenes;

use std::cell::RefCell;

use engine::app::*;
use scenes::gameplay::Gameplay;

// TODO:
// - [x] BVH terrain chunks
// - [x] Randomly placed trees, bushes, rocks
// - [x] Player which can move around
//
// ## Survival stats
// - [ ] "feeling" bar that gives the most pressing matter
//     - [ ] gui quads
//     - [ ] font cacheing
// - [ ] wild berries that appear on bushes
//     - [ ] clicking and holding puts them into your hand
//     - [ ] can eat them once they're in your hand
// - [ ] can go up to a body of water, click, and drink
//

fn main() -> Result<(), String> {
    run(
        nalgebra_glm::I32Vec2::new(800, 600),
        "Survival Prototype",
        &|_app| RefCell::new(Box::new(Gameplay::new())),
    )
}
