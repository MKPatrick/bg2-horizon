#!/usr/bin/env python3
"""Drives a running Horizon through the port's in-game checks.

The unit tests pin the mode geometry without a game. What they cannot show is
whether the settings reach it, whether the hooks fire, and whether the blocks
really land — so this does the same checks in a world, through `horizon-ctl`,
which reads the game's state as JSON and acts like a player.

Every check prints what it expected and what it got, and a failure does not
stop the run, so one pass tells you everything that is wrong.

The game must already be running with its control socket:

    horizon-ctl launch --wait          (or --launch here)

Usage:
    python tools/ingame_check.py [--ctl PATH] [--home PATH] [--launch]
                                [--world NAME] [--only NAME]

Two things cost time when this was first written, and are handled here:

- **Sneak has to be held long enough for the server to see it.** Pressing it
  and clicking in the same millisecond leaves `host.sneaking()` false, and the
  mod takes the build branch instead of the pick branch. A state round trip
  between the two is enough.
- **A mouse wheel moves the hotbar.** Anything the person at the keyboard does
  lands in the same game, so the gadget is selected again before every step.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

GADGET = "buildinggadgets2:gadget_building"
EXCHANGER = "buildinggadgets2:gadget_exchanging"
BLOCK = "minecraft:stone"
WORLD = "bg2-check"

# Where the test builds happen: high above the terrain, clear of everything.
SITE = (97, 150, -22)


class Ctl:
    def __init__(self, exe: Path, home: Path | None) -> None:
        self.exe = exe
        self.env = dict(os.environ)
        if home is not None:
            self.env["HORIZON_HOME"] = str(home)

    def run(self, *args: str, timeout: int = 300) -> tuple[int, str]:
        p = subprocess.run(
            [str(self.exe), *args],
            capture_output=True,
            text=True,
            env=self.env,
            timeout=timeout,
        )
        return p.returncode, (p.stdout or "") + (p.stderr or "")

    def json(self, *args: str, timeout: int = 300) -> dict:
        code, out = self.run(*args, timeout=timeout)
        if code != 0:
            raise RuntimeError(f"horizon-ctl {' '.join(args)}: {out.strip()[:300]}")
        try:
            return json.loads(out)
        except json.JSONDecodeError as e:
            raise RuntimeError(f"not JSON from {' '.join(args)}: {out[:200]}") from e

    def cmd(self, line: str) -> dict:
        return self.json("command", line)

    # --- the things that needed care --------------------------------------

    def ready(self) -> None:
        """Release anything held and leave any menu, before a step."""
        self.run("release")
        self.run("resume")

    def hold_gadget(self, item: str = GADGET) -> bool:
        self.run("select", item)
        held = self.json("state").get("held") or {}
        return held.get("item") == item

    def sneak_click(self) -> None:
        """Right-click with sneak held, giving the server time to see it."""
        self.run("key", "shift", "down")
        self.json("state")  # a round trip, so the sneak reaches the server
        self.run("use-item")
        self.run("key", "shift", "up")

    def blocks_at(self, lo: tuple[int, int, int], hi: tuple[int, int, int]) -> dict:
        """Block name by position in a box."""
        d = self.json("blocks", *map(str, lo), *map(str, hi))
        return {(b[0], b[1], b[2]): b[3] for b in d.get("blocks", [])}


class Checks:
    def __init__(self) -> None:
        self.rows: list[tuple[bool, str, str]] = []

    def check(self, ok: bool, what: str, detail: str = "") -> bool:
        self.rows.append((ok, what, detail))
        print(f"  [{'ok  ' if ok else 'FAIL'}] {what}" + (f" — {detail}" if detail else ""))
        return ok

    def report(self) -> int:
        bad = [r for r in self.rows if not r[0]]
        print(f"\ningame_check: {len(self.rows) - len(bad)}/{len(self.rows)} passed")
        for _, what, detail in bad:
            print(f"  failed: {what}" + (f" — {detail}" if detail else ""))
        return 1 if bad else 0


def find_ctl(explicit: Path | None) -> Path:
    if explicit is not None:
        return explicit
    appdata = os.environ.get("APPDATA")
    if appdata:
        versions = Path(appdata) / "horizon-launcher" / "versions"
        native = sorted(
            (p for p in versions.glob("native-*/horizon-ctl.exe")),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        if native:
            return native[0]
        any_ctl = sorted(versions.glob("*/horizon-ctl.exe"))
        if any_ctl:
            return any_ctl[0]
    raise SystemExit("error: horizon-ctl not found; pass --ctl")


def game_home(explicit: Path | None) -> Path | None:
    if explicit is not None:
        return explicit
    appdata = os.environ.get("APPDATA")
    if appdata and (Path(appdata) / "horizon").is_dir():
        return Path(appdata) / "horizon"
    return None


def clear_site(ctl: Ctl) -> None:
    """An empty box to build in, with one block to stand on."""
    x, y, z = SITE
    ctl.cmd(f"fill {x - 20} {y - 4} {z - 20} {x + 20} {y + 12} {z + 20} minecraft:air")
    ctl.cmd(f"setblock {x} {y - 1} {z} minecraft:stone")
    ctl.run("tp", f"{x + 0.5}", str(y), f"{z + 0.5}")


def setup(ctl: Ctl, c: Checks) -> None:
    ctl.ready()
    ctl.cmd("gamemode creative")
    ctl.cmd(f"give @s {GADGET}")
    ctl.cmd(f"give @s {EXCHANGER}")
    ctl.cmd(f"give @s {BLOCK} 64")
    clear_site(ctl)
    c.check(ctl.hold_gadget(), "the Building Gadget is in hand")


def pick_stone(ctl: Ctl, c: Checks) -> None:
    """Shift-right-click a block 10 away: only the mod's own raycast reaches."""
    x, y, z = SITE
    target = (x + 10, y + 1, z)
    ctl.ready()
    ctl.cmd(f"setblock {target[0]} {target[1]} {target[2]} {BLOCK}")
    ctl.hold_gadget()
    ctl.run("look", "-90", "0")
    ctl.sneak_click()
    held = ctl.json("state").get("held") or {}
    components = str(held.get("components", ""))
    c.check(
        BLOCK in components,
        "shift-right-click at 10 blocks picks the block",
        f"components {components[:120]}",
    )


