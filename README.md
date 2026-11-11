# Building Gadgets 2 for Horizon

An unofficial port of [Building Gadgets 2](https://github.com/Direwolf20-MC/BuildingGadgets2)
(MIT, by direwolf20) to [Horizon](https://github.com/horizon-rust/horizon), the
Rust rewrite of Minecraft: Java Edition.

Horizon runs no Java, so this is not the jar repackaged: the mod's behaviour is
rewritten in Rust against Horizon's hook points, and its models, textures,
names and recipes are carried over from the Java mod.

> **In progress.** The Building Gadget and the Exchanger work: pick a block,
> choose a mode, see the preview, build. Undo, copy/paste, the destruction
> gadget and the screens are not written yet.

## What works

Tested in a running game on a native build, not only compiled:

- **The Building Gadget**, with all eight of its modes — Build to Me, Vertical
  Wall, Vertical Column, Surface, Horizontal Wall, Stairs, Horizontal Row and
  Grid.
- **The Exchanger**, with its four, replacing blocks in place and handing the
  old ones back.
- **Picking a block** with shift-right-click, at up to 32 blocks — far past
  vanilla's reach, which is the point of the gadget.
- **The preview**: what a click would build, drawn in the world before it
  happens. Translucent boxes for now, not textured block models.
- **The settings** the modes read: range, place on top, fuzzy, connected area,
  affect tiles.
- **Materials**: taken from the inventory outside creative, and the build stops
  where they run out.

## What does not, yet

- **Undo**, copy and paste, the Cut-Paste and Destruction gadgets, templates
  and redprints.
- **The screens**: the radial mode menu, the copy/paste and material-list
  screens, the template manager, and the four keybinds. Until they exist,
  `/bg2 mode | range | toggle` reaches a gadget's settings instead.
- **Textured ghost blocks.** The preview draws boxes; the Java mod draws
  translucent block models.
- **The build animation.** Blocks appear at once, where the Java mod places
  them a few per tick behind its render block.
- **Energy.** The Java gadget costs Forge Energy per block. Horizon has none,
  and whether this port charges anything at all is still open.
- **Protection checks.** `Host::may_build` is not in any released Horizon yet,
  so a gadget ignores claims and spawn protection. That matters on a server,
  not in single player.
- **No Curios, no Patchouli, no Java addon API** — none of them exist on
  Horizon.

[docs/hook-map.md](docs/hook-map.md) lists every part of the Java mod and what
it lands on, including the differences above and why.

## Installing

Needs Horizon with a prepared instance. Three steps, because this launcher
reads a combined `.hmod`'s code but not its content:

1. the mod into `<game folder>/horizon-native-mods/`;
2. its content into `<game folder>/horizon-mods/`;
3. `mod.buildinggadgets2:true` in `<game folder>/horizon-mods.txt`, or the mod
   is quietly left out of the build.

Then build the game with it:

```bash
horizon-launcher native-build --instance <instance> --trust-native-code
```

**A native mod is not sandboxed: its code is compiled into the game and runs
with your rights.** Read what you build; `--trust-native-code` is you saying
you did.

From a checkout, `python tools/dev_install.py` does the first three steps and
prints the fourth.

## Building it yourself

Needs Rust 1.90 or later, Python 3.10 or later, and a Horizon source tree —
the launcher keeps the one it builds from in
`<launcher data>/native/source/<commit>/`.

```bash
python tools/local_check.py --horizon <horizon tree> --tests
```

```bash
python tools/check_bundle.py --horizon <horizon tree>
```

```bash
python tools/dev_install.py --build --trust
```

[docs/developing.md](docs/developing.md) has the loop, the three things that
cost the most time to find, and how the in-game checks run.

## How it is put together

| Path | What it is |
|---|---|
| `mixins/buildinggadgets2/` | the native mod: a trait mod, an ordinary Rust crate implementing `horizon_client::mod_api::Hooks` |
| `bg2-content/` | the blocks' and items' declarations, and the models, textures, names, recipes and loot carried over from the Java mod |
| `horizon-native.json` | the registry manifest: the game version, the hook-point API and the Horizon commit this was tested on |
| `docs/hook-map.md` | every part of the Java mod and the Horizon hook it lands on |
| `docs/developing.md` | the build loop and the traps in it |
| `tools/` | convert the content, check the bundle, compile against a local Horizon, install, and drive the in-game checks |
| `PLAN.md` | the working journal: what each round tried and what it turned up |
| `TODO.md` | the milestones and what is left under each |

It is a **trait mod** rather than a [horizon-mixins](https://github.com/horizon-rust/horizon-mixins)
bundle, because every part of the mapping is a named hook point and nothing
needs the game's source rewritten. That also makes it unpinned, switchable in
Options → Mods, and compilable without a native build.

## Tests

79 unit tests, which need no game: the modes, the raycast, the bulk-edit runs
and the ids. The mode geometry has to match the Java mod block for block, so
each test states what the Java does for that input. They are checked for teeth
as well as for green — breaking `invertOnFace` and the grid spacing on purpose
makes exactly the two tests that cover them fail, and nothing else.

`tools/ingame_check.py` runs the same checks in a world through `horizon-ctl`,
with every expected count worked out from the mode rather than read off a run.

## Credits and licence

- **Building Gadgets 2**, © 2023 [Direwolf20-MC](https://github.com/Direwolf20-MC/BuildingGadgets2),
  [MIT](LICENSE). This port is a rewrite of that mod's behaviour and carries
  over its assets, under the same licence.
- **Horizon** and its mod API: the [horizon-rust](https://github.com/horizon-rust)
  project, MIT.
- The **AE2 Horizon port** was the reference for how a Java mod becomes a
  native one.

An unofficial port, not made or endorsed by direwolf20. Please report problems
here, not to them.
