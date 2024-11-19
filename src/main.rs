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
// - [x] "feeling" bar that gives the most pressing matter
//     - [x] gui quads
//     - [x] font cacheing
//     - [x] hunger bar
//         - [x] 9-slices
// - [ ] wild berries that appear on bushes
//     - [ ] two "hand" inventory menus, with the right-one being the dominant one
//         - [ ] a button to swap between them
//         - [ ] a small icon showing them in the hand inventory
//         - [ ] can drop them with (q)
//     - [ ] clicking and holding items puts them into your hand
//         - [ ] when you hover over something, it highlights it with an action billboard that's like "grab X"
//     - [ ] can eat them once they're in your hand
// - [ ] can go up to a body of water, click, and drink
// - [ ] wild vegetables/grains (potatoes/wheat). can plant them into tilled land, have them grow, pick them.
// - [ ] wild animals
//     - [ ] killing them drops meat and hide
//
// ## Crafting and building
// - [ ] press (c) to craft using the two items in hand
//     - [ ] (stone, stone) => knapping menu
//     - [ ] (clay, X) => molding menu
//     - [ ] (stone tool blank, stick) -> (stone tool, -)
// - [ ] can use tools by clicking, with the tool in your dominant hand
// - [ ] can press (b) to open up list of structures you can build, then you can place a blueprint. blueprints must be given the right materials, then they're built.
//     - [ ] firepit
//     - [ ] thatch lean-to

fn main() -> Result<(), String> {
    run(
        nalgebra_glm::I32Vec2::new(800, 600),
        "Survival Prototype",
        &|app| RefCell::new(Box::new(Gameplay::new(app))),
    )
}
