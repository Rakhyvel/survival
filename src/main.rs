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
//     - [x] gui quads
//     - [x] font cacheing
//     - [ ] hunger bar
//         - [ ] 9-slices
// - [ ] wild berries that appear on bushes
//     - [ ] clicking and holding puts them into your hand
//         > maybe when you hover over a bush, it highlights the bush with an action billboard that's like "(p) Pick berry", and pressing p picks the berry
//     - [ ] can eat them once they're in your hand
// - [ ] can go up to a body of water, click, and drink
// - [ ] wild vegetables (potatoes). can plant them into tilled land, have them grow, pick them.
//
// ## Crafting and building
// - [ ] can take two stones, click them together (?), you get a list of stone tools you can make
// - [ ] can use tools by clicking, with the tool in your dominant hand
// - [ ] can press (b) to open up list of structures you can build, then you can place a blueprint. blueprints must be given the right materials, then they're built.

fn main() -> Result<(), String> {
    run(
        nalgebra_glm::I32Vec2::new(800, 600),
        "Survival Prototype",
        &|app| RefCell::new(Box::new(Gameplay::new(app))),
    )
}
