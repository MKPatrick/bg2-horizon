#!/usr/bin/env python3
"""`cargo check` the trait mod against a local Horizon source tree.

The launcher builds the mod by pointing its `horizon-client` dependency at the
Horizon source it is building the game from. This does the same thing locally,
so a mistake in the mod's code is found in minutes instead of after a native
build.

Two things have to be replaced for that to work:

- `horizon-client`, which `api/Cargo.toml` names as a git dependency, becomes
  the local tree's `crates/horizon-client`;
- the three crates Horizon patches to vendored copies in its own root
  `Cargo.toml` (`wgpu-hal`, `naga`, `libmimalloc-sys`). Cargo only honours a
  `[patch]` table from the workspace root, and the root here is the mod's own
  package, so without these the build picks the crates.io versions and fails
  inside `horizon-client-render` (`wgpu::hal::dx12::disk_cache` not found).

The patches are passed with `--config`, so nothing in the repository has to
carry a path that is only right on one machine.

Usage:
    python tools/local_check.py [--horizon PATH] [--bundle PATH]
                               [--release] [--tests] [-- <extra cargo args>]

--horizon defaults to ../horizon-main next to this repository. --tests runs
`cargo test` instead, for the mod's own unit tests.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

# The crates Horizon's root Cargo.toml patches to third_party/, which a
# standalone build of the mod has to patch the same way.
VENDORED = ("wgpu-hal", "naga", "libmimalloc-sys")

# The git URL api/Cargo.toml names for horizon-client.
HORIZON_GIT = "https://github.com/horizon-rust/horizon"


def cargo_config(horizon: Path) -> list[str]:
    """`--config` arguments replacing the git dependency and the vendored crates."""
    horizon = horizon.resolve()
    client = (horizon / "crates/horizon-client").as_posix()
    args = [
        "--config",
        f'patch."{HORIZON_GIT}".horizon-client.path="{client}"',
    ]
    for name in VENDORED:
        path = (horizon / "third_party" / name).as_posix()
        args += ["--config", f'patch.crates-io.{name}.path="{path}"']
    return args


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--horizon", type=Path, default=repo.parent / "horizon-main")
    ap.add_argument("--bundle", type=Path, default=repo / "mixins/buildinggadgets2")
    ap.add_argument("--release", action="store_true")
    ap.add_argument(
        "--tests", action="store_true", help="cargo test instead of cargo check"
    )
    ap.add_argument("extra", nargs="*", help="further arguments for cargo")
    args = ap.parse_args()

    if shutil.which("cargo") is None:
        print(
            "error: cargo is not on PATH. Horizon needs Rust 1.90 or later; "
            "install it with rustup and reopen the shell.",
            file=sys.stderr,
        )
        return 1

    api = args.bundle / "api"
    if not (api / "Cargo.toml").is_file():
        print(f"error: {api / 'Cargo.toml'} does not exist", file=sys.stderr)
        return 1
    for needed in ("crates/horizon-client", *(f"third_party/{n}" for n in VENDORED)):
        if not (args.horizon / needed).is_dir():
            print(
                f"error: {args.horizon / needed} does not exist: is --horizon a "
                f"Horizon source tree?",
                file=sys.stderr,
            )
            return 1

    cmd = ["cargo", "test" if args.tests else "check"]
    cmd += cargo_config(args.horizon)
    if args.release:
        cmd.append("--release")
    cmd += args.extra

    print(f"$ {' '.join(cmd)}\n  in {api}")
    return subprocess.call(cmd, cwd=api)


if __name__ == "__main__":
    raise SystemExit(main())
