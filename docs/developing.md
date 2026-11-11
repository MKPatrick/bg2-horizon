# Developing this port

What you need, the loop, and the three things that cost hours to find.

## What you need

| | |
|---|---|
| Rust 1.90+ | for `cargo check` and the unit tests. The launcher uses its **own** toolchain for a native build and never the system one, so this is only for checking |
| Python 3.10+ | the tools in `tools/` |
| A Horizon source tree | the launcher keeps the one it builds from in `<launcher data>/native/source/<commit>/`. On Windows that is `%APPDATA%\horizon-launcher\native\source\<commit>` |
| Horizon, installed and prepared | only to run it. The launcher, a signed-in account and a prepared instance |
| `cargo-horizon-mod` | Horizon's content-mod tool, for `bg2-content`. Build it from a Horizon checkout: `cargo build -p cargo-horizon-mod` |

No JDK is needed for this port. Horizon's own `horizon-prepare` wants one, but
the launcher has already done that by the time you have an instance.

## The loop

```bash
python tools/local_check.py --horizon <horizon tree> --tests
```

About 90 seconds once the dependencies are built, and it runs the unit tests.
This is where nearly all the work happens: the mode geometry, the raycast and
the bulk-edit runs are written against a `Blocks` trait with a hand-written
world behind it, so they need no game.

```bash
python tools/check_bundle.py --horizon <horizon tree>
```

Reads Horizon's own rules out of its source — the manifest's schema, the hook
catalog, the API number each hook and helper needs — and says what the bundle
gets wrong before a build finds out.

```bash
python tools/dev_install.py --build --trust
```

Packs, installs both halves, turns the mod on, and builds. Four to six minutes
on the `dev` profile.

```bash
python tools/ingame_check.py
```

Drives a running game through the checks a world is needed for.

## The three traps

### 1. The launcher builds the installed file, not your source

Editing `api/src/*.rs` and running `native-build` again answers **"Up to date
(no build needed)"** and keeps the old build. The build key comes from the
installed `.hmod`, so every change has to be packed and copied in again.
`tools/dev_install.py` is that step.

### 2. A native mod has to be turned on

`docs/native-mods.md` says native mods "are always on in the launcher: there is
no switch". The instance's `horizon-mods.txt` lists them anyway, and **only the
listed ones are built** — without `mod.buildinggadgets2:true` the launcher
reuses the old build and says nothing. It is also why a mod you dropped into
`horizon-native-mods/` by hand may never appear.

### 3. A combined `.hmod` loses its content here

`docs/api/mod-files.md` describes one `.hmod` holding both manifests, read as
content plus code. This launcher takes the code and ignores the content: the
built version has no `native-content/<id>/`, and a trait mod cannot declare
blocks and items itself, so nothing would exist. The content goes in
separately, to `horizon-mods/`.

## Check against the Horizon you will build on

`horizon-main`, or whatever snapshot you read the docs from, can be **ahead of
every released Nightly**. This port was written against API 415 and then found
that the newest Nightly was 411, missing two things it used: `Host::may_build`
and `NativeBlockEdit.neighbour_updates`.

Always point `--horizon` at the tree in `<launcher data>/native/source/`. That
is what the build uses, and compiling against it is the only way to find a
helper that is not there yet.

## Driving the game

`horizon-ctl`, beside the game in its version folder, reads the game's state as
JSON and acts like a player. `tools/ingame_check.py` uses it. Two things it has
to work around, both learnt the hard way:

- **Sneak has to be held long enough for the server to see it.** Pressing it
  and clicking in the same millisecond leaves `host.sneaking()` false on the
  server, and the mod takes the build branch instead of the pick branch. A
  state round trip between the two is enough.
- **A person at the keyboard shares the game.** A mouse-wheel scroll moves the
  hotbar off the gadget, and the log says so (`motion input from a person while
  an agent was driving`). Select the gadget again before each step.

The game's own `stdout` and `stderr` — where a mod's `eprintln!` lands — go to
the log beside the control socket (`%TEMP%\horizon-control.log` on Windows),
**not** to `logs/latest.log`.

## Debug logging

Set `BG2_DEBUG` in the environment the game starts in and the mod writes a line
per gadget click: which hook fired, what the player looked like, what the
gadget held, and what came of it. It is off otherwise.

It is worth reaching for early. The first in-game run of this port did nothing
and said nothing; one line per click found the cause immediately, and the
guess that had looked obvious from the documentation turned out to be wrong.

## Where the knowledge is

- [hook-map.md](hook-map.md) — every part of the Java mod, the Horizon hook it
  lands on, and the differences.
- [../PLAN.md](../PLAN.md) — the working journal. Each round says what was
  tried and what it turned up, including the wrong turns, which is usually the
  useful part.
- [../TODO.md](../TODO.md) — the milestones and what is left.
