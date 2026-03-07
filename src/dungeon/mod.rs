//! Dungeon module — generation, tile types, fog of war.

mod generation;
mod fov;

pub use generation::{AltarKind, DungeonLevel, RoomModifier, SealKind, Tile};
pub use fov::compute_fov;
