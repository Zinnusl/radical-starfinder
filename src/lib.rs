//! Radical Roguelike — WASM entry point.

use wasm_bindgen::prelude::*;

mod audio;
mod dungeon;
mod enemy;
mod game;
mod player;
mod radical;
mod render;
mod srs;
mod vocab;

#[cfg(feature = "console_error_panic_hook")]
fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();
}

#[wasm_bindgen]
pub fn start_game() -> Result<(), JsValue> {
    game::init_game()
}
