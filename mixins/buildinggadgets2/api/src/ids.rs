//! The mod's names: its items, blocks and the data components a gadget carries.
//!
//! The items and blocks themselves are declared by the content mod
//! (`bg2-content/data/buildinggadgets2/horizon/`), not here; these constants
//! are what the hooks match on.

/// The mod's namespace, as the Java mod's `MODID`.
pub const MOD_ID: &str = "buildinggadgets2";

/// The Exchanger, whose modes behave differently (`BaseMode.isExchanging`).
pub const EXCHANGER: &str = "buildinggadgets2:gadget_exchanging";

/// The five gadgets, in the order the creative tab lists them.
pub const GADGETS: [&str; 5] = [
    "buildinggadgets2:gadget_building",
    "buildinggadgets2:gadget_exchanging",
    "buildinggadgets2:gadget_copy_paste",
    "buildinggadgets2:gadget_cut_paste",
    "buildinggadgets2:gadget_destruction",
];

/// The mod's other items.
pub const ITEMS: [&str; 2] = ["buildinggadgets2:template", "buildinggadgets2:redprint"];

/// The mod's blocks. `render_block` is the temporary block the build animation
/// runs on (`RenderBlock` in the Java mod) and has no item.
pub const BLOCKS: [&str; 2] = [
    "buildinggadgets2:template_manager",
    "buildinggadgets2:render_block",
];

/// A gadget's settings that are on or off (`GadgetNBT.ToggleableSettings`).
pub const TOGGLES: [&str; 7] = [
    "affect_tiles",
    "raytrace_fluid",
    "place_on_top",
    "paste_replace",
    "bind",
    "fuzzy",
    "connected_area",
];

/// A gadget's numbers (`GadgetNBT.IntSettings`).
pub const INT_SETTINGS: [&str; 6] = [
    "bind_direction",
    "left",
    "right",
    "up",
    "down",
    "depth",
];

/// The components a gadget carries that are neither a toggle nor a number
/// (`BG2DataComponents`' fixed ones).
pub const FIXED_COMPONENTS: [&str; 12] = [
    "bound_global_pos",
    "anchor_pos",
    "anchor_side",
    "anchor_list",
    "render_type",
    "copy_start_pos",
    "copy_end_pos",
    "relative_paste",
    "gadget_uuid",
    "copy_uuid",
    "gadget_blockstate",
    "gadget_mode",
];

/// Every data component the mod registers, namespaced, in a stable order.
///
/// `native_component_types` refuses duplicates and the `minecraft` namespace,
/// so this is built once and checked by `component_ids_are_unique`.
pub fn component_ids() -> Vec<String> {
    let mut ids = Vec::with_capacity(
        FIXED_COMPONENTS.len() + TOGGLES.len() + INT_SETTINGS.len() + 3,
    );
    for name in FIXED_COMPONENTS {
        ids.push(format!("{MOD_ID}:{name}"));
    }
    // The Java mod's `*_toggle` and `*_value` components, one per setting.
    for name in TOGGLES {
        ids.push(format!("{MOD_ID}:{name}_toggle"));
    }
    for name in INT_SETTINGS {
        ids.push(format!("{MOD_ID}:{name}_value"));
    }
    ids.push(format!("{MOD_ID}:gadget_range"));
    ids.push(format!("{MOD_ID}:template_name"));
    ids.push(format!("{MOD_ID}:undo_list"));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_ids_are_unique_and_namespaced() {
        let ids = component_ids();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "a component id is repeated");
        for id in &ids {
            let (ns, name) = id.split_once(':').expect("a component id has no namespace");
            assert_eq!(ns, MOD_ID);
            assert!(!name.is_empty());
            assert!(
                name.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{id} is not a valid path"
            );
        }
    }

    #[test]
    fn the_java_mods_components_are_all_here() {
        // BG2DataComponents registers 12 fixed components, a toggle for each of
        // the 7 ToggleableSettings, a value for each of the 6 IntSettings, and
        // gadget_range, template_name and undo_list.
        assert_eq!(component_ids().len(), 12 + 7 + 6 + 3);
    }

    #[test]
    fn gadgets_and_items_are_namespaced() {
        for id in GADGETS.iter().chain(ITEMS.iter()).chain(BLOCKS.iter()) {
            assert!(id.starts_with("buildinggadgets2:"), "{id}");
        }
    }
}
