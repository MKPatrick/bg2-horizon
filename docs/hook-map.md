# Building Gadgets 2 → Horizon: what each part of the Java mod lands on

Checked against Horizon's own source in the snapshot this port started from
(`native-hooks.json`: 434 hooks, 177 helpers, `NATIVE_HOOKS_API` **415**), not
only against its docs. Where a claim below came from running something, it says
so. The Java side is Building Gadgets 2 for Minecraft 26.1.2 / NeoForge
26.1.2.122: 144 files, about 14 000 lines.

`API n` is the `"hooks": n` the mod must declare to use that hook. The highest
one it actually uses is what `horizon-native.json` carries, and
`tools/check_bundle.py` computes it from the code.

## The mod is a trait mod, not a mixin bundle

Horizon takes native mods two ways, and this matters for everything below.

- A **mixin bundle** (`[[injection]]`, `[[module]]`) rewrites the game's source
  before it is compiled. AE2's port is one, because it needed surgery in the
  mesher, in block entity drawing and in villager trade tables.
- A **trait mod** (`[api]`) is an ordinary Rust crate that implements
  `horizon_client::mod_api::Hooks` and is linked into the game the launcher
  builds. `cargo horizon-mod new --template native` writes one, so it is the
  route Horizon points a new mod at.

Building Gadgets 2 is a trait mod. Every line of the mapping below is a named
hook point, so nothing needs the source rewritten, and the trait route gives:

- **No pin while developing.** A mixin bundle must name the Horizon commit it
  was built against, because the file it patches can move (`E_MODULE`,
  `E_TARGET_NOT_FOUND`). A trait mod declares `"hooks": N` and a build from the
  folder compiles against whatever Horizon the instance plays — hook points are
  only ever added, so a mod builds on any Horizon at N or later. Horizon's own
  native mods (Zoom, the FPS counter, the minimap) are made this way so they
  need no re-release per Nightly.

  A **published** file is stricter, which is easy to miss: its two manifests
  must name the same commit, or the reader refuses it with *"native manifest and
  mixin identity/revision differ"* (`horizon_mods::index::native`). So
  `horizon-native.json`'s `horizon` and `[compatibility] horizon_revision` both
  carry the commit the mod was tested on, and `hooks` is what still lets the
  launcher build it on a later one. `tools/pack_mod.py` adds that pin to the
  copy it packs — as the launcher does to its own copy of a mod it builds — and
  the bundle in this repository stays unpinned.
- **It can be switched off** in Options → Mods without a rebuild. Mixin code
  cannot.
- **It compiles locally.** `tools/local_check.py` points the crate's
  `horizon-client` dependency at a local Horizon tree, the way the launcher
  does, and `cargo check` runs in about 90 seconds once the dependencies are
  built. The mod's own `cargo test` runs too, which is what the mode geometry in
  M3 is checked with.

The generated trait has **533 methods** — one per catalog hook. Every one of
the 29 hooks this port needs is among them, confirmed by running Horizon's own
generator (`horizon_mod_api_gen::generate_test_hooks`) against this tree.

### What a trait mod cannot reach

`horizon_mods::content::{Item, Block}` — the types `items` and `blocks` take —
are not re-exported through `horizon_client::mod_api`, and the `[api]` crate may
only depend on `horizon-client`. So **declaring blocks and items in code is not
available to a trait mod**; they come from the content mod's files instead
(`bg2-content/data/buildinggadgets2/horizon/`), which is where this port puts
them anyway. Nothing is lost: the files are the declaration either way, and a
content mod's ids are what a save and a joining client check against.

## Registration and per-stack data

| Java | Horizon | API |
|---|---|---|
| `DeferredRegister` ITEMS, 7 plain items | `bg2-content`'s `horizon/item/*.json` | — |
| `DeferredRegister` BLOCKS, `render_block` + `template_manager` | `horizon/block/*.json` | — |
| `BlockItem` for the template manager | the block's own `"item": true` (the default a file sets). A block's item is **not** a separate `horizon/item/` file; `render_block` sets `"item": false` so it has none | — |
| `Item.Properties.stacksTo(1)` | the item file's `"max_stack_size": 1`. **Not** a `minecraft:max_stack_size` component: Horizon refuses that with *"component … : set max_stack_size instead"* (`content.rs`'s `check_components`), which `cargo horizon-mod build` reports | — |
| `Item.Properties.enchantable(3)` on the exchanger | the item file's `"components": {"minecraft:enchantable": {"value": 3}}` | — |
| the gadgets' template | `minecraft:compass`, and `minecraft:paper` for the template and redprint: inert items with no use action. Not `minecraft:stick`, which is furnace fuel — a template's components come with it, so a fuel template makes the gadget burnable | — |
| `BG2DataComponents`: 12 fixed, 7 toggles, 6 numbers, 3 more | `component_types`, then `component` / `set_component` / `remove_component`. Implemented, with tests, in `api/src/ids.rs` | 65 |
| `BlockEntityType` `RenderBlockBE`, `TemplateManagerBE` | `block_entity`, `block_entity_ticks`, `block_entity_tick` | 15 |
| `TemplateManagerContainer` (`MenuType`) | `block_entity_container` + `horizon/menu/template_manager.json` | 16, 17 |
| `SoundEvent` `beep` | the content mod's `assets/buildinggadgets2/sounds` | — |
| Creative tab | `horizon/creative_tab/buildinggadgets2.json` | — |

