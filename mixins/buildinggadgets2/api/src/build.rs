//! Doing a build: `GadgetBuilding.onAction` and `BuildingUtils.build`.
//!
//! The Java flow, from `BaseGadget.use`:
//!
//! 1. raycast with the gadget's range (`VectorHelper.getLookingAt`);
//! 2. nothing there and no anchor → do nothing;
//! 3. `mode.collect(hitSide, player, hitPos, state)` → offsets;
//! 4. `BuildingUtils.build` checks each position again, takes the materials
//!    and places it.
//!
//! What this does differently, and why:
//!
//! - **The blocks appear at once.** The Java mod hands each position to
//!   `ServerTickHandler`, which places a few per tick behind the render
//!   block's animation. That animation is M10; until then the whole build is
//!   one `edit_blocks` call, which is also how it stays a single packed write
//!   per chunk section.
//! - **No energy.** The Java gadget costs Forge Energy per block
//!   (`BUILDINGGADGET_COST`, 50). Horizon has no Forge Energy, and whether
//!   this port has an energy cost at all is still open (TODO.md, M10), so
//!   nothing is charged yet.
//! - **Materials are the block's own item, one per block.** The Java mod asks
//!   for the state's silk-touch drops (`GadgetUtils.getDropsForBlockState`),
//!   which differ for a handful of blocks. M2 takes the item named like the
//!   block.
//! - **`canSurvive` is not checked** (see `crate::modes::is_pos_valid`).

use crate::host_world::HostBlocks;
use crate::modes::{self, PlayerAt};
use crate::world::Blocks;
use crate::ray;
use crate::runs;
use crate::state;
use horizon_client::mod_api::{
    EquipmentSlot, Host, ItemStack, NativeBlockEdit, NativeBlockRun, Text,
};

/// The Java mod's `Config.RAYTRACE_RANGE` default.
const RAYTRACE_RANGE: f64 = 32.0;

/// What a build did, for the caller to report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// How many blocks were written.
    pub placed: u64,
    /// How many positions were dropped for want of materials.
    pub missing: usize,
}

/// `GadgetBuilding.onShiftAction`: pick the block the player is looking at.
///
/// Answers the state that was picked, for the caller to write onto the stack
/// — the hooks get the stack by reference and Horizon's `set_component` wants
/// it mutable, so the writing is done there.
///
/// The Java mod refuses a block it would not place
/// (`GadgetUtils.isValidBlockState`) and its own render block. Air is refused
/// here; the rest of that check is M3's, with the rest of the mode work.
pub fn pick(
    host: &mut dyn Host,
    gadget: &ItemStack,
    hit: Option<[i32; 3]>,
) -> Option<String> {
    let player = host.acting_player()?;
    let settings = state::settings(host, gadget);
    let entity = host.entity(u32::try_from(player.entity_id).ok()?)?;
    let eye = entity.eye_position(host)?;

    let blocks = HostBlocks::new(host, &player.dimension, (player.uuid, &player.name))?;
    let pos = match hit {
        Some(pos) => pos,
        None => {
            let dir = ray::view_vector(player.yaw, player.pitch);
            ray::cast(&blocks, eye, dir, RAYTRACE_RANGE, settings.raytrace_fluid)?.pos
        }
    };
    let name = blocks.block(pos)?;
    if blocks.is_air(&name) {
        return None;
    }
    // The whole state, with its properties, as `GadgetNBT` keeps it.
    let level = host.level(&player.dimension)?;
    host.level_block_state(level, pos)
}

