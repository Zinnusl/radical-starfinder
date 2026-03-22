//! Dungeon module — kept for backward compatibility, re-exports from world.

#[allow(unused_imports)]
pub use crate::world::fov::compute_fov;
#[allow(unused_imports)]
pub use crate::world::{DungeonLevel, AltarKind, Rng, RoomModifier, SealKind, SpecialRoomKind, Tile};
#[allow(unused_imports)]
pub use crate::world::location_gen::*;
