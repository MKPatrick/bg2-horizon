# Building Gadgets 2 on Horizon: working plan

How the port is built and in what order. The work list itself is
[TODO.md](TODO.md); what each Java class lands on is
[docs/hook-map.md](docs/hook-map.md).

## What we are porting from

- **Building Gadgets 2**, Minecraft 26.1.2 / NeoForge 26.1.2.122, MIT.
  144 Java files, ~14 000 lines — small next to AE2's port (196 Rust files,
  ~78 000 lines), but with one part AE2 only touched in passing: a preview that
  draws thousands of translucent block models in the world.
- **Horizon**, Minecraft 26.3, `NATIVE_HOOKS_API` 415 in the snapshot this
  started from.

Every hook the port needs already exists: the generated `Hooks` trait has 533
methods and all 29 the port needs are among them, confirmed by running
Horizon's own generator against the tree. Nothing here waits on a Horizon
change, with one exception worth knowing: `can_build` / `may_build` (the
protection check) is documented with an unstamped API placeholder, so builds
skip it until that number lands.

**The port is a trait mod, not a mixin bundle** — an ordinary Rust crate
implementing `Hooks`, linked into the game the launcher builds. That decision is
argued in [docs/hook-map.md](docs/hook-map.md); its consequences run through
this plan: the mod is unpinned, it can be switched off in game, and it compiles
and tests locally in about 90 seconds.

## Layers

The port is built bottom-up, so each layer can be tested before the one above
it exists.

1. **Content.** The gadgets, the template manager and the render block, their
   models, textures and recipes, converted from the jar. No behaviour.
2. **State.** The data components a gadget carries per stack, and the
   level attachment that replaces `BG2Data`'s `SavedData` (undo lists, copy
   snapshots, redprint names).
3. **Geometry.** The modes: given a hit position, a side, a player's look and
   the gadget's settings, which block positions does a build fill? This is pure
   Rust with no hooks at all, so it is testable on its own and is where the
   mod's behaviour actually lives.
4. **Acting.** Right click builds, left click undoes: materials counted and
   taken, blocks written with `native_edit_blocks`, undo recorded.
5. **Networking.** The 16 payloads, each re-checked on arrival.
6. **The preview.** The wgpu pass run from `native_world_drawn`.
7. **Screens.** The radial menu, copy, paste, destruction, material list, and
   the template manager's menu.
8. **The rest.** Templates and redprints, commands, particles, sounds, the
   build animation, tooltips, AE2 and JEI.

Layer 3 is the one to get exactly right: every mode's block set has to match the
Java mod's for the same input, or every build is subtly wrong. It is also the
cheapest to check, because it needs no game.

## Milestones

Each ends with a native build that is actually played, as AE2's rounds did.

| # | Milestone | Done when |
|---|---|---|
| **M1** | **It loads.** The content mod carries the Java mod's models, textures and recipes; the 8 items and 2 blocks exist; the creative tab shows them. | A build boots and the gadgets can be held. |
| **M2** | **One block placed.** The Building Gadget in Build to Me mode: ray cast, the block set, materials taken, `native_edit_blocks` writes it. No preview, no modes menu. | Right-clicking builds a line of blocks that survives a reload. |
| **M3** | **Every mode.** All eleven modes, the range and the toggles, the Exchanger. Mode switching over the network. | Each mode places what the Java mod places, checked against it. |
| **M4** | **The preview.** The wgpu ghost pass, and the boxes and lines around it. | What the preview shows is what M3 places. |
| **M5** | **Undo.** The level attachment, the undo list per gadget, left click. | Several builds undone in order, across a save. |
| **M6** | **Copy, cut, paste.** The copy region, the snapshot in the attachment, paste with rotation and the relative offset, split across messages. | A structure copied and pasted, block entities included. |
| **M7** | **Destruction.** The destruction gadget, its box, its screen, and the drops. | A box destroyed and undone. |
| **M8** | **Screens.** The radial menu and the four screens, the keybinds, the template manager and its menu. | Every setting reachable in game, not only by command. |
| **M9** | **Templates and redprints.** The template item, the manager's save and load, the redprint and its command. | A template written, named, carried and pasted. |
| **M10** | **Polish.** The build animation (`RenderBlockBE`), particles, sounds, tooltips, the material list, AE2 and JEI. | Nothing in `docs/hook-map.md` left unanswered. |

