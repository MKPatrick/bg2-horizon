#!/usr/bin/env python3
"""Installs the mod into a local Horizon instance, ready for a native build.

Getting the mod into a running game took three things that are easy to miss,
so they are written down here rather than in a comment somewhere:

1. **A native mod has to be turned on.** `docs/native-mods.md` says native mods
   "are always on in the launcher: there is no switch", but the instance's
   `horizon-mods.txt` lists them like content mods and only the listed ones are
   built. Without `mod.buildinggadgets2:true` the launcher answers "Up to date
   (no build needed)" and silently keeps the old build.

2. **Both halves have to be installed.** The combined `.hmod` this repository
   packs holds content and code, which is what a registry release wants, but
   this launcher takes only its code: the built version has no
   `native-content/<id>/`, so the blocks and items would not exist — and a
   trait mod cannot declare them in code. So the code goes to
   `horizon-native-mods/` and the content separately to `horizon-mods/`.

3. **The launcher builds the installed file, not the source.** Editing
   `api/src/*.rs` and running `native-build` again changes nothing: the build
   key comes from the installed `.hmod`. Every change has to be packed and
   copied in again, which is what this script is for.

Usage:
    python tools/dev_install.py [--home PATH] [--instance NAME] [--build]
                               [--trust]

`--build` runs the native build afterwards; it needs `--trust`, which stands
for `--trust-native-code`, because the build compiles this mod's code into the
game and runs it with your rights. Without `--trust` the command to run is
printed instead.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

MOD_ID = "buildinggadgets2"


def game_home(explicit: Path | None) -> Path:
    if explicit is not None:
        return explicit
    appdata = os.environ.get("APPDATA")
    if appdata and (Path(appdata) / "horizon").is_dir():
        return Path(appdata) / "horizon"
    raise SystemExit(
        "error: the game folder was not found; pass --home (the folder holding "
        "options.txt)"
    )


def launcher() -> Path | None:
    local = os.environ.get("LOCALAPPDATA")
    if local:
        exe = Path(local) / "Programs" / "Horizon Launcher" / "horizon-launcher.exe"
        if exe.is_file():
            return exe
    found = shutil.which("horizon-launcher")
    return Path(found) if found else None


def run(cmd: list[str], what: str, cwd: Path | None = None) -> None:
    print(f"  {what}")
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    if proc.returncode != 0:
        out = (proc.stdout or "") + (proc.stderr or "")
        raise SystemExit(f"error: {what} failed:\n{out.strip()[:800]}")


def turn_on(home: Path, mod_id: str) -> None:
    """Add `mod.<id>:true` to horizon-mods.txt if it is not there."""
    path = home / "horizon-mods.txt"
    lines = (
        path.read_text(encoding="utf-8").splitlines() if path.is_file() else []
    )
    key = f"mod.{mod_id}:"
    if any(line.startswith(key) for line in lines):
        on = [l for l in lines if l.startswith(key)][0]
        if on.endswith(":true"):
            print(f"  already on: {on}")
            return
        lines = [f"{key}true" if l.startswith(key) else l for l in lines]
        print(f"  turning on: {on} -> {key}true")
    else:
        lines.append(f"{key}true")
        print(f"  turning on: {key}true (was not listed, so it was not built)")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--home", type=Path, default=None)
    ap.add_argument("--instance", default="horizon")
    ap.add_argument("--build", action="store_true")
    ap.add_argument(
        "--trust",
        action="store_true",
        help="pass --trust-native-code to the build: this mod's code is "
        "compiled into the game and runs with your rights",
    )
    args = ap.parse_args()

    home = game_home(args.home)
    print(f"dev_install: game folder {home}")

    print("\npacking:")
    run(
        [sys.executable, "-I", str(repo / "tools/pack_mod.py")],
        "the combined .hmod (code, for horizon-native-mods)",
    )
    content_dir = repo / "bg2-content"
    run(
        ["cargo", "horizon-mod", "build"],
        "the content .hmod (for horizon-mods)",
        cwd=content_dir,
    )

    print("\ninstalling:")
    code = repo / "dist" / f"{MOD_ID}.hmod"
    content = content_dir / "target" / "horizon-mod" / f"{MOD_ID}.hmod"
    for src, dest_dir, what in (
        (code, home / "horizon-native-mods", "code"),
        (content, home / "horizon-mods", "content"),
    ):
        if not src.is_file():
            raise SystemExit(f"error: {src} does not exist (pack it first)")
        dest_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest_dir / src.name)
        print(f"  {what}: {src.name} -> {dest_dir}")

    print("\nturning the mod on:")
    turn_on(home, MOD_ID)

    exe = launcher()
    cmd = [
        str(exe) if exe else "horizon-launcher",
        "native-build",
        "--instance",
        args.instance,
        "--trust-native-code",
    ]
    if args.build and args.trust:
        print("\nbuilding (this compiles the mod into the game):")
        proc = subprocess.run(cmd, text=True)
        return proc.returncode
    print("\nnot built. The build compiles this mod's code into the game and")
    print("runs it with your rights, so it is yours to start:")
    print("  " + " ".join(f'"{c}"' if " " in c else c for c in cmd))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
