//! Building Gadgets 2 for Horizon: the native mod.
//!
//! A trait mod (docs/native-mods.md, "Trait mods"): the launcher links this
//! crate into the game it builds and calls [`hooks`]. Each method of
//! `horizon_client::mod_api::Hooks` is one of Horizon's hook points, and this
//! mod implements the ones Building Gadgets 2 needs. The blocks and items
//! themselves come from the content mod in `bg2-content`, not from here.
//!
//! What each part of the Java mod lands on: `docs/hook-map.md`.
//!
//! Milestone 2: shift-right-click picks the block you look at, right-click
//! builds a line of it in Build to Me mode. The other ten modes, the preview,
//! undo and the screens are later milestones.

use horizon_client::mod_api::{
    ComponentTypesEvent, Flow, Hooks, Host, ItemStack, PerformCommandEvent,
    RegisterCommandsEvent, UseItemEvent, UseItemOnEvent,
};

pub mod build;
pub mod command;
pub mod host_world;
pub mod ids;
pub mod modes;
pub mod ray;
pub mod runs;
pub mod state;
pub mod world;

/// The launcher calls this once and keeps the object for the game's life.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(BuildingGadgets2)
}

/// The mod. Trait mods are process-wide and their hooks take `&self`, so
/// anything mutable that is added later belongs behind its own lock, not in
/// this struct.
struct BuildingGadgets2;

/// Whether an item id is one of the gadgets this mod acts for.
fn is_gadget(item: &str) -> bool {
    ids::GADGETS.contains(&item)
}

/// `BaseGadget.use`: one right click with a gadget, from either use hook.
///
/// `hit` is the block and face a use-on hook gave — in reach, that is
/// vanilla's own pick, so no raycast of ours is needed. Without one (a click
/// in the air, or a block past the player's reach) the gadget raycasts up to
/// 32 blocks itself, which is the whole point of `VectorHelper.getLookingAt`.
///
/// Answers whether the click was the gadget's, so the caller can claim it.
fn clicked(
    host: &mut dyn Host,
    stack: &ItemStack,
    hit: Option<([i32; 3], [i32; 3])>,
) -> bool {
    // While the port has had no in-game run yet, every click says what it did
    // in the game's log: whether the hook fired at all, which way it went and
    // why nothing happened. Take it out once the milestones are checked in a
    // world (TODO.md, M2).
    let sneaking = host.sneaking();
    eprintln!(
        "buildinggadgets2: click item={} sneaking={sneaking} hit={hit:?}          player={:?} state={:?}",
        stack.item,
        host.acting_player().map(|p| (p.name, p.position, p.yaw, p.pitch, p.entity_id)),
        state::block_state(host, stack),
    );

    // `Player.isShiftKeyDown`, which the Java mod branches on in `use`.
    if sneaking {
        let picked = build::pick(host, stack, hit.map(|(pos, _)| pos));
        eprintln!("buildinggadgets2: pick -> {picked:?}");
        let Some(state) = picked else {
            return false;
        };
        // The hooks hand out the stack behind a shared reference, so the pick
        // is written back into the hand that holds the gadget.
        let mut held = stack.clone();
        state::set_block_state(host, &mut held, &state);
        let written = build::put_back(host, held);
        eprintln!("buildinggadgets2: picked {state}, written back = {written}");
        return true;
    }
    let built = build::build(host, stack, hit);
    eprintln!("buildinggadgets2: build -> {built:?}");
    built.is_some()
}

impl Hooks for BuildingGadgets2 {
    /// Register the data components a gadget carries (`BG2DataComponents`).
    ///
    /// Asked once before content loads. Duplicates, invalid ids and the
    /// `minecraft` namespace stop startup, so the list is built and checked in
    /// [`ids::component_ids`].
    fn component_types(&self, event: &mut ComponentTypesEvent<'_>) {
        event.types.extend(ids::component_ids());
    }

    /// `/bg2`, which reaches a gadget's mode and settings until the screens
    /// exist (M8). Not in the Java mod; see `command.rs`.
    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.extend(command::commands());
    }

    /// A player ran one of the commands above. Claiming it keeps the game
    /// from answering "unknown command" for one this mod handled.
    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        if command::perform(host, event.command) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A right click with a gadget on a block in reach (`ItemStack.useOn`).
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if !is_gadget(&event.stack.item) {
            return Flow::Continue;
        }
        let stack = event.stack.clone();
        // The click is the gadget's: claim it so the item's own use does not
        // also run, as the Java mod's SUCCESS does.
        if clicked(host, &stack, Some((event.pos, event.face))) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A right click with a gadget in the air (`Item.use`), where building at
    /// a distance happens.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if !is_gadget(&event.stack.item) {
            return Flow::Continue;
        }
        let stack = event.stack.clone();
        if clicked(host, &stack, None) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_gadgets_are_acted_on() {
        assert!(is_gadget("buildinggadgets2:gadget_building"));
        assert!(is_gadget("buildinggadgets2:gadget_exchanging"));
        assert!(!is_gadget("buildinggadgets2:template"));
        assert!(!is_gadget("minecraft:stone"));
    }
}
