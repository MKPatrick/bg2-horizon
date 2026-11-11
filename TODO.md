# Work list

Grouped by the milestones in [PLAN.md](PLAN.md). The Horizon hook each item
needs is in [docs/hook-map.md](docs/hook-map.md).

## M0 — Before anything compiles

- [ ] Pin a Horizon Nightly: put its commit in `horizon-native.json`
      (`horizon`) and `mixins/bg2/horizon-mixin.toml`
      (`[compatibility] horizon_revision`). Both say
      `TODO-pin-a-nightly-commit`.
- [ ] Pin a `horizon-mixins` commit (`mixins` in `horizon-native.json`).
- [ ] Confirm `NATIVE_HOOKS_API` on that commit, and set `hooks` to the highest
      API this port actually uses (415 is the snapshot's ceiling, not a
      requirement).
- [ ] Start the bundle from
      [horizon-native-mod-template](https://github.com/horizon-rust/horizon-native-mod-template)
      and check that an empty bundle with only `core.rs` applies and compiles,
      before any hook is added.
- [ ] Get Horizon building locally at all: Rust 1.90+, Python 3.10+, JDK 25,
      and `horizon-prepare` run against an own copy of Java Edition 26.3.

## M1 — It loads

- [ ] `tools/make_content_mod.py`: read the Java mod's jar (or
      `src/main/resources`) and write `bg2-content` — models, textures, lang,
      recipes, tags — the way AE2's tool does it. 240 asset files and 37
      generated resources to convert.
- [ ] Convert the item models: the gadgets are flat `item/generated` models, so
      they should carry over nearly unchanged.
- [ ] Convert the recipes from 26.1.2 to 26.3's format.
- [ ] Decide the render block's look: it is never seen in an inventory and is
      replaced within a tick, so a `barrier`-like template with no collision is
      enough.
- [ ] Pack `bg2-content` with Horizon's own tool
      (`cargo horizon-mod build`, then `cargo horizon-mod check`), put the
      result in `mixins/bg2/data/bg2-content.hmod`, and load it from the bundle
      at start.
- [ ] `native_items` and `native_blocks` (API 145): declare the 7 plain items
      and the 2 blocks in code so they exist even if the content mod's files
      change. The template manager's item comes from its block
      (`"item": true`), and `render_block` sets `"item": false`.
- [ ] Check in game: the creative tab, the 8 items with the right models and
      names, the template manager placeable.

## M2 — One block placed

- [ ] `native_component_types` (API 65): register every component in
      `BG2DataComponents` — mode, range, the 7 toggles, the 6 numbers, the
      anchor and its list, the copy/paste positions, the gadget and copy UUIDs,
      the blockstate, the undo list, the template name.
- [ ] Port `VectorHelper` / `VecHelper`: the ray cast from the player's eye,
      with and without fluids (`raytrace_fluid`).
- [ ] Port `UseContext` and `BuildingUtils`' placement check: can this position
      take a block, is it replaceable, is it in range, is it loaded.
- [ ] Port `BaseMode` and `BuildToMe` only, with its tests.
- [ ] Count and take the player's materials (`native_count_items`,
      `native_take_items`), creative mode excepted.
- [ ] Write the blocks with `native_edit_blocks` (API 126): build the palette
      and the runs, one call per build.
- [ ] `native_use_item_on` + `native_use_item` (API 13): the right click that
      starts a build, claimed so the item's own use does not also run.
- [ ] Check in game: a line of blocks placed, materials consumed, survives a
      reload.

## M3 — Every mode

- [ ] Port the remaining ten modes, each with tests read off the Java source:
      `VerticalWall`, `VerticalColumn`, `Surface`, `HorizontalWall`, `Stairs`,
      `HorizontalRow`, `Grid`, `Copy`, `Cut`, `Paste`.
- [ ] The `exchanging` variant of `Surface`, `VerticalColumn`, `HorizontalRow`
      and `Grid` (the same classes with the flag set).
- [ ] `fuzzy` and `connected_area`: the flood fill that decides a surface's
      extent.
- [ ] `place_on_top`, `affect_tiles`, `paste_replace`.
- [ ] Range, and the anchor with its direction (`bind`, `bind_direction`,
      `anchor_pos`, `anchor_list`, `anchor_side`).
- [ ] The Exchanger: replace a block in place, drop what was there.
- [ ] Mode and range over the network (`ModeSwitchPayload`,
      `RangeChangePayload`, `ToggleSettingPayload`).
- [ ] Check each mode against the Java mod for the same click.

## M4 — The preview

- [ ] Read AE2's `mixins/ae2/ae2_draw.rs` first: the translucent quad pass it
      already runs from `native_world_drawn` is the same problem.
- [ ] `native_render_needs`: ask for `world_depth`, and only while something is
      drawn.
- [ ] The pass: vertex buffer, pipeline, bind groups, and the world's depth read
      in the shader rather than bound (the world may draw at a lower resolution
      than the frame).
- [ ] Block sprites per quad (`native_block_sprites`), so a ghost carries the
      real block's texture.
- [ ] Build the quad set from a mode's block positions, with face culling
      between neighbouring ghosts — the Java mod's `DireVertexConsumer` work.
