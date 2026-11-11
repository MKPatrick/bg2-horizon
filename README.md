# Building Gadgets 2 for Horizon

An unofficial, in-progress port of
[Building Gadgets 2](https://github.com/Direwolf20-MC/BuildingGadgets2) (MIT, by
direwolf20) to [Horizon](https://github.com/horizon-rust/horizon), as a **native
mod**: Rust compiled into the game, with no Java.

> **Status: scaffolding.** Nothing is playable yet. The repository holds the
> plan, the Java → Horizon hook mapping, and the manifests the build needs.
> [PLAN.md](PLAN.md) has the milestones and [TODO.md](TODO.md) the work list.

## Why a rewrite and not a jar

Horizon is Minecraft: Java Edition 26.3 rewritten in Rust. It runs no Java, so
Fabric, Forge and NeoForge jars do not load: their code calls and patches
Minecraft's Java classes, and there are none. What a mod can do instead is one
of two things, and Building Gadgets 2 needs both:

- a **content mod** (`.hmod`) — blocks, items, recipes, models, textures, with
  no code;
- a **native mod** (`.hnmod`) — a [horizon-mixins](https://github.com/horizon-rust/horizon-mixins)
  bundle: Rust that hooks named points in the game's own source before it is
  compiled. The launcher compiles the game for the player.

Nearly all of Building Gadgets 2 is behaviour — ray casts, modes, placement,
undo, the ghost preview, five screens — so the port is a rewrite of its logic in
Rust against Horizon's hooks. No original code runs.

## Layout

Following the [AE2 Horizon port](https://github.com/), which took the same road
from a Java mod to a native one:

| Path | What it is |
|---|---|
| `mixins/bg2/` | the native mod: `horizon-mixin.toml` and the Rust that hooks the game |
| `mixins/bg2/data/bg2-content.hmod` | `bg2-content` packed as a plain zip, loaded by the native mod at start, so one `.hnmod` holds everything |
| `bg2-content/` | the source of that file: the gadgets' and blocks' declarations, models, textures and recipes |
| `horizon-native.json` | the registry manifest: the game version, the hooks API and the Horizon commit this is built for |
| `docs/hook-map.md` | every part of the Java mod and the Horizon hook it lands on |
| `tools/make_content_mod.py` | regenerates `bg2-content` from the Java mod's jar |
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

This is a Horizon native mod bundle. Native mods are **unsandboxed code that
runs with your rights** — read what you build.

A bundle builds against one exact Horizon commit, named in
`horizon-native.json` and in `mixins/bg2/horizon-mixin.toml`. Both say
`TODO-pin-a-nightly-commit` until the first milestone is reached.

Put each bundle's `mixins/<id>` folder in one folder, then:

```bash
horizon-launcher native-build "path/to/that/folder" --trust-native-code
```

The launcher compiles the game with the mods you have installed.

## Differences from the Java mod

Written down as they are found. The ones already known from the plan:

- **No Curios integration.** Curios does not exist on Horizon, so a gadget
  cannot sit in a Curios slot.
- **No Patchouli book.** Patchouli does not exist on Horizon. The in-game guide
  is either drawn by the mod itself or dropped.
- **No Java addon API.** `GadgetModes.registerMode` let other Java mods add
  modes. Nothing Java runs here.
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
