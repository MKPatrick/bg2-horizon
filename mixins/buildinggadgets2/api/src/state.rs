//! A gadget's own state, in the data components registered in [`crate::ids`].
//!
//! The Java mod keeps all of this per stack in `GadgetNBT`, and so does this
//! port: the state has to travel with the item. Horizon's reader and writer
//! are `Host::component` and `Host::set_component`, which take NBT `Tag`s.
//!
//! Every getter answers the Java mod's default when the component is absent,
//! so a fresh gadget behaves like a fresh gadget in Java.

use crate::ids::MOD_ID;
use crate::modes::{Mode, Settings};
use horizon_client::mod_api::{Host, ItemStack, Tag};

/// `GadgetNBT.getMode`: the mode, Build to Me by default.
pub fn mode(host: &dyn Host, stack: &ItemStack) -> Mode {
    string(host, stack, "gadget_mode")
        .as_deref()
        .and_then(Mode::from_id)
        .unwrap_or(Mode::BuildToMe)
}

/// `GadgetNBT.setMode`.
pub fn set_mode(host: &dyn Host, stack: &mut ItemStack, mode: Mode) {
    let _ = host.set_component(stack, &id("gadget_mode"), Tag::String(mode.id().as_str().into()));
}

/// `GadgetNBT.getGadgetBlockState`: the block the gadget places, as a state
/// string (`minecraft:oak_stairs[facing=east]`). None means air, which the
/// Java mod treats as "nothing selected".
pub fn block_state(host: &dyn Host, stack: &ItemStack) -> Option<String> {
    string(host, stack, "gadget_blockstate").filter(|s| !is_air(s))
}

/// `GadgetUtils.setBlockState`.
pub fn set_block_state(host: &dyn Host, stack: &mut ItemStack, state: &str) {
    let _ = host.set_component(stack, &id("gadget_blockstate"), Tag::String(state.into()));
}

/// `GadgetNBT.getToolRange`, 1 by default as the Java mod has it.
pub fn range(host: &dyn Host, stack: &ItemStack) -> i32 {
    int(host, stack, "gadget_range").unwrap_or(1)
}

/// `GadgetNBT.setToolRange`.
pub fn set_range(host: &dyn Host, stack: &mut ItemStack, range: i32) {
    let _ = host.set_component(stack, &id("gadget_range"), Tag::Int(range));
}

/// Every setting a mode reads, in one go.
pub fn settings(host: &dyn Host, stack: &ItemStack) -> Settings {
    Settings {
        place_on_top: toggle(host, stack, "place_on_top"),
        raytrace_fluid: toggle(host, stack, "raytrace_fluid"),
        fuzzy: toggle(host, stack, "fuzzy"),
        connected_area: toggle(host, stack, "connected_area"),
        affect_tiles: toggle(host, stack, "affect_tiles"),
        range: range(host, stack),
    }
}

/// `GadgetNBT.getSetting`: a toggle, off by default (`Codec.BOOL.orElse(false)`).
pub fn toggle(host: &dyn Host, stack: &ItemStack, name: &str) -> bool {
    match component(host, stack, &format!("{name}_toggle")) {
        Some(Tag::Byte(b)) => *b != 0,
        Some(Tag::Int(i)) => *i != 0,
        _ => false,
    }
}

/// `GadgetNBT.setSetting`.
pub fn set_toggle(host: &dyn Host, stack: &mut ItemStack, name: &str, on: bool) {
    let _ = host.set_component(
        stack,
        &id(&format!("{name}_toggle")),
        Tag::Byte(i8::from(on)),
    );
}

/// `GadgetNBT.getToolValue`: one of the numbered settings.
pub fn value(host: &dyn Host, stack: &ItemStack, name: &str) -> Option<i32> {
    int(host, stack, &format!("{name}_value"))
}

/// The namespaced id of one of the mod's components.
fn id(name: &str) -> String {
    format!("{MOD_ID}:{name}")
}

fn component<'a>(host: &'a dyn Host, stack: &'a ItemStack, name: &str) -> Option<&'a Tag> {
    host.component(stack, &id(name)).ok().flatten()
}

fn string(host: &dyn Host, stack: &ItemStack, name: &str) -> Option<String> {
    match component(host, stack, name) {
        Some(Tag::String(s)) => Some(String::from_utf16_lossy(&s.0)),
        _ => None,
    }
}

fn int(host: &dyn Host, stack: &ItemStack, name: &str) -> Option<i32> {
    match component(host, stack, name) {
        Some(Tag::Int(i)) => Some(*i),
        Some(Tag::Byte(b)) => Some(i32::from(*b)),
        Some(Tag::Short(s)) => Some(i32::from(*s)),
        _ => None,
    }
}

/// Whether a state string names air, which the Java mod uses for "unset".
fn is_air(state: &str) -> bool {
    let name = state.split('[').next().unwrap_or(state);
    matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" | ""
    )
}

/// The block name of a state string, without its properties.
pub fn block_of(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_strings_block_is_its_name_without_properties() {
        assert_eq!(block_of("minecraft:stone"), "minecraft:stone");
        assert_eq!(
            block_of("minecraft:oak_stairs[facing=east,half=bottom]"),
            "minecraft:oak_stairs"
        );
    }

    #[test]
    fn air_states_count_as_unset() {
        assert!(is_air("minecraft:air"));
        assert!(is_air("minecraft:cave_air"));
        assert!(is_air(""));
        assert!(!is_air("minecraft:stone"));
        assert!(!is_air("minecraft:oak_stairs[facing=east]"));
    }
}