- [ ] Rebuild only when the target, the mode, the range or the settings change;
      keep the buffer's capacity between frames.
- [ ] The lines and boxes: the build's bounds, the anchor, the copy region, the
      destruction box — as quads in the same pass, since `world_primitives` is
      not reachable from a mixin bundle.
- [ ] The squished variant (`DireVertexConsumerSquished`) for the exchanger's
      overlay.
- [ ] Check: the preview and the placement agree, exactly, in every mode.
- [ ] Measure: a few thousand ghost blocks must not cost noticeable frames.

## M5 — Undo

- [ ] The level attachment replacing `BG2Data`: undo lists per gadget UUID, the
      copy/paste snapshots, the block entity data of a cut, the redprint names —
      with a codec, on `Holder::Level`.
- [ ] Record each build's previous states as it is made.
- [ ] `UndoPayload` and the undo keybind; left click with a gadget.
- [ ] Undo several builds in order, and across a save and reload.
- [ ] Decide and document the undo depth, and what happens when the world has
      changed under an undo.

## M6 — Copy, cut, paste

- [ ] The copy region: two corners, `CopyCoordsPayload`, the box drawn.
- [ ] The snapshot: block states plus block entity NBT
      (`native_block_entity_nbt`).
- [ ] Cut: take the blocks, keep their data (`CutPayload`).
- [ ] Paste: `native_edit_blocks` with `block_entities` for the NBT, the
      relative offset (`RelativePastePayload`) and rotation (`RotatePayload`).
- [ ] Chunked transfer for a snapshot over 30 000 bytes, both ways, with
      reassembly and a cap (`SendCopyDataPayload`,
      `SendCopyDataToServerPayload`, `RequestCopyDataPayload`).
- [ ] The paste preview, reusing M4's pass.

## M7 — Destruction

- [ ] The destruction gadget's box: left, right, up, down, depth, and the side
      it was clicked on.
- [ ] `native_destroy_block` (API 14) and the drops, or `native_edit_blocks`
      plus the drops worked out by hand — decide which, and say why.
- [ ] `DestructionRangesPayload` and the destruction screen's six numbers.
- [ ] Undo a destruction (it shares M5's lists).
- [ ] Refuse what must not be destroyed: bedrock, unbreakable blocks, blocks
      out of reach.

## M8 — Screens and keys

- [ ] `native_set_mod_key` / `native_mod_key` for the four keybinds:
      `settings_menu` (G), `undo` (U), `anchor` (H), `range` (R).
- [ ] `native_mod_screen_open` (API 111) and the drawing in
      `native_screen_render_post`; input from the `native_screen_*_pre` hooks.
- [ ] The widgets the screens need, drawn by hand (`native_fill`,
      `native_quad`, `native_sprite`, `native_text`, `native_item_stack`):
      toggle, slider, number field, button.
- [ ] `ModeRadialMenu`: the mode wheel, with each mode's icon.
- [ ] `CopyGUI`, `PasteGUI`, `DestructionGUI`.
- [ ] `MaterialListGUI`: what a paste would cost, and what the player has.
- [ ] The template manager's menu (API 17): the block entity's slots, the
      menu's declaration, `native_screen` / `native_screen_draw` /
      `native_screen_click` / `native_menu_button`.
- [ ] `GuiTemplatePreview` with `native_draw_scene`.
- [ ] Every setting reachable in game, not only by command.

## M9 — Templates and redprints

- [ ] The template item: a name, a snapshot, copied between gadgets.
- [ ] The template manager's save and load, and `UpdateTemplateManagerPayload`.
- [ ] The redprint: the name → UUID map in the attachment, and
      `RedprintCommand`.
- [ ] `native_register_commands` (API 8) for the mod's commands.
- [ ] Import and export: decide whether a template can be a file, and whether
      the Java mod's format is read.

## M10 — Polish

- [ ] The build animation: the render block, its block entity and its tick
      (API 15), so a build appears block by block as it does in Java.
- [ ] Particles (API 11, 103) and the `beep` sound.
- [ ] `native_tooltip` (API 12): a gadget's mode, range and settings on its
      tooltip.
- [ ] `native_item_decorations` and `native_item_bar` if a gadget shows a bar.
- [ ] Energy: the Java mod has a `forge_energy` component. Decide whether the
      port has an energy cost at all, and if so use `item_energy_buffer`.
- [ ] AE2 integration behind an `optional` macro, once the AE2 port is a build
      people run.
- [ ] JEI: the recipe-viewer hooks, against `horizon-jei`.
- [ ] The guide book: draw it, or drop it and say so in the README.

## Before release

- [ ] **Multiplayer audit.** Read every `native_net_from_client` handler again
      against a client that sends anything: reach, game mode, the stack in hand,
      position count and distance, the UUIDs a message names. Test on a
      dedicated server as a non-operator.
- [ ] Commands need an operator level where they change someone else's world.
- [ ] `can_build` / `may_build` once its API number is stamped, so region
      protection mods are respected.
- [ ] Performance: a large build, a large paste, a large preview — measured, not
      guessed.
- [ ] Write down every difference from the Java mod in the README.
- [ ] Credits and licences checked: the Java mod's MIT notice, the textures'
      terms, Horizon's.
- [ ] Pin and test one Horizon commit, and say which in the README.
