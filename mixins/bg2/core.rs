//! Building Gadgets 2 for Horizon: the mod's own state.
//!
//! This module is the port's root. It holds what the Java mod keeps in static
//! registries and `DataComponentType`s, and nothing that touches the world:
//! the hooks in the other modules reach in here.
//!
//! The Java mod this ports: `com.direwolf20.buildinggadgets2` (MIT).
//! The mapping from its classes to Horizon's hooks: `docs/hook-map.md`.

/// The mod's namespace, as the Java mod's `MODID`. The content mod, the data
/// components, the network channels and the keybinds all live under it.
pub const MOD_ID: &str = "buildinggadgets2";

/// The gadgets, in the order the creative tab lists them.
pub const GADGETS: [&str; 5] = [
    "gadget_building",
    "gadget_exchanging",
    "gadget_copy_paste",
    "gadget_cut_paste",
    "gadget_destruction",
];

/// The other items the mod adds. The template manager's item comes from its
/// block declaration (`"item": true`), not from a declaration of its own.
pub const ITEMS: [&str; 2] = ["template", "redprint"];

/// The blocks the mod adds: the template manager, and the temporary block the
/// build animation runs on (`RenderBlock` in the Java mod).
pub const BLOCKS: [&str; 2] = ["template_manager", "render_block"];

/// A gadget's per-stack settings that are on or off (`GadgetNBT.ToggleableSettings`).
pub const TOGGLES: [&str; 7] = [
    "affect_tiles",
    "raytrace_fluid",
    "place_on_top",
    "paste_replace",
    "bind",
    "fuzzy",
    "connected_area",
];

/// A gadget's per-stack numbers (`GadgetNBT.IntSettings`).
pub const INT_SETTINGS: [&str; 6] = ["bind_direction", "left", "right", "up", "down", "depth"];

// TODO (M1): `native_component_types` (API 65) registers a component id for each
// of the above, plus mode, range, anchor, the copy/paste positions, the gadget
// and copy UUIDs and the undo list. See docs/hook-map.md.