M1–M5 is a mod that is already worth playing: build, exchange, undo. M6–M10
makes it the whole mod.

## How each milestone is checked

- **Mode geometry against the Java mod.** The Java modes are small and
  self-contained (`util/modes/*.java`, one `collect` each). The Rust ones get
  unit tests whose expected block sets are read off the Java source by hand,
  case by case: a position, a side, a look vector, the settings, and the sorted
  positions out. This is the only way the port stays 1:1, and it costs nothing
  to run.
- **A native build, then play.** Every milestone ends with
  `horizon-launcher native-build … --trust-native-code` and a real world.
  Whatever is found gets written into a round's notes at the bottom of this
  file, as AE2's PLAN.md did.
- **Multiplayer last, but not forgotten.** A connected player's messages are
  untrusted. Before anything is released, every `native_net_from_client`
  handler is read again against a client that sends anything at all: reach,
  game mode, the stack in hand, the positions' distance and count.

## Decisions taken

- **A trait mod, not a mixin bundle.** Everything in the mapping is a named hook
  point, and the trait route is unpinned, switchable in game and locally
  compilable. See [docs/hook-map.md](docs/hook-map.md).
- **One combined `.hmod`, content and code together.** Horizon reads a `.hmod`
  that holds both manifests as content plus code, so `tools/pack_mod.py` packs
  `bg2-content` and the bundle into one file and a player installs one thing.
  AE2's port predates this and packs a `.hmod` inside its `.hnmod`, loading it
  with `native_data!`; none of that is needed.
- **The repository's bundle stays unpinned, the packed copy is pinned.** A
  folder build compiles against whatever Horizon the instance plays, which is
  the point of a hook-point mod. A published file must name the commit in both
  manifests, so the packer adds it to the copy it writes.
- **Blocks and items are declared in files, not in code.** A trait mod cannot
  reach `horizon_mods::content::{Item, Block}`, so the `items` and `blocks`
  hooks are not available to it. The content mod's files are the declaration,
  which is where they belonged anyway.
- **Namespace stays `buildinggadgets2`.** The jar's models, textures, recipes
  and lang files then carry over by path, and a world's saved gadgets keep their
  ids.
- **Components, not attachments, for a gadget's own state.** It is per stack and
  has to travel with the item, exactly as in Java.
- **The preview is one pass, not one draw per block.** A build of a few thousand
  blocks is a few thousand quads; they go into one vertex buffer rebuilt only
  when the target moves, as the Java mod's VBO does.
- **Modes are a Rust trait, not a registry.** Nothing Java can register into it,
  so the indirection the Java mod needs for its addon API buys nothing here.

## Open questions

- **How large a copy is worth supporting?** 30 000 bytes a message and 32
  messages a tick per player caps a paste's transfer rate. A big schematic needs
  chunked transfer with progress; the Java mod just sends it. Decide the cap and
  say so in the README.
- **The guide book.** Draw it like AE2's (`kit_guide*.rs`, a real cost) or drop
  it and point at the wiki?
- **AE2 integration.** Worth it only once the AE2 port is a build people run.
  Keep it behind the `optional` macro so a build without AE2 is unaffected.
- **Does `can_build` get a stamped API number?** Until it does, builds skip the
  protection check, which a server owner may object to. Watch for the number.

## Rounds

Notes from each build and test go here, newest last, as AE2's plan did: what was
done, what was found, what is left.

### Round 0 (2026-11-11): the repository

Read Horizon's mod docs, AE2's port and the Java mod. Wrote the hook mapping,
this plan and the work list; laid out the bundle, the content mod and the
manifests. Nothing compiled yet: the Horizon commit is not pinned, and M1 has
not started.

### Round 1 (2026-11-11): M0, and the architecture

Rust 1.99.1 installed; no JDK and no launcher on this machine, so a native
build and a real play test are still out of reach. Everything short of that was
done, and two things turned out differently from Round 0's reading.

- **Built Horizon's own content-mod tool** from the snapshot
  (`cargo build -p cargo-horizon-mod`, two dependencies, 1m52s). `cargo
  horizon-mod build` packs `bg2-content` and `check` passes it:
  *"buildinggadgets2 0.1.0 (mod, API 0.2, both) … no code: content only"*.
- **It found a real bug in Round 0's declarations.** The gadgets had
  `"components": {"minecraft:max_stack_size": 1}`; Horizon refuses that —
  *"component "minecraft:max_stack_size": set max_stack_size instead"* — and the
  dedicated `max_stack_size` field is the right one. Verified by making the
  mistake again on a copy and watching the build fail.