A gadget's state is per stack, so it stays in components, as in Java. The
world-level stores are attachments, below.

## Using a gadget

| Java | Horizon | API |
|---|---|---|
| `BaseGadget.useOn` / `use` (right click) | `use_item_on`, then `use_item` | 13 |
| left click with a gadget (undo, anti-build) | `left_click_block` | 13 |
| `GadgetDestruction` breaking blocks | `destroy_block` (cancellable) | 14 |
| `BuildingUtils` placing many blocks | `edit_blocks` (one packed write per section, as `fill … strict`), block entity NBT through `NativeBlockEdit.block_entities` | 126 |
| a single placement that must tell its neighbours | `set_block` | — |
| taking and giving the player's materials | `count_items`, `take_items`, `give_items` | 9 |
| protection checks (claims, spawn) | `may_build`, which **is** there as a helper¹ | unstamped |
| `BlockState.canBeReplaced` | `block_name_in_tag(block, "minecraft:replaceable")` | unstamped |
| `Player.isShiftKeyDown` (pick vs build) | `sneaking()` — what `UseOnContext.isSecondaryUseActive` reads | unstamped |
| `VectorHelper.getLookingAt` | no server-side helper: `raycast` and `view_hit` are the client's own view. A voxel walk of ours over `entity_eye_position` + `view_vector`, with `block_collision` for shapes (`api/src/ray.rs`) | 363, 379 |
| writing a changed gadget back | `living_set_equipment(living, MainHand/OffHand, stack)`: the use hooks hand the stack out behind a shared reference | unstamped |
| `Item.appendHoverText` | `tooltip` | 12 |
| `BuildingGadgets2Commands`, `RedprintCommand` | `register_commands`, `perform_command` | 8 |
| build and destroy particles | `send_particles`, `ParticleOptions::named` for the mod's own | 11, 103 |

¹ An earlier draft of this document called the protection check unavailable,
on the grounds that `can_build`'s API number is an unstamped
`@API:can_build@`. That was too pessimistic: `Host::may_build` is in the
generated host trait and compiles, so the check is made. What the unstamped
number means is only that the mod cannot name a version below the catalog's
own — see the note on `hooks` below.

## What `hooks` has to say

A mod declares `"hooks": N` and the launcher builds it on any Horizon at N or
later. **The helpers a mod calls set the same floor as the hooks it
implements**: a helper that is not there yet does not compile. Several helpers
this port needs carry an unstamped `@API:host_bridge@` instead of a number
(`sneaking`, `block_name_in_tag`, `may_build`, `component`,
`living_set_equipment`, `action_bar`), so no version below the catalog's own is
known to have them, and `hooks` is the catalog's API — 415 in this snapshot.
`tools/check_bundle.py` scans the source for helper calls and works that floor
out, so the manifest cannot drift below what the code needs.

## The stores the Java mod keeps in `SavedData`

`BG2Data extends SavedData` holds the undo lists, the copy/paste snapshots, the
block entity data of a cut, and the redprint name → UUID map. Horizon's
equivalent is a server attachment with a codec on `Holder::Level`, saved into
the dimension's `horizon_attachments.dat`:

```rust
static BG2_DATA: AttachmentType<Bg2Data> = AttachmentType {
    namespace: "buildinggadgets2", name: "data",
    default: Bg2Data::default,
    codec: Some(AttachmentCodec { encode: ..., decode: ... }),
    copy_on_death: false, sync: false,
};
```

`AttachmentType`, `AttachmentCodec` and `Holder` are re-exported through
`horizon_client::mod_api`, so a trait mod reaches them.
`Holder::Player(uuid)` carries what the Java mod hangs off a player, and
`Holder::BlockEntity([x, y, z])` what a `RenderBlockBE` keeps.

## Client and server talking

The Java mod has 16 payloads (`common/network/data`) with 16 handlers. They all
go over Horizon's single mod-net channel, one channel name each:

| Java payload | Direction |
|---|---|
| `ModeSwitchPayload`, `RangeChangePayload`, `ToggleSettingPayload`, `RenderChangePayload`, `AnchorPayload`, `RotatePayload`, `RelativePastePayload`, `CopyCoordsPayload`, `CutPayload`, `UndoPayload`, `SendPastePayload`, `SendCopyDataToServerPayload`, `RequestCopyDataPayload`, `DestructionRangesPayload`, `UpdateTemplateManagerPayload` | client → server |
| `SendCopyDataPayload` | server → client |

