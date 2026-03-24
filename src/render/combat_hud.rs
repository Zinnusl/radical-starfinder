//! Tactical battle (combat arena) rendering.

use crate::combat::{
    ArenaBiome, BattleTile, Direction, EnemyIntent, Projectile, TacticalBattle, TacticalPhase,
    TargetMode, TypingAction, Weather, WuxingElement,
};
use crate::player::Player;
use crate::radical;

use super::{COL_PLAYER, COL_HP_BAR, COL_HP_BG};

mod grid;
mod panels;
mod effects;

impl super::Renderer {
    pub(crate) fn draw_tactical_battle(&self, battle: &TacticalBattle, anim_t: f64, _player: &Player) {
        let grid_size = battle.arena.width as f64;
        let max_grid_px = (self.canvas_h - 80.0).min(self.canvas_w * 0.55);
        let cell = (max_grid_px / grid_size).floor().max(24.0).min(36.0);
        let grid_px = grid_size * cell;
        let grid_x = (self.canvas_w - grid_px) / 2.0;
        let grid_y = 30.0;

        // Full-screen dark backdrop
        self.ctx.set_fill_style_str("rgba(10,6,18,0.94)");
        self.ctx.fill_rect(0.0, 0.0, self.canvas_w, self.canvas_h);


        // Grid, terrain, units, projectiles
        self.draw_tactical_grid(battle, anim_t, _player, cell, grid_px, grid_x, grid_y, grid_size);

        // Right panel, menus, look mode
        self.draw_tactical_panels(battle, anim_t, cell, grid_px, grid_x, grid_y, grid_size);

        // Banners, messages, typing UI, battle log
        self.draw_tactical_effects(battle, anim_t, cell, grid_px, grid_x, grid_y, grid_size);
    }
}