- **The mod is a trait mod, not a mixin bundle.** `cargo horizon-mod new
  --template native` writes a trait mod, and every line of the mapping is a
  named hook point, so nothing needs the game's source rewritten. Ran Horizon's
  own API generator against the tree: the trait has 533 methods and all 29 this
  port needs are there.
- **`world_primitives` is reachable after all.** Round 0 wrote it off because it
  is a trait-mod API absent from `native-hooks.json`. That was right about the
  API and wrong about the consequence. The preview's lines, boxes and the
  destruction box come free; only the textured ghost blocks need a pass of our
  own.
- **A trait mod cannot declare blocks and items in code**:
  `horizon_mods::content::{Item, Block}` are not reachable from the `[api]`
  crate. The content mod's files do it instead, so nothing is lost.
- **The mod compiles and its tests run against the real Horizon source.**
  `tools/local_check.py` patches the `horizon-client` git dependency to the
  local tree *and* Horizon's three vendored crates (`wgpu-hal`, `naga`,
  `libmimalloc-sys`) — without the latter the build dies in
  `horizon-client-render` on `wgpu::hal::dx12::disk_cache`. `cargo check` is
  clean in 1m27s; three unit tests in `ids.rs` pass.
- **Wrote `tools/check_bundle.py`**, which enforces Horizon's own manifest rules
  (read out of `horizon_mods::index::native`) and computes the `hooks` number
  the code needs. It caught the version mismatch and the over-declared
  `hooks: 415`, now 65.

Left for the user: the Horizon commit the mod is tested on, and a machine with
a JDK and the launcher for the first real build.

### Round 2 (2026-11-11): M1, the mod's own look

Building Gadgets 2's data generator already writes the formats 26.3 reads, so
carrying the look over was mostly a matter of copying the right files and
supplying the three things the Java loader gave it for free.

- **`tools/make_content_mod.py`** writes `bg2-content` from the Java mod's
  `src/` tree: 100 files — the 8 item definitions, the item and block models,
  the blockstates, 5 languages, 6 recipes, 2 loot tables, the recipe
  advancements, the tags, and the textures. The textures are not listed in the
  script: the models are parsed and whatever they name is taken, so a model
  that gains a layer needs no change here. It leaves out Building Gadgets 1's
  construction blocks, paste and paste containers, which are still in
  `src/main/resources` but which `Registration.java` does not register, and the
  Patchouli book, which Horizon has no reader for.
- **The `c:` tags had to be written.** The recipes use `#c:ingots/iron` and
  five others. Those are NeoForge's convention, not vanilla's, and Horizon
  ships none of them — AE2's port ships its own too. Without them every recipe
  would match nothing, so the script writes the six from the vanilla items they
  stand for, and the recipes stay exactly as the Java mod wrote them.
- **`sounds.json`** maps the `beep` event to its file.
- **The gadgets are no longer sticks.** M0 gave them `minecraft:stick` to have
  something that holds; a stick is furnace fuel, so a gadget could be burnt.
  They are `minecraft:compass` now, and the template and redprint
  `minecraft:paper` — inert items with no use action and no fuel. The exchanger
  also carries `minecraft:enchantable {"value": 3}`, as the Java mod's
  `.enchantable(3)` gives it, which its `mining_loot` tag needs.
- **One combined `.hmod`.** `docs/api/mod-files.md` says a `.hmod` holding both
  manifests is read as content plus code, so `tools/pack_mod.py` writes one:
  112 content files and 6 native entries, 132 KB. It enforces the reader's own
  rules, which is how `api/Cargo.lock` turned out not to be allowed (only
  `.rs` and `.toml` under the bundle).
- **The pin is a packaging matter, not a development one.** The combined file
  was refused with *"native manifest and mixin identity/revision differ"*: a
  published file must name the commit in `horizon-native.json` *and*
  `[compatibility] horizon_revision`. A folder build does not. So the packer
  adds the pin to the copy it packs, as the launcher does to its own copy, and
  the repository's bundle stays unpinned.
- **Verified:** `cargo horizon-mod check` passes the combined file as
  `native-mod … for 26.3`. And the content inside it really is checked — broke
  one declaration on a copy and the check refused the whole file.

Still nothing clicked in a world: that needs the launcher and a JDK. M2 is the
first milestone where a gadget does something.
