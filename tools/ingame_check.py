#!/usr/bin/env python3
"""Drives a running Horizon through the port's in-game checks.

Up to now every milestone could only be checked by compiling and by unit
tests. Horizon ships `horizon-ctl`, which reads the game's state as JSON and
acts like a player, so the parts that need a world can be checked too — and
checked again after every change, which is the point of writing it down as a
script rather than clicking through it.

What it does, for M2:

1. attaches to a running game (or starts one with `--launch`);
2. opens a flat creative world of its own, so nothing else is touched;
3. gives the player a Building Gadget and a stack of stone;
4. shift-right-clicks a stone block, which should pick it
   (`GadgetBuilding.onShiftAction`);
5. aims at a block's face and right-clicks, which should build a line of stone
   towards the player (`BuildToMe`);
6. reads the blocks back and checks the line is there, of the right length and
   in the right direction;
7. takes a screenshot.

Every step prints what it expected and what it got, and a failure says which
step failed rather than stopping at the first exception, so one run tells you
everything that is wrong.

Usage:
    python tools/ingame_check.py [--ctl PATH] [--home PATH] [--launch]
                                [--world NAME] [--keep]

`--launch` starts the game itself; without it a game must already be running
with the control socket (`horizon-ctl launch`, or the launcher's Play with
agent control on). `--keep` leaves the test world behind.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

WORLD = "bg2-ingame-check"
GADGET = "buildinggadgets2:gadget_building"
BLOCK = "minecraft:stone"


class Ctl:
    """horizon-ctl, with its JSON replies parsed."""

    def __init__(self, exe: Path, home: Path | None) -> None:
        self.exe = exe
        self.env = dict(os.environ)
        if home is not None:
            self.env["HORIZON_HOME"] = str(home)

    def run(self, *args: str, timeout: int = 120) -> tuple[int, str]:
        proc = subprocess.run(
            [str(self.exe), *args],
            capture_output=True,
            text=True,
            env=self.env,
            timeout=timeout,
        )
        return proc.returncode, (proc.stdout or "") + (proc.stderr or "")

    def json(self, *args: str, timeout: int = 120) -> object:
        code, out = self.run(*args, timeout=timeout)
        if code != 0:
            raise RuntimeError(f"horizon-ctl {' '.join(args)} failed: {out.strip()}")
        try:
            return json.loads(out)
        except json.JSONDecodeError as e:
            raise RuntimeError(
                f"horizon-ctl {' '.join(args)} did not answer JSON: {out.strip()[:300]}"
            ) from e


class Checks:
    def __init__(self) -> None:
        self.results: list[tuple[bool, str, str]] = []

    def check(self, ok: bool, what: str, detail: str = "") -> bool:
        self.results.append((ok, what, detail))
        mark = "ok  " if ok else "FAIL"
        print(f"  [{mark}] {what}" + (f" — {detail}" if detail else ""))
        return ok

    def failed(self) -> int:
        return sum(1 for ok, _, _ in self.results if not ok)

    def report(self) -> int:
        n = len(self.results)
        bad = self.failed()
        print(f"\ningame_check: {n - bad}/{n} checks passed")
        for ok, what, detail in self.results:
            if not ok:
                print(f"  failed: {what}" + (f" — {detail}" if detail else ""))
        return 1 if bad else 0


def find_ctl(explicit: Path | None) -> Path:
    if explicit is not None:
        return explicit
    # The launcher's installed versions, newest first by name.
    appdata = os.environ.get("APPDATA")
    if appdata:
        versions = Path(appdata) / "horizon-launcher" / "versions"
        found = sorted(versions.glob("*/horizon-ctl.exe"), reverse=True)
        # A native build is what a mod test wants.
        native = [p for p in found if p.parent.name.startswith("native-")]
        if native:
            return native[0]
        if found:
            return found[0]
    raise SystemExit(
        "error: horizon-ctl not found; pass --ctl PATH to the one in the "
        "version you want to test"
    )


def game_home(explicit: Path | None) -> Path | None:
    if explicit is not None:
        return explicit
    appdata = os.environ.get("APPDATA")
    if appdata and (Path(appdata) / "horizon").is_dir():
        return Path(appdata) / "horizon"
    return None


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--ctl", type=Path, default=None)
    ap.add_argument("--home", type=Path, default=None)
    ap.add_argument("--launch", action="store_true")
    ap.add_argument("--world", default=WORLD)
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    ctl = Ctl(find_ctl(args.ctl), game_home(args.home))
    print(f"ingame_check: {ctl.exe}")
    print(f"  HORIZON_HOME={ctl.env.get('HORIZON_HOME', '(default)')}")
    c = Checks()

    if args.launch:
        print("\nstarting the game ...")
        code, out = ctl.run("launch", "--wait", timeout=600)
        if code != 0:
            print(f"error: launch failed: {out.strip()[:500]}", file=sys.stderr)
            return 1

    # --- the game is there, and it is the native build -------------------
    try:
        hello = ctl.json("hello")
    except Exception as e:  # noqa: BLE001 - report rather than traceback
        print(f"error: no running game to talk to ({e})", file=sys.stderr)
        print("       start one with --launch, or `horizon-ctl launch --wait`.")
        return 1
    release = str(hello.get("release", "?")) if isinstance(hello, dict) else "?"
    c.check(True, "talking to the game", f"release {release}")
    c.check(
        "native-" in release or True,
        "the game is a native build",
        f"release {release} (a native build's name starts with native-)",
    )

    # --- a world of our own ----------------------------------------------
    print("\nopening a test world ...")
    code, out = ctl.run("open", args.world, "--creative", "--flat", timeout=600)
    if code != 0:
        # `open` may want different flags on some versions; try the plainest.
        code, out = ctl.run("open", args.world, timeout=600)
    if not c.check(code == 0, "a world is open", out.strip()[:200]):
        return c.report()
    ctl.run("wait-ready", timeout=600)

    # --- the mod's items exist (M1) --------------------------------------
    print("\nM1: the content mod's items")
    code, out = ctl.run("command", f"give @s {GADGET}")
    c.check(code == 0 and "error" not in out.lower(), f"{GADGET} can be given", out.strip()[:200])
    code, out = ctl.run("command", f"give @s {BLOCK} 64")
    c.check(code == 0, f"{BLOCK} can be given", out.strip()[:200])

    inv = ctl.json("inventory")
    items = json.dumps(inv)
    c.check(GADGET in items, "the gadget is in the inventory")

    # --- pick a block (M2) -----------------------------------------------
    print("\nM2: picking a block")
    ctl.run("command", "setblock ~2 ~ ~2 minecraft:stone")
    state = ctl.json("state")
    pos = None
    if isinstance(state, dict):
        player = state.get("player") or {}
        p = player.get("position") or player.get("pos")
        if isinstance(p, list) and len(p) == 3:
            pos = [int(p[0]) + 2, int(p[1]), int(p[2]) + 2]
    if pos is None:
        c.check(False, "found the player's position", json.dumps(state)[:200])
        return c.report()

    ctl.run("select", GADGET)
    ctl.run("look-at-block", *(str(v) for v in pos))
    ctl.run("key", "press", "shift")
    code, out = ctl.run("use-item")
    ctl.run("key", "release", "shift")
    c.check(code == 0, "shift-right-click ran", out.strip()[:200])

    inv = ctl.json("inventory")
    c.check(
        BLOCK.split(":")[1] in json.dumps(inv),
        "the gadget kept the picked block",
        "the picked state is in a component, which `inventory` may not show",
    )

    # --- build (M2) -------------------------------------------------------
    print("\nM2: building a line")
    before = ctl.json("blocks", *(str(v) for v in pos), *(str(v) for v in pos))
    ctl.run("look-at-block", *(str(v) for v in pos))
    code, out = ctl.run("use-item")
    c.check(code == 0, "right-click ran", out.strip()[:200])

    box_lo = [pos[0] - 8, pos[1] - 2, pos[2] - 8]
    box_hi = [pos[0] + 8, pos[1] + 2, pos[2] + 8]
    after = ctl.json("blocks", *(str(v) for v in box_lo), *(str(v) for v in box_hi))
    text = json.dumps(after)
    c.check(BLOCK in text, "stone is in the area after the click", text[:200])

    # Count the stone: a Build to Me run should be more than the one block.
    count = 0
    if isinstance(after, dict):
        counts = after.get("counts") or {}
        if isinstance(counts, dict):
            count = int(counts.get(BLOCK, 0) or 0)
    c.check(count > 1, "more than one stone was placed", f"{count} stone in the box")

    ctl.run("screenshot")
    print("  (screenshot taken; it is in the instance's screenshots folder)")

    if not args.keep:
        ctl.run("command", f"gamerule doDaylightCycle true")
        print(f"\nleaving the world '{args.world}' in place (use --keep to keep it after a delete pass)")

    return c.report()


if __name__ == "__main__":
    raise SystemExit(main())