- `net_send_to_server(channel, data)` and
  `net_send_to_player(player, channel, data)` send; `net_from_client(player,
  channel, data)` and `net_from_server(channel, data)` hear. API **40**.
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
| `TemplateManagerGUI` (a block's menu) | `screen`, `screen_draw`, `screen_click`, `screen_tooltip`, `menu_button` | 19, 20 |
| `ModeRadialMenu`, `CopyGUI`, `PasteGUI`, `DestructionGUI`, `MaterialListGUI` (no block behind them) | `mod_screen_open(mod_id)`, drawn in `screen_render_post`, input from the `screen_*_pre` hooks | 111 |
| `GuiTemplatePreview` | `draw_scene(batch, rect, blocks, view)` | — |
| the widgets in `client/screen/widgets` | drawn by hand with `fill`, `quad`, `sprite`, `text`, `item_stack` | 313 |
| `KeyBindings`: `settings_menu` G, `undo` U, `anchor` H, `range` R | `set_mod_key` / `mod_key`, presses through `key` / `input_key` | 111 |

## The ghost preview

The Java mod draws its preview with its own buffer builders and a VBO
(`VBORenderer`, `DireBufferBuilder`, `DireVertexConsumer`,
`DireVertexConsumerSquished`, `MyRenderMethods`, `OurRenderTypes`,
`FakeRenderingWorld`): translucent block models at the positions a build would
fill, re-baked when the target moves.

Horizon has no `RenderType`, no `PoseStack` and no `BufferBuilder`. It splits
into two halves, and only the second is real work.

**The lines and boxes come free.** `world_primitives(host, event)` is a trait
method — `line`, `box_outline`, `filled_box`, `beam` and `text`, in world
coordinates with ARGB colours, each with `.through_walls()`, drawn by Horizon
after the entities. That covers the build's bounding box, the anchor marker, the
copy region and the destruction box with no pipeline of our own. (An earlier
draft of this document had it as unreachable, on the grounds that it is a
trait-mod API and not in `native-hooks.json`. That was right about the API and
wrong about the consequence: this mod *is* a trait mod.)

**The textured ghost blocks are the custom part.** `RenderHooks::world_drawn`
(API **58**, reached through `Hooks::render`) runs on the render thread once the
world is drawn, with the frame's device, queue, encoder, colour target, the
world's reversed depth and the camera's matrices. With it:

- `render_needs` asks for `world_depth` to be kept, so the pass reads the
  world's depth rather than binding it — the world may draw at a lower
  resolution than the frame, and a pass's attachments must agree in size.
- `block_sprites` hands over the block sprite arrays, so a ghost quad carries
  the real block's texture.

**AE2's port already built this pass**: `mixins/ae2/ae2_draw.rs`, 306 lines — a
translucent quad pass run from `world_drawn`, with a per-quad block sprite, an
ARGB colour and a `behind` factor for what shows through the world, written for
(among other things) its own placement preview. Read it before writing ours. The
shape of the problem is the same; only the number of quads is larger. It lives
in a module in the renderer's crate because AE2 is a mixin bundle; as a trait
mod the same code goes in the `[api]` crate behind `Hooks::render`.

## Integrations

| Java | Horizon |
|---|---|
| `AE2Integration` (pull materials from an ME network) | trait mods can depend on each other by `[mod] version` with Cargo semver (`base_mod = "^1"`, optional ones behind a feature named after the mod id), so this becomes an optional dependency on the AE2 port — cleaner than AE2's own `optional` macro |
| `CuriosIntegration` (a gadget in a Curios slot) | no Curios on Horizon: dropped, unless a port appears |
| JEI | `horizon-jei` exists (Joeypc6/horizon-jei); the recipe-viewer hooks are API 2 plus the pending `panel_style` |
| the Patchouli book in `patchouli_books/` | no Patchouli on Horizon. AE2 drew its own guide over any screen; the same is possible here, or the book is dropped |

## What will not carry over

- **Java addons.** `GadgetModes.registerMode` is Building Gadgets 2's public API
  for other mods. Nothing Java runs on Horizon. Another *native* mod could add
  modes through the trait-mod dependency mechanism above, which is the nearest
  equivalent.
- **`FakeRenderingWorld`.** It exists to feed vanilla's block renderer a
  pretend world. The preview pass builds its quads directly, so there is nothing
  left to fake.
- **Minecraft 26.1.2.** Horizon is 26.3. Block ids, components and recipe
  formats move with it; `tools/make_content_mod.py` converts what it can, and
  the differences get written down here as they are found.
