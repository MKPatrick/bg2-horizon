# Building Gadgets 2 → Horizon: what each part of the Java mod lands on

Checked against Horizon's `native-hooks.json` (434 hooks, 177 helpers,
`NATIVE_HOOKS_API` **415**) and `docs/native-mods.md` in the snapshot this port
started from. The Java side is Building Gadgets 2 for Minecraft 26.1.2 /
NeoForge 26.1.2.122: 144 files, about 14 000 lines.

`API n` is the `"hooks": n` a bundle must declare to use that hook. The highest
one this port needs decides `horizon-native.json`'s `hooks`.

## Registration and per-stack data

| Java | Horizon | API |
|---|---|---|
| `DeferredRegister` ITEMS, 8 items | `native_items`, plus `bg2-content`'s `horizon/item/*.json` | 145 |
| `DeferredRegister` BLOCKS, `render_block` + `template_manager` | `native_blocks`, plus `horizon/block/*.json` | 145 |
| `BG2DataComponents`: 15 fixed components, 7 toggles, 6 numbers | `native_component_types`, then `native_component` / `native_set_component` / `native_remove_component` | 65 |
| `BlockEntityType` `RenderBlockBE`, `TemplateManagerBE` | `native_block_entity`, `native_block_entity_ticks`, `native_block_entity_tick` | 15 |
| `TemplateManagerContainer` (`MenuType`) | `native_block_entity_container` + `horizon/menu/template_manager.json` | 16, 17 |
| `SoundEvent` `beep` | the content mod's `assets/buildinggadgets2/sounds` | — |
| Creative tab | `horizon/creative_tab/buildinggadgets2.json` | — |

A gadget's state is per stack, so it stays in components, as in Java. The
world-level stores (below) become attachments.

## Using a gadget

| Java | Horizon | API |
|---|---|---|
| `BaseGadget.useOn` / `use` (right click) | `native_use_item_on`, then `native_use_item` | 13 |
| left click with a gadget (undo, anti-build) | `native_left_click_block` | — |
| `GadgetDestruction` breaking blocks | `native_destroy_block` (cancellable) | 14 |
| `BuildingUtils` placing many blocks | `native_edit_blocks` (one packed write per section, as `fill … strict`), block entity NBT through `NativeBlockEdit.block_entities` | 126 |
| a single placement that must tell its neighbours | `native_set_block` | — |
| taking and giving the player's materials | `native_count_items`, `native_take_items`, `native_give_items` | 9 |
| protection checks (claims, spawn) | `native_may_build` → every mod's `native_can_build` | pending¹ |
| `Item.appendHoverText` | `native_tooltip` | 12 |
| `BuildingGadgets2Commands`, `RedprintCommand` | `native_register_commands`, `native_perform_command` | 8 |
| build and destroy particles | `native_send_particles`, `ParticleOptions::named` for the mod's own | 11, 103 |

¹ `can_build` / `may_build` carry an unstamped `@API:can_build@` placeholder in
this snapshot. Confirm the number before relying on them; until then place
without the check, as the Java mod does.

## The stores the Java mod keeps in `SavedData`

`BG2Data extends SavedData` holds the undo lists, the copy/paste snapshots, the
block entity data of a cut, and the redprint name → UUID map. Horizon's
equivalent is a server attachment with a codec on `Holder::Level`, saved into the
dimension's `horizon_attachments.dat`:

```rust
static BG2_DATA: AttachmentType<Bg2Data> = AttachmentType {
    namespace: "buildinggadgets2", name: "data",
    default: Bg2Data::default,
    codec: Some(AttachmentCodec { encode: ..., decode: ... }),
    copy_on_death: false, sync: false,
};
```

`Holder::Player(uuid)` carries what the Java mod hangs off a player, and
`Holder::BlockEntity([x, y, z])` what a `RenderBlockBE` keeps.

## Client and server talking

The Java mod has 16 payloads (`common/network/data`) with 16 handlers. They all
go over Horizon's single mod-net channel, one channel name each:

| Java payload | Direction |
|---|---|
| `ModeSwitchPayload`, `RangeChangePayload`, `ToggleSettingPayload`, `RenderChangePayload`, `AnchorPayload`, `RotatePayload`, `RelativePastePayload`, `CopyCoordsPayload`, `CutPayload`, `UndoPayload`, `SendPastePayload`, `SendCopyDataToServerPayload`, `RequestCopyDataPayload`, `DestructionRangesPayload`, `UpdateTemplateManagerPayload` | client → server |
| `SendCopyDataPayload` | server → client |