/// Run a build for the acting player with `gadget` in hand.
///
/// `hit` is the block and face a use-on hook gave, when it had one: in reach,
/// that is vanilla's own pick and is used as it is. Without one (a click in
/// the air, or a block past the player's reach) the gadget's own raycast runs,
/// which is what reaches the full 32 blocks.
///
/// Answers `None` when there was nothing to do, so the caller can leave the
/// click alone.
pub fn build(
    host: &mut dyn Host,
    gadget: &ItemStack,
    hit: Option<([i32; 3], [i32; 3])>,
) -> Option<Built> {
    let player = host.acting_player()?;
    let state = state::block_state(host, gadget)?; // nothing selected: PASS
    let mode = state::mode(host, gadget);
    let settings = state::settings(host, gadget);

    // Where the player is, as a mode reads them.
    let entity = host.entity(u32::try_from(player.entity_id).ok()?)?;
    let eye = entity.eye_position(host)?;
    let block_position = entity.block_position(host)?;
    let at = PlayerAt {
        block: block_position,
        eye_y: eye[1],
    };

    let (start, face) = {
        let blocks = HostBlocks::new(host, &player.dimension, (player.uuid, &player.name))?;
        match hit.and_then(|(pos, face)| {
            crate::world::Face::from_step(face).map(|f| (pos, f))
        }) {
            Some(found) => found,
            None => {
                let dir = ray::view_vector(player.yaw, player.pitch);
                let found = ray::cast(&blocks, eye, dir, RAYTRACE_RANGE, settings.raytrace_fluid)?;
                (found.pos, found.face)
            }
        }
    };

    // The offsets the mode fills, relative to the hit block.
    let offsets = {
        let blocks = HostBlocks::new(host, &player.dimension, (player.uuid, &player.name))?;
        modes::collect(mode, &blocks, face, start, at, settings)
    };
    if offsets.is_empty() {
        return Some(Built::default());
    }

    // Absolute positions, as `BuildingUtils.build` offsets them.
    let positions: Vec<[i32; 3]> = offsets
        .iter()
        .map(|o| [start[0] + o[0], start[1] + o[1], start[2] + o[2]])
        .collect();

    // Materials, unless creative. The Java mod checks them per position and
    // skips the ones it cannot pay for.
    let creative = player.game_mode == 1;
    let item = state::block_of(&state).to_owned();
    let (affordable, missing) = if creative {
        (positions, 0)
    } else {
        let have = host.count_items(&item);
        let want = u32::try_from(positions.len()).unwrap_or(u32::MAX);
        let paid = have.min(want);
        if paid == 0 {
            tell(host, player.uuid, "buildinggadgets2.messages.nomaterials");
            return Some(Built {
                placed: 0,
                missing: positions.len(),
            });
        }
        let taken = host.take_items(&item, paid);
        let n = taken as usize;
        let missing = positions.len().saturating_sub(n);
        (positions.into_iter().take(n).collect(), missing)
    };
    if affordable.is_empty() {
        return Some(Built {
            placed: 0,
            missing,
        });
    }

    let edit = edit_for(&state, &affordable);
    let placed = match host.edit_blocks(&player.dimension, &edit) {
        Ok(n) => n,
        Err(_) => {
            // The edit was refused as a whole, so nothing was placed: give
            // the materials back rather than eating them.
            if !creative {
                let n = u32::try_from(affordable.len()).unwrap_or(u32::MAX);
                host.give_items(&item, n);
            }
            return Some(Built {
                placed: 0,
                missing,
            });
        }
    };

    // Fewer blocks written than paid for (a chunk went out of range, a run
    // fell outside the world): hand back what was not used.
    if !creative {
        let unused = u64::try_from(affordable.len()).unwrap_or(u64::MAX).saturating_sub(placed);
        if unused > 0 {
            host.give_items(&item, u32::try_from(unused).unwrap_or(u32::MAX));
        }
    }

    Some(Built {
        placed,
        missing,
    })
}

/// One bulk edit placing `state` at every position.
pub fn edit_for(state: &str, positions: &[[i32; 3]]) -> NativeBlockEdit {
    NativeBlockEdit {
        palette: vec![state.to_owned()],
        runs: runs::runs(positions)
            .into_iter()
            .map(|r| NativeBlockRun {
                x: r.x,
                z: r.z,
                y: r.y,
                height: r.height,
                surface: None,
                block: 0,
                only: None,
            })
            .collect(),
        // The Java mod places through the normal path, so neighbours hear
        // about it: a torch beside a block it removed falls, redstone
        // updates. `edit_blocks` tells them only when asked.
        neighbour_updates: true,
        ..NativeBlockEdit::default()
    }
}

fn tell(host: &mut dyn Host, uuid: u128, key: &str) {
    host.action_bar(uuid, Text::translate(key, &[]));
}

/// Write a changed gadget back into the hand that holds it.
///
/// The use hooks hand out the stack behind a shared reference, so a change to
/// it would be thrown away. `BaseGadget.getGadget` looks in the main hand and
/// then the off hand; this writes back to whichever really holds the gadget.
pub fn put_back(host: &mut dyn Host, stack: ItemStack) -> bool {
    let Some(player) = host.acting_player() else {
        return false;
    };
    let Some(entity) = u32::try_from(player.entity_id).ok().and_then(|id| host.entity(id)) else {
        return false;
    };
    let Some(living) = host.living(entity) else {
        return false;
    };
    for slot in [EquipmentSlot::MainHand, EquipmentSlot::OffHand] {
        let holds = host
            .living_equipment(living, slot)
            .is_some_and(|s| s.item == stack.item);
        if holds {
            return host.living_set_equipment(living, slot, Some(stack));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_names_the_state_once_and_groups_its_runs() {
        let positions: Vec<[i32; 3]> = (0..5).map(|y| [1, y, 2]).collect();
        let edit = edit_for("minecraft:stone", &positions);
        assert_eq!(edit.palette, vec!["minecraft:stone".to_owned()]);
        assert_eq!(edit.runs.len(), 1);
        assert_eq!(edit.runs[0].height, 5);
        assert_eq!((edit.runs[0].x, edit.runs[0].y, edit.runs[0].z), (1, 0, 2));
        assert_eq!(edit.runs[0].block, 0);
        assert!(edit.neighbour_updates);
    }

    #[test]
    fn a_state_with_properties_is_kept_whole_in_the_palette() {
        let edit = edit_for("minecraft:oak_stairs[facing=east]", &[[0, 0, 0]]);
        assert_eq!(edit.palette[0], "minecraft:oak_stairs[facing=east]");
    }

    #[test]
    fn no_positions_make_no_runs() {
        assert!(edit_for("minecraft:stone", &[]).runs.is_empty());
    }
}
