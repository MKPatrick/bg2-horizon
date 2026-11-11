#!/usr/bin/env python3
"""Packs the mod into one `.hmod` holding both its content and its code.

Horizon reads three shapes of mod file (docs/api/mod-files.md):

- content only: `horizon-mod.toml` with `assets/` and `data/`;
- code only (`.hnmod`): `horizon-native.json` with `mixins/<id>/`;
- **both, in one `.hmod`**: the two manifests naming the same mod id. Content
  entries are checked as content and left out of the build.

The third is what this writes, so a player installs one file and the models and
textures come with the code. It is also why the bundle lives at
`mixins/buildinggadgets2/` in this repository: that is the path the packed file
must use, so the layout here and the layout in the file are the same.

AE2's port predates combined files and works around it by packing a `.hmod`
into its `.hnmod` and loading it at start with `native_data!`. This needs none
of that.

The rules enforced here are the reader's, from docs/api/mod-files.md:

- under `mixins/<id>/`, only `.rs` and `.toml` files, files under a `data/`
  folder, and `README*`/`LICENSE*`/`LICENCE*`/`COPYING*`/`NOTICE*`/`.gitignore`
  — so `api/target/` and `api/Cargo.lock` are left out (`.lock` is not an
  allowed extension);
- content paths are `assets/` or `data/` then at least two parts of
  `[a-z0-9_.-]`;
- native entries: at most 512, 64 MiB unpacked in total;
- content: at most 65 536 files, 16 MiB each, 512 MiB in total;
- every path appears once, relative, `/`-separated.

Usage:
    python tools/pack_mod.py [--out PATH] [--revision HEX40]

`--revision` replaces `horizon` and `mixins` in the packed manifest without
touching the file in the repository. It exists to test the packing while the
real commit is unknown: `horizon-native.json` needs two 40-character revisions
to be valid, and the repository honestly carries placeholders until the mod has
been built and tested on a commit. Do not publish a file packed this way.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import zipfile
from pathlib import Path

CONTENT_MANIFEST = "horizon-mod.toml"
NATIVE_MANIFEST = "horizon-native.json"

# What a file under `mixins/<id>/` may be.
CODE_SUFFIXES = {".rs", ".toml"}
CODE_NAMES = {".gitignore"}
CODE_PREFIXES = ("README", "LICENSE", "LICENCE", "COPYING", "NOTICE")

# Reader limits.
MAX_NATIVE_ENTRIES = 512
MAX_NATIVE_BYTES = 64 * 1024 * 1024
MAX_CONTENT_FILES = 65_536
MAX_CONTENT_FILE_BYTES = 16 * 1024 * 1024
MAX_CONTENT_BYTES = 512 * 1024 * 1024

CONTENT_PATH = re.compile(r"^(assets|data)/[a-z0-9_.-]+/[a-z0-9_.-]+")
REVISION = re.compile(r"^[0-9a-f]{40}$")


def code_allowed(rel: Path) -> bool:
    """Whether a file under the bundle may go into the packed file."""
    if "data" in rel.parts[:-1]:
        return True
    name = rel.name
    return (
        rel.suffix in CODE_SUFFIXES
        or name in CODE_NAMES
        or name.startswith(CODE_PREFIXES)
    )


def collect_content(content_dir: Path, problems: list[str]) -> dict[str, Path]:
    """The content entries: the manifest, then everything under assets/ and data/."""
    entries: dict[str, Path] = {}
    manifest = content_dir / CONTENT_MANIFEST
    if not manifest.is_file():
        problems.append(f"{manifest} does not exist")
        return entries
    entries[CONTENT_MANIFEST] = manifest

    for top in ("assets", "data"):
        root = content_dir / top
        if not root.is_dir():
            continue
        for path in sorted(p for p in root.rglob("*") if p.is_file()):
            rel = path.relative_to(content_dir).as_posix()
            if not CONTENT_PATH.match(rel):
                problems.append(f"{rel}: not a valid content path")
                continue
            entries[rel] = path
    return entries


def pinned_mixin_toml(bundle: Path, revision: str, problems: list[str]) -> str:
    """`horizon-mixin.toml` with `[compatibility] horizon_revision` set.

    A packed file must name the commit in both manifests: the reader refuses it
    with "native manifest and mixin identity/revision differ" when
    `horizon-native.json`'s `horizon` and `[compatibility] horizon_revision`
    disagree, and an absent revision counts as disagreeing.

    The bundle in this repository stays unpinned on purpose — built from the
    folder (`native-build <folder>`) it then compiles against whatever Horizon
    the instance plays, which is the point of a hook-point mod. So the pin is
    added to the packed copy only, which is what the launcher does to its own
    copy of a mod it builds.
    """
    text = (bundle / "horizon-mixin.toml").read_text(encoding="utf-8")
    existing = re.search(
        r"^\s*horizon_revision\s*=\s*\"([0-9a-f]{40})\"", text, re.MULTILINE
    )
    if existing:
        if existing.group(1) != revision:
            problems.append(
                f"horizon-mixin.toml pins {existing.group(1)} but "
                f"{NATIVE_MANIFEST} says {revision}"
            )
        return text
    if re.search(r"^\s*\[compatibility\]", text, re.MULTILINE):
        problems.append(
            "horizon-mixin.toml has a [compatibility] table without a "
            "horizon_revision; set it or remove the table"
        )
        return text
    return text + (
        "\n# Added when packing (tools/pack_mod.py): a published file must name the\n"
        "# commit in both manifests. The repository's copy stays unpinned.\n"
        "[compatibility]\n"
        f'horizon_revision = "{revision}"\n'
    )


def collect_code(bundle: Path, mod_id: str, problems: list[str]) -> dict[str, Path]:
    """The native entries, under `mixins/<id>/`."""
    entries: dict[str, Path] = {}
    manifest = bundle / "horizon-mixin.toml"
    if not manifest.is_file():
        problems.append(f"{manifest} does not exist")
        return entries

    skipped: list[str] = []
    for path in sorted(p for p in bundle.rglob("*") if p.is_file()):
        rel = path.relative_to(bundle)
        # Build output never belongs in the file.
        if "target" in rel.parts:
            continue
        if not code_allowed(rel):
            skipped.append(rel.as_posix())
            continue
        entries[f"mixins/{mod_id}/{rel.as_posix()}"] = path

    for name in skipped:
        print(f"  left out (not an allowed native entry): {name}")
    return entries


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--content", type=Path, default=repo / "bg2-content")
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument(
        "--revision",
        help="40 hex characters to use for `horizon` and `mixins` when packing, "
        "for testing while the real commit is unknown",
    )
    args = ap.parse_args()

    native_path = repo / NATIVE_MANIFEST
    if not native_path.is_file():
        print(f"error: {native_path} does not exist", file=sys.stderr)
        return 1
    native = json.loads(native_path.read_text(encoding="utf-8"))
    mod_id = native.get("id")
    if not isinstance(mod_id, str) or not mod_id:
        print(f"error: {NATIVE_MANIFEST} has no id", file=sys.stderr)
        return 1

    if args.revision:
        if not REVISION.match(args.revision):
            print(
                "error: --revision must be 40 lowercase hex characters",
                file=sys.stderr,
            )
            return 1
        native["horizon"] = native["mixins"] = args.revision
        print(f"packing with a stand-in revision {args.revision}: do not publish")
    for key in ("horizon", "mixins"):
        if not REVISION.match(str(native.get(key, ""))):
            print(
                f"error: {NATIVE_MANIFEST}'s {key} is {native.get(key)!r}, not a "
                f"40-character revision. The mod has not been built and tested on "
                f"a Horizon commit yet; pass --revision to pack anyway for a test.",
                file=sys.stderr,
            )
            return 1

    bundle = repo / "mixins" / mod_id
    if not bundle.is_dir():
        print(
            f"error: {bundle} does not exist. The bundle folder must be named "
            f"after the mod id, because the packed file uses mixins/<id>/.",
            file=sys.stderr,
        )
        return 1

    problems: list[str] = []
    content = collect_content(args.content, problems)
    code = collect_code(bundle, mod_id, problems)
    code[NATIVE_MANIFEST] = native_path  # its text is written, not copied
    mixin_toml = pinned_mixin_toml(bundle, native["horizon"], problems)
    mixin_entry = f"mixins/{mod_id}/horizon-mixin.toml"

    clash = set(content) & set(code)
    if clash:
        problems.append(f"the same path on both sides: {sorted(clash)}")

    native_bytes = sum(p.stat().st_size for n, p in code.items() if n != NATIVE_MANIFEST)
    if len(code) > MAX_NATIVE_ENTRIES:
        problems.append(f"{len(code)} native entries, at most {MAX_NATIVE_ENTRIES}")
    if native_bytes > MAX_NATIVE_BYTES:
        problems.append(f"native entries are {native_bytes} bytes unpacked, too many")
    content_bytes = 0
    for name, path in content.items():
        size = path.stat().st_size
        content_bytes += size
        if size > MAX_CONTENT_FILE_BYTES:
            problems.append(f"{name} is {size} bytes, over 16 MiB")
    if len(content) > MAX_CONTENT_FILES:
        problems.append(f"{len(content)} content files, too many")
    if content_bytes > MAX_CONTENT_BYTES:
        problems.append(f"content is {content_bytes} bytes, over 512 MiB")

    if problems:
        for p in problems:
            print(f"error: {p}", file=sys.stderr)
        return 1

    out = args.out or repo / "dist" / f"{mod_id}.hmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        for name in sorted(content):
            zf.write(content[name], name)
        for name in sorted(code):
            if name == NATIVE_MANIFEST:
                zf.writestr(name, json.dumps(native, indent=2, sort_keys=True) + "\n")
            elif name == mixin_entry:
                zf.writestr(name, mixin_toml)
            else:
                zf.write(code[name], name)

    print(
        f"packed {out} — {len(content)} content file(s), {len(code)} native entry(ies), "
        f"{out.stat().st_size} bytes"
    )
    print(f"check it with: cargo horizon-mod check {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
