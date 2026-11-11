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

Every hook the port needs already exists. Nothing here waits on a Horizon
change, with one exception worth knowing: `can_build` / `may_build` (the
protection check) is documented with an unstamped API placeholder, and
`world_primitives` is a trait-mod API that a mixin bundle cannot call — the
preview pass draws its own lines instead.

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
| **M1** | **It loads.** Content mod packed and loaded from the bundle; the 8 items and 2 blocks exist; the creative tab shows them with the right models. | A build boots and the gadgets can be held. |
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

- **One `.hnmod`, no separate content mod.** The packed `bg2-content.hmod` ships
  inside the bundle and is loaded at start, as AE2 does it. A player installs
  one file.
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
