//! Dungeon module — kept for backward compatibility, re-exports from world.

pub use crate::world::fov::compute_fov;
pub use crate::world::{DungeonLevel, AltarKind, Rng, RoomModifier, SealKind, SpecialRoomKind, Tile};
pub use crate::world::location_gen::*;