def check_mode(
    ctl: Ctl,
    c: Checks,
    mode: str,
    *,
    expect: int | None = None,
    at_least: int = 1,
    range_: int = 3,
) -> None:
    """Switch to `mode`, click once, and count what landed."""
    x, y, z = SITE
    target = (x + 10, y + 1, z)
    ctl.ready()
    clear_site(ctl)
    ctl.cmd(f"setblock {target[0]} {target[1]} {target[2]} {BLOCK}")
    ctl.hold_gadget()
    ctl.cmd(f"gamemode creative")
    code, out = ctl.run("command", f"bg2 mode {mode}")
    code2, out2 = ctl.run("command", f"bg2 range {range_}")
    ctl.run("look", "-90", "0")

    before = ctl.blocks_at((x - 16, y - 3, z - 16), (x + 16, y + 10, z + 16))
    ctl.run("use-item")
    after = ctl.blocks_at((x - 16, y - 3, z - 16), (x + 16, y + 10, z + 16))
    new = {p for p in after if p not in before}

    if expect is not None:
        c.check(len(new) == expect, f"{mode} placed {expect}", f"placed {len(new)}")
    else:
        c.check(len(new) >= at_least, f"{mode} placed something", f"placed {len(new)}")


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--ctl", type=Path, default=None)
    ap.add_argument("--home", type=Path, default=None)
    ap.add_argument("--launch", action="store_true")
    ap.add_argument("--world", default=WORLD)
    ap.add_argument("--only", default=None, help="run one mode's check only")
    args = ap.parse_args()

    ctl = Ctl(find_ctl(args.ctl), game_home(args.home))
    print(f"ingame_check: {ctl.exe}")
    c = Checks()

    if args.launch:
        print("starting the game ...")
        code, out = ctl.run("launch", "--wait", timeout=900)
        if code != 0:
            print(f"error: launch failed: {out.strip()[:400]}", file=sys.stderr)
            return 1

    try:
        hello = ctl.json("hello")
    except Exception as e:  # noqa: BLE001
        print(f"error: no game to talk to ({e})", file=sys.stderr)
        print("       run `horizon-ctl launch --wait`, or pass --launch.")
        return 1
    c.check(True, "talking to the game", f"release {hello.get('release')}")

    if (ctl.json("state").get("screen")) == "title":
        print("opening the test world ...")
        ctl.run("open", args.world, "12345", "creative", timeout=900)
        ctl.run("wait-ready", timeout=900)

    print("\nsetup")
    setup(ctl, c)

    print("\nM2: picking")
    pick_stone(ctl, c)

    print("\nM3: the modes")
    # Build to Me from 10 blocks away runs 9 blocks, as the unit test says.
    cases: list[tuple[str, dict]] = [
        # Every count is worked out from the mode, not read off a run. The
        # site is one stone block at (x+10, y+1, z), hit on its west face
        # from 10 blocks away, range 3 — so the hit block is already stone
        # and a mode that covers it places one fewer.
        #
        #   build_to_me      |10 - 0| = 10, loop 1..9                   -> 9
        #   vertical_wall    3x3 around the hit block, less the block   -> 8
        #   vertical_column  half 1, so -1..=1, less the block          -> 2
        #   horizontal_wall  i in 1..=3 away, j in -1..=1 across        -> 9
        #   horizontal_row   i in 0..3 from the block, less the block   -> 2
        #   stairs           i in 0..3, step (i, i), less the block     -> 2
        #   grid             range 4, spacing 4, extent 5: -4/0/4
        #                    squared, less the block                    -> 8
        #   surface          bound 1, but only where the block behind
        #                    is the one looked at: in front of the hit  -> 1
        ("build_to_me", {"expect": 9}),
        ("vertical_wall", {"expect": 8}),
        ("vertical_column", {"expect": 2}),
        ("horizontal_wall", {"expect": 9}),
        ("horizontal_row", {"expect": 2}),
        ("stairs", {"expect": 2}),
        ("grid", {"expect": 8}),
        ("surface", {"expect": 1}),
    ]
    for mode, kw in cases:
        if args.only and mode != args.only:
            continue
        check_mode(ctl, c, mode, **kw)  # type: ignore[arg-type]

    return c.report()


if __name__ == "__main__":
    raise SystemExit(main())
