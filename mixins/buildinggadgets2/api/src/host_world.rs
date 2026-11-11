//! [`Blocks`] over Horizon's `Host`, for the server.
//!
//! The raycast and the modes read the world through `Blocks` so they can be
//! tested without a game (`crate::world::Fake`). This is the real one.

use crate::world::Blocks;
use horizon_client::mod_api::{Host, LevelRef};

/// The `minecraft:replaceable` block tag, which is what
/// `BlockState.canBeReplaced` answers from in 26.3.
const REPLACEABLE: &str = "minecraft:replaceable";

/// One level, as the acting player's build reads it.
pub struct HostBlocks<'a> {
    host: &'a dyn Host,
    level: LevelRef,
    dimension: &'a str,
    /// The player the build is for: their uuid and name, for `may_build`.
    player: (u128, &'a str),
}

impl<'a> HostBlocks<'a> {
    /// A view of `dimension`, or `None` when the server has no such level.
    pub fn new(
        host: &'a dyn Host,
        dimension: &'a str,
        player: (u128, &'a str),
    ) -> Option<Self> {
        let level = host.level(dimension)?;
        Some(Self {
            host,
            level,
            dimension,
            player,
        })
    }
}

impl Blocks for HostBlocks<'_> {
    fn block(&self, pos: [i32; 3]) -> Option<String> {
        // `level_block` is the name alone, without the state's properties.
        // An unloaded chunk answers None, which ends a ray and fails a
        // placement, as it should.
        if !self.host.level_is_loaded(self.level, pos) {
            return None;
        }
        self.host.level_block(self.level, pos)
    }

    fn is_replaceable(&self, pos: [i32; 3]) -> bool {
        let Some(block) = self.block(pos) else {
            return false;
        };
        if self.is_air(&block) {
            return true;
        }
        // `BlockState.canBeReplaced` is the `minecraft:replaceable` tag.
        self.host
            .block_name_in_tag(&block, REPLACEABLE)
            .unwrap_or(false)
    }

    fn collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        self.host.block_collision(pos)
    }

    fn height(&self) -> (i32, i32) {
        self.host.level_height(self.level).unwrap_or((-64, 384))
    }

    fn may_place(&self, _pos: [i32; 3]) -> bool {
        // `BaseMode.isPosValid` calls `level.mayInteract(player, pos)`, which
        // is spawn protection and what region-protection mods hook.
        //
        // Horizon's equivalent is `Host::may_build`, which asks every mod's
        // `can_build`. It is **not in the newest Nightly** (2026-11-08,
        // `NATIVE_HOOKS_API` 411) — it is on main with an unstamped
        // `@API:can_build@`, so there is no released Horizon to build it
        // against yet. Until a Nightly carries it, every position is allowed:
        // in single player that changes nothing, but on a server a gadget
        // ignores claims and spawn protection. TODO.md lists it under
        // "Before release", and `host_world.rs` is the one place to change.
        let _ = (&self.player, self.dimension);
        true
    }
}
