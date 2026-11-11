# Building Gadgets 2 for Horizon

An unofficial, in-progress port of
[Building Gadgets 2](https://github.com/Direwolf20-MC/BuildingGadgets2) (MIT, by
direwolf20) to [Horizon](https://github.com/horizon-rust/horizon), as a **native
mod**: Rust compiled into the game, with no Java.

> **Status: M2 done and played, M3 next.** The Building Gadget picks a block
> (shift-right-click, at up to 32 blocks) and builds a line of it in Build to
> Me mode, taking the materials. **Verified in a running game** on a native
> build: 9 blocks placed where the unit tests said, and `placed: 5, missing: 4`
> with only 5 stone in the bag. The other ten modes, the ghost preview, undo
> and the screens are still to come.
> [PLAN.md](PLAN.md) has the milestones and [TODO.md](TODO.md) the work list,
> including what M2 does differently from the Java mod.

## Why a rewrite and not a jar

Horizon is Minecraft: Java Edition 26.3 rewritten in Rust. It runs no Java, so
Fabric, Forge and NeoForge jars do not load: their code calls and patches
Minecraft's Java classes, and there are none. What a mod can do instead is one
of two things, and Building Gadgets 2 needs both:

- a **content mod** (`.hmod`) — blocks, items, recipes, models, textures, with
  no code;
- a **native mod** (`.hnmod`) — Rust that hooks named points in the game. This
  port is a **trait mod**: an ordinary Rust crate implementing
  `horizon_client::mod_api::Hooks`, which the launcher links into the game it
  builds for the player. It is not a [horizon-mixins](https://github.com/horizon-rust/horizon-mixins)
  bundle, because nothing here needs the game's source rewritten — see
  [docs/hook-map.md](docs/hook-map.md) for why that matters.

Nearly all of Building Gadgets 2 is behaviour — ray casts, modes, placement,
undo, the ghost preview, five screens — so the port is a rewrite of its logic in
Rust against Horizon's hooks. No original code runs.

## Layout

| Path | What it is |
|---|---|
| `mixins/buildinggadgets2/` | the native mod: `horizon-mixin.toml` and the `api/` crate that implements `Hooks`. Named after the mod id because that is the path a packed file must use |
| `mixins/buildinggadgets2/api/src/ids.rs` | the mod's ids and its data components, with tests |
| `bg2-content/` | the gadgets' and blocks' declarations, and the models, textures, names, recipes and loot carried over from the Java mod |
| `dist/buildinggadgets2.hmod` | content and code in one file, what a player installs (built, not committed) |
| `horizon-native.json` | the registry manifest: the game version, the hooks API and the commit the mod was tested on |
| `docs/hook-map.md` | every part of the Java mod and the Horizon hook it lands on |
| `tools/check_bundle.py` | checks the bundle and the manifest against a Horizon tree before a build |
| `tools/local_check.py` | `cargo check`/`cargo test` the mod against a local Horizon tree |
| `tools/make_content_mod.py` | regenerates `bg2-content` from the Java mod's own resources |
| `tools/pack_mod.py` | packs content and code into one `.hmod` |
| `PLAN.md`, `TODO.md` | the milestones, and the work list under them |

## What is being ported

The whole mod, as the Java version has it:

- **Five gadgets.** Building, Exchanging, Copy-Paste, Cut-Paste, Destruction.
- **Eleven modes.** Build to Me, Vertical Wall, Vertical Column, Surface,
  Horizontal Wall, Stairs, Horizontal Row, Grid for the Building Gadget; the
  four area modes for the Exchanger; Copy, Cut and Paste.
- **The per-stack settings.** Range, fuzzy, connected area, affect tiles,
  raytrace fluid, place on top, paste replace, the anchor and its direction, and
  the destruction gadget's six-sided box.
- **The ghost preview**, the thing the mod is known for: translucent block
  models showing what a click would place, before it places it.
- **Undo**, per gadget, over several builds.
- **Templates and redprints**, the template manager block and its screen.
- **The radial mode menu**, the copy, paste, destruction and material list
  screens, and the four keybinds.

## Building

Native mods are **unsandboxed code that runs with your rights** — read what you
build.

The mod is unpinned: it declares the hook-point API it needs (`hooks` in
`horizon-native.json`) and the launcher builds it on any Horizon at that version
or later, so it does not need a new release for every Nightly.

```bash
horizon-launcher native-build mixins/buildinggadgets2 --instance <instance> --trust-native-code
```

A build from the folder is unpinned: it compiles against whatever Horizon the
instance plays. A *published* file must name the commit in both of its
manifests, so `tools/pack_mod.py` adds the pin to the copy it packs.

### Checking it without a native build

A native build takes a while, so both halves can be checked first. Needs Rust
1.90+ and a Horizon source tree (`../horizon-main` by default):

```bash
python tools/check_bundle.py
```

```bash
python tools/local_check.py --tests
```

The content side is built and checked with Horizon's own tool, from
`bg2-content`:

```bash
cargo horizon-mod build && cargo horizon-mod check target/horizon-mod/buildinggadgets2.hmod
```

And the whole thing — content and code in one file, which is what a player
installs — with:

```bash
python tools/pack_mod.py && cargo horizon-mod check dist/buildinggadgets2.hmod
```

## Differences from the Java mod

Written down as they are found. The ones already known from the plan:

- **No Curios integration.** Curios does not exist on Horizon, so a gadget
  cannot sit in a Curios slot.
- **No Patchouli book.** Patchouli does not exist on Horizon. The in-game guide
  is either drawn by the mod itself or dropped.
- **No Java addon API.** `GadgetModes.registerMode` let other Java mods add
  modes. Nothing Java runs here; another native mod could add modes through
  Horizon's trait-mod dependencies instead.
- **Minecraft 26.3, not 26.1.2.** The Java mod targets 26.1.2; Horizon is 26.3.
- **Large copies are split across several network messages.** Horizon's mod-net
  channel takes 30 000 bytes a message, which a big template exceeds.

## Credits and licences

- **Building Gadgets 2**, © 2023 [Direwolf20-MC](https://github.com/Direwolf20-MC/BuildingGadgets2),
  [MIT](LICENSE). This port is a rewrite of that mod's behaviour, under the same
  licence.
- Models, textures and translations come from the Java mod and keep its terms.
- **Horizon** and its mod hooks: the [horizon-rust](https://github.com/horizon-rust)
  project, MIT.
- The **AE2 Horizon port** was the reference for how a Java mod becomes a native
  one; its `ae2_draw.rs` in particular for drawing in the world.

An unofficial port, not made or endorsed by direwolf20. Please report problems
here, not to the original author.
