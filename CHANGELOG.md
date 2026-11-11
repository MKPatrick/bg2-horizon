# Changelog

Nothing is released yet. Until the first release this records what each
milestone added; [PLAN.md](PLAN.md) has the detail and the wrong turns.

## Unreleased

Built and played on Horizon nightly-2026-11-08 (`fdc87ae`, hook-point API
411), Minecraft 26.3.

### The gadgets

- The **Building Gadget** places blocks in all eight of its modes: Build to
  Me, Vertical Wall, Vertical Column, Surface, Horizontal Wall, Stairs,
  Horizontal Row and Grid.
- The **Exchanger** replaces blocks in place in its four modes and hands the
  old blocks back.
- **Shift-right-click picks** the block you look at, up to 32 blocks away —
  the gadget raycasts itself rather than using the player's reach.
- **A preview** of what a click would build, drawn in the world: a translucent
  box per block and an outline around the build.
- **Materials** are counted and taken outside creative, and a build stops
  where they run out.
- The settings the modes read: range, place on top, fuzzy, connected area,
  affect tiles.
- `/bg2 info | mode | range | toggle`, which reaches a gadget's settings until
  the screens exist. Not in the Java mod.

### Content

- The Java mod's items, blocks, models, textures, recipes, loot tables and
  five languages, carried over by `tools/make_content_mod.py`.
- The six `c:` item tags its recipes need, which Horizon does not ship.

### Known differences from the Java mod

Listed in full in [docs/hook-map.md](docs/hook-map.md) and under each milestone
in [TODO.md](TODO.md). The ones a player would notice:

- the preview draws boxes, not textured block models;
- blocks appear at once, with no build animation;
- no energy cost;
- no protection check (`Host::may_build` is not in a released Horizon yet), so
  a gadget ignores claims and spawn protection on a server;
- no Curios, Patchouli or Java addon support, none of which exist on Horizon.

### Not written yet

Undo, copy and paste, the Cut-Paste and Destruction gadgets, templates and
redprints, and every screen including the radial mode menu and the keybinds.
