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

    fn state(&self, pos: [i32; 3]) -> Option<String> {
        if !self.host.level_is_loaded(self.level, pos) {
            return None;
        }
        self.host.level_block_state(self.level, pos)
    }

    fn has_block_entity(&self, pos: [i32; 3]) -> bool {
        // `block_entity_nbt` needs a mutable host, which this view does not
        // hold, so the block's own name decides. It covers what the
        // `affect_tiles` setting is really about — a chest or a sign losing
        // its contents — but a mod block entity whose name is not listed here
        // is not recognised. TODO.md carries this under M3.
        self.block(pos).is_some_and(|b| {
            matches!(
                b.as_str(),
                "minecraft:chest"
                    | "minecraft:trapped_chest"
                    | "minecraft:barrel"
                    | "minecraft:furnace"
                    | "minecraft:blast_furnace"
                    | "minecraft:smoker"
                    | "minecraft:hopper"
                    | "minecraft:dropper"
                    | "minecraft:dispenser"
                    | "minecraft:shulker_box"
                    | "minecraft:brewing_stand"
                    | "minecraft:beacon"
                    | "minecraft:lectern"
                    | "minecraft:jukebox"
                    | "minecraft:campfire"
                    | "minecraft:soul_campfire"
                    | "minecraft:crafter"
                    | "minecraft:chiseled_bookshelf"
                    | "minecraft:decorated_pot"
                    | "minecraft:enchanting_table"
                    | "minecraft:ender_chest"
                    | "minecraft:sign"
                    | "minecraft:spawner"
            ) || b.ends_with("_sign")
                || b.ends_with("_shulker_box")
                || b.ends_with("_banner")
                || b.ends_with("_bed")
                || b.ends_with("_skull")
                || b.ends_with("_head")
        })
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
