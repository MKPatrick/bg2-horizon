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
//! Milestone 1 registers the gadgets' data components and nothing else.

use horizon_client::mod_api::{ComponentTypesEvent, Hooks};

pub mod ids;

/// The launcher calls this once and keeps the object for the game's life.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(BuildingGadgets2)
}

/// The mod. Trait mods are process-wide and their hooks take `&self`, so
/// anything mutable that is added later belongs behind its own lock, not in
/// this struct.
struct BuildingGadgets2;

impl Hooks for BuildingGadgets2 {
    /// Register the data components a gadget carries (`BG2DataComponents`).
    ///
    /// Asked once before content loads. Duplicates, invalid ids and the
    /// `minecraft` namespace stop startup, so the list is built and checked in
    /// [`ids::component_ids`].
    fn component_types(&self, event: &mut ComponentTypesEvent<'_>) {
        event.types.extend(ids::component_ids());
    }
}
