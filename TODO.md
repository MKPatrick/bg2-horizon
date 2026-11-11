# Work list

Grouped by the milestones in [PLAN.md](PLAN.md). The Horizon hook each item
needs is in [docs/hook-map.md](docs/hook-map.md).

## M0 — Before anything compiles — **done except the pin**

- [x] Rust toolchain (1.99.1; Horizon needs 1.90+) and Python 3.11.
- [x] Decide pinning: the mod is a trait mod on named hook points, so it is
      **unpinned** — `hooks` in `horizon-native.json` says which hook-point API
      it needs and the launcher builds it on that version or later. The
      `[compatibility] horizon_revision` block stays commented out.
- [x] Set `hooks` to what the code actually uses: 65 (`component_types`).
      `tools/check_bundle.py` computes it and fails when the two disagree.
- [x] Start the bundle from the official template
      (`cargo horizon-mod new --template native`).
- [x] Confirm every hook the port needs exists: ran Horizon's own API generator
      against the tree — 533 trait methods, all 29 present.
- [x] Check that the mod compiles against Horizon: `tools/local_check.py`,
      clean in 1m27s, and `--tests` runs the mod's own tests.
- [x] Check the content mod with Horizon's own tool: `cargo horizon-mod build`
      and `check` both pass.
- [ ] **Needs the user:** the Horizon commit the mod is tested on, for
      `horizon-native.json`'s `horizon` (only needed to publish to the
      registry, not to build).
- [ ] **Needs a machine with a JDK 25 and the launcher:** `horizon-prepare`
      against an own copy of Java Edition 26.3, then the first real
      `native-build` and a play test. Neither is installed here.

## M1 — It loads — **done except the in-game check**

- [x] `tools/make_content_mod.py`: writes `bg2-content` from the Java mod's
      `src/` tree, 100 files. The textures come from parsing the models, not
      from a list. Leaves out Building Gadgets 1's unregistered construction
      blocks and paste, and the Patchouli book Horizon has no reader for.
- [x] The item models carried over unchanged: the data generator already wrote
      26.3's formats (`assets/<ns>/items/<id>.json`, singular `recipe` and
      `loot_table` folders, results as `{"id": ...}`).
- [x] The 6 `c:` item tags the recipes use, which Horizon does not ship.
- [x] `sounds.json` for the `beep` event.
- [x] The 7 items, the 2 blocks, the creative tab and the template manager's
      menu declared, and the content passing `cargo horizon-mod check`.
- [x] Real templates: `minecraft:compass` for the gadgets and `minecraft:paper`
      for the template and redprint, both inert — M0's `minecraft:stick` was
      furnace fuel, so a gadget could be burnt. The exchanger carries
      `minecraft:enchantable {"value": 3}`, as `.enchantable(3)` gives it.
- [x] `tools/pack_mod.py`: content and code in one `.hmod`, passing
      `cargo horizon-mod check` as `native-mod … for 26.3`.
- [ ] **Needs the launcher:** check in game — the creative tab, the 8 items
      with the right models and names, the template manager placeable, a gadget
      craftable.
- [ ] The render block's look is `minecraft:barrier` with hardness 20. Enough
      until M10 puts the build animation on it; revisit then.

## M2 — One block placed — **done except the in-game check**

- [x] `component_types` (API 65): all 28 of `BG2DataComponents`' components
      registered in `api/src/ids.rs`, with tests.
- [x] The raycast (`VectorHelper.getLookingAt`): a voxel walk from the player's
      eye, 32 blocks, with and without fluids, in `api/src/ray.rs`. It runs on
      the server, as the Java `use` does, so the build stays authoritative.
      Uses a block's collision boxes where it has them.
- [x] `is_pos_valid` (`BaseMode.isPosValid`): world height, `may_build` for
      protection, and the `minecraft:replaceable` tag for `canBeReplaced`.
- [x] `BaseMode.collect` and `BuildToMe.collectWorld`, with 13 tests read off
      the Java source — the `i < difference` run that stops one short, the
      32-block clamp, `place_on_top` measuring to the eye, and `(int) yPos`
      truncating towards zero where `getBlockY()` floors.
- [x] Materials taken with `count_items` / `take_items`, creative excepted,
      and given back when an edit places fewer blocks than were paid for.
- [x] The blocks written with `edit_blocks` (API 126), positions grouped into
      vertical runs (`api/src/runs.rs`, 9 tests).
- [x] `use_item_on` and `use_item` (API 13), claiming the click with
      `Flow::Cancel` as the Java `SUCCESS` does. In reach the hook's own pos
      and face are used — vanilla's pick, no approximation; past it the
      gadget's raycast runs.
- [x] Shift-right-click picks the block looked at
      (`GadgetBuilding.onShiftAction`), written back into the hand that holds
      the gadget.
- [ ] **Needs the launcher:** check in game — a line of blocks placed,
      materials consumed, surviving a reload.

### What M2 does differently from the Java mod

- [ ] **The blocks appear at once.** Java hands each position to
      `ServerTickHandler`, which places a few per tick behind the render
      block's animation. That is M10.
- [ ] **No energy.** The Java gadget costs 50 Forge Energy per block. Horizon
      has no Forge Energy and whether this port charges anything is still open
      (M10).
- [ ] **Materials are the block's own item, one per block.** Java asks for the
      state's silk-touch drops (`GadgetUtils.getDropsForBlockState`), which
      differ for a few blocks (a lit redstone lamp, grass).
- [ ] **`canSurvive` is not checked.** Horizon has no helper that asks a state
      whether it can stay at a position, so a build of torches or saplings is
      not filtered and those blocks drop. It does not affect full blocks.
- [ ] **The raycast uses collision boxes, not outline shapes.** Vanilla's pick
      uses the outline, which a server mod cannot read here. A block with no
      collision (a flower, a torch) is treated as a whole block, so the hit
      can differ from vanilla's by less than a block at its edges.
- [ ] **`isValidBlockState` is not checked when picking.** Java refuses to
      pick a block it would not place; M3 brings that with the rest of the
      mode work.

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
      destruction box — through `world_primitives`, which a trait mod reaches,
      so they need no pass of our own.
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
- [ ] AE2 integration as an optional trait-mod dependency on the AE2 port, once
      that port is a build people run.
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
- [ ] Test on one Horizon commit, put it in `horizon-native.json`'s `horizon`,
      and say which in the README.
- [ ] `tools/check_bundle.py` and `tools/local_check.py --tests` clean.
