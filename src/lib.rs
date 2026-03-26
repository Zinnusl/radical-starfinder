//! Radical Starfinder — WASM entry point.

// is_multiple_of is unstable on MSVC toolchain used for coverage measurement
#![allow(clippy::manual_is_multiple_of)]

use wasm_bindgen::prelude::*;

mod achievement;
mod audio;
mod codex;
mod combat;
mod crucible;
mod dungeon;
mod enemy;
mod game;
mod particle;
mod player;
mod radical;
mod rarity;
mod render;
mod skill_tree;
mod sprites;
mod srs;
mod status;
mod vocab;
mod world;

fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        web_sys::console::error_1(&msg.clone().into());

        let _ = (|| -> Result<(), JsValue> {
            let win = web_sys::window().ok_or(JsValue::NULL)?;
            let doc = win.document().ok_or(JsValue::NULL)?;
            let canvas: web_sys::HtmlCanvasElement = doc
                .get_element_by_id("game-canvas")
                .ok_or(JsValue::NULL)?
                .dyn_into()?;
            let ctx: web_sys::CanvasRenderingContext2d =
                canvas.get_context("2d")?.ok_or(JsValue::NULL)?.dyn_into()?;

            let w = canvas.width() as f64;
            let h = canvas.height() as f64;

            ctx.set_fill_style_str("#0066cc");
            ctx.fill_rect(0.0, 0.0, w, h);

            ctx.set_stroke_style_str("#002255");
            ctx.set_line_width(4.0);
            ctx.stroke_rect(8.0, 8.0, w - 16.0, h - 16.0);

            ctx.set_fill_style_str("#000000");
            ctx.set_font("bold 28px monospace");
            let _ = ctx.fill_text("\u{26A0} SYSTEM FAILURE \u{26A0}", 24.0, 50.0);

            ctx.set_font("16px monospace");
            let _ = ctx.fill_text("Critical system failure. Details below:", 24.0, 80.0);

            ctx.set_font("13px monospace");
            let max_w = (w - 48.0) as usize / 8;
            let max_w = if max_w < 20 { 60 } else { max_w };
            let mut y = 116.0;
            for line in msg.lines() {
                let chars: Vec<char> = line.chars().collect();
                if chars.is_empty() {
                    y += 18.0;
                    continue;
                }
                let mut start = 0;
                while start < chars.len() {
                    let end = (start + max_w).min(chars.len());
                    let chunk: String = chars[start..end].iter().collect();
                    let _ = ctx.fill_text(&chunk, 24.0, y);
                    y += 18.0;
                    start = end;
                    if y > h - 30.0 {
                        let _ = ctx.fill_text("... (truncated)", 24.0, y);
                        return Ok(());
                    }
                }
            }

            ctx.set_font("14px monospace");
            ctx.set_fill_style_str("#003366");
            let _ = ctx.fill_text("Refresh to reinitialize systems.", 24.0, h - 16.0);

            Ok(())
        })();
    }));
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    set_panic_hook();
}

#[wasm_bindgen]
pub fn start_game() -> Result<(), JsValue> {
    game::init_game()
}