- `native_net_send_to_server(channel, data)` and
  `native_net_send_to_player(player, channel, data)` send;
  `native_net_from_client(player, channel, data)` and
  `native_net_from_server(channel, data)` hear. API **40**.
- Channels are `buildinggadgets2:<name>`.
- At most 30 000 bytes a message and 32 messages a tick each way per player, so
  a large copy must be split and reassembled — something the Java mod never has
  to do. This is the one place where the network layer is not a transcription.
- What a client sends is checked by nothing. Every handler re-checks the
  player's reach, their game mode, and that the stack in their hand really is
  the gadget the message claims.

## The screens

| Java | Horizon | API |
|---|---|---|
| `TemplateManagerGUI` (a block's menu) | `native_screen`, `native_screen_draw`, `native_screen_click`, `native_screen_tooltip`, `native_menu_button` | 19, 20 |
| `ModeRadialMenu`, `CopyGUI`, `PasteGUI`, `DestructionGUI`, `MaterialListGUI` (no block behind them) | `native_mod_screen_open(mod_id)`, drawn in `native_screen_render_post`, input from the `native_screen_*_pre` hooks | 111 |
| `GuiTemplatePreview` | `native_draw_scene(batch, rect, blocks, view)` | — |
| the widgets in `client/screen/widgets` | drawn by hand with `native_fill`, `native_quad`, `native_sprite`, `native_text`, `native_item_stack` | 313 |
| `KeyBindings`: `settings_menu` G, `undo` U, `anchor` H, `range` R | `native_set_mod_key` / `native_mod_key`, presses through `native_key` / `native_input_key` | — |

## The hard part: the ghost preview

The Java mod draws its preview with its own buffer builders and a VBO
(`VBORenderer`, `DireBufferBuilder`, `DireVertexConsumer`,
`DireVertexConsumerSquished`, `MyRenderMethods`, `OurRenderTypes`,
`FakeRenderingWorld`): translucent block models at the positions a build would
fill, re-baked when the target moves.

Horizon has no `RenderType`, no `PoseStack` and no `BufferBuilder`. The preview
becomes the mod's own wgpu pass, run from the renderer's own hook point:

- `native_world_drawn(world)` (API **58**) runs on the render thread once the
  world is drawn, with the frame's device, queue, encoder, colour target, the
  world's reversed depth and the camera's matrices.
- `native_render_needs(needs)` asks for `world_depth` to be kept, so the pass
  reads the world's depth rather than binding it — the world may draw at a lower
  resolution than the frame, and a pass's attachments must agree in size.
- `native_block_sprites` hands over the block sprite arrays, so a ghost quad can
  carry a real block texture.
- `native_world_primitives` would draw the lines and boxes (a build's bounding
  box, the anchor, a copy region), but in this snapshot it is a **trait mod**
  API and not in `native-hooks.json`. From a mixin bundle, draw those as quads
  in the same pass instead.

**AE2's port already built this pass**: `mixins/ae2/ae2_draw.rs`, 306 lines — a
translucent quad pass run from `native_world_drawn`, with a per-quad block
sprite, an ARGB colour and a `behind` factor for what shows through the world,
written for (among other things) its own placement preview. Read it before
writing ours. The shape of the problem is the same; only the number of quads is
larger.

## Integrations

| Java | Horizon |
|---|---|
| `AE2Integration` (pull materials from an ME network) | the AE2 Horizon port's own hooks, compiled in only when that mod is in the build (AE2's `mixins/ae2/optional` macro) |
| `CuriosIntegration` (a gadget in a Curios slot) | no Curios on Horizon: dropped, unless a port appears |
| JEI | `horizon-jei` exists (Joeypc6/horizon-jei); the recipe-viewer hooks are API 2 plus the pending `panel_style` |
| the Patchouli book in `patchouli_books/` | no Patchouli on Horizon. AE2 drew its own guide over any screen (`kit_guide*.rs`); the same is possible here, or the book is dropped |

## What will not carry over

- **Java addons.** `GadgetModes.registerMode` is Building Gadgets 2's public API
  for other mods. Nothing Java runs on Horizon, so it becomes a Rust-side list
  that another native mod could only extend by being compiled into the same
  build.
- **`FakeRenderingWorld`.** It exists to feed vanilla's block renderer a
  pretend world. The preview pass builds its quads directly, so there is nothing
  left to fake.
- **Minecraft 26.1.2.** Horizon is 26.3. Block ids, components and recipe
  formats move with it; `tools/make_content_mod.py` converts what it can, and
  the differences get written down here as they are found.
