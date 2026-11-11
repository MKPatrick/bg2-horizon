#!/usr/bin/env python3
"""Builds bg2-content from Building Gadgets 2's own resources.

Horizon runs no Java, so the mod's behaviour is the trait mod in
`mixins/buildinggadgets2`. What it needs beside that is the look: the models,
textures, names, recipes and loot the Java mod already ships. Those are carried
over here.

Almost nothing has to be converted. Building Gadgets 2 targets Minecraft
26.1.2 and its data generator already writes the formats 26.3 reads: singular
`data/<ns>/recipe` and `loot_table` folders, `assets/<ns>/items/<id>.json`
item definitions, recipe results as `{"id": ...}`. So this is mostly a
copy of the files that are actually used, plus three things the Java mod gets
from its loader and Horizon does not:

- **`c:` item tags.** The recipes use `#c:ingots/iron` and five others. Those
  are NeoForge's convention, not vanilla's, and Horizon ships none of them
  (AE2's port ships its own too). Without them every recipe silently matches
  nothing, so they are written here from the vanilla items they stand for.
- **`sounds.json`**, which maps the sound event `beep` to `beep.ogg`.
- **Leaving the unused assets out.** `src/main/resources` still carries
  Building Gadgets 1's construction blocks, paste and paste containers, which
  `Registration.java` does not register, and a Patchouli book Horizon has no
  reader for.

The textures to copy are not hardcoded: the item and block models are parsed
and whatever they reference is taken, so a model that gains a layer does not
need this script changed.

What it does **not** touch is `data/buildinggadgets2/horizon/`, the block, item,
creative tab and menu declarations. Those are Horizon's own format, written by
hand, and are the one part with no Java counterpart. The script checks them
against the items it found and reports a mismatch instead of overwriting them.

Usage:
    python tools/make_content_mod.py [--java PATH] [--out PATH] [--check]

--java defaults to ../BuildingGadgets2 next to this repository (its `src/`
tree; a built jar is not read). --check reports what would change and writes
nothing. Then:

    cd bg2-content && cargo horizon-mod build
    cargo horizon-mod check target/horizon-mod/buildinggadgets2.hmod
"""

from __future__ import annotations

import argparse
import filecmp
import json
import shutil
import sys
from pathlib import Path

NS = "buildinggadgets2"

# The `c:` item tags the recipes use, and the vanilla items they stand for.
# Horizon ships no `c:` tags, so a recipe naming one matches nothing until it
# is defined. Keeping the recipes as the Java mod wrote them and supplying the
# tags is closer to the original than rewriting six recipes.
COMMON_ITEM_TAGS: dict[str, list[str]] = {
    "gems/diamond": ["minecraft:diamond"],
    "gems/emerald": ["minecraft:emerald"],
    "gems/lapis": ["minecraft:lapis_lazuli"],
    "ingots/iron": ["minecraft:iron_ingot"],
    "dusts/redstone": ["minecraft:redstone"],
    "ender_pearls": ["minecraft:ender_pearl"],
}

# The sound events the mod plays, and the files behind them.
SOUNDS: dict[str, list[str]] = {"beep": [f"{NS}:beep"]}

# Copied as they are from the data generator's output: already in 26.3's form.
FROM_GENERATED = [
    f"assets/{NS}/items",
    f"assets/{NS}/models/item",
    f"assets/{NS}/models/block",
    f"assets/{NS}/blockstates",
    f"assets/{NS}/lang",
    f"data/{NS}/recipe",
    f"data/{NS}/loot_table",
    f"data/{NS}/advancement",
    f"data/{NS}/tags",
    "data/c/tags",
    "data/minecraft/tags",
]

# Copied from the hand-written resources. The textures are worked out from the
# models instead of being listed.
FROM_MAIN = [
    f"assets/{NS}/lang",
    f"assets/{NS}/sounds",
    f"assets/{NS}/textures/gui",
]

# Written by hand in Horizon's own format, never generated from the Java mod.
KEEP = f"data/{NS}/horizon"


class Result:
    def __init__(self) -> None:
        self.written: list[str] = []
        self.unchanged = 0
        self.problems: list[str] = []


def copy_tree(src: Path, dst: Path, res: Result, check: bool) -> None:
    """Copy every file under `src` to the same relative place under `dst`."""
    if not src.is_dir():
        return
    for path in sorted(p for p in src.rglob("*") if p.is_file()):
        target = dst / path.relative_to(src)
        copy_file(path, target, res, check)


def copy_file(src: Path, dst: Path, res: Result, check: bool) -> None:
    if dst.is_file() and filecmp.cmp(src, dst, shallow=False):
        res.unchanged += 1
        return
    res.written.append(str(dst))
    if not check:
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)


def write_json(dst: Path, data: object, res: Result, check: bool) -> None:
    text = json.dumps(data, indent=2) + "\n"
    if dst.is_file() and dst.read_text(encoding="utf-8") == text:
        res.unchanged += 1
        return
    res.written.append(str(dst))
    if not check:
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_text(text, encoding="utf-8")


def referenced_textures(generated: Path) -> set[str]:
    """`namespace:path` of every texture the mod's models name.

    Read from the Java mod's own output, not from `bg2-content`, so `--check`
    sees the same thing a real run would.
    """
    found: set[str] = set()
    for models in (f"assets/{NS}/models", f"assets/{NS}/items"):
        root = generated / models
        if not root.is_dir():
            continue
        for path in root.rglob("*.json"):
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                raise SystemExit(f"{path}: {e}") from e
            found |= texture_values(data)
    return found


def texture_values(node: object) -> set[str]:
    """Every value under a `textures` map, however deeply nested."""
    found: set[str] = set()
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "textures" and isinstance(value, dict):
                found |= {v for v in value.values() if isinstance(v, str)}
            else:
                found |= texture_values(value)
    elif isinstance(node, list):
        for item in node:
            found |= texture_values(item)
    return found


def copy_textures(java: Path, generated: Path, out: Path, res: Result, check: bool) -> None:
    """Copy the textures the models name, and report any that are missing."""
    for ref in sorted(referenced_textures(generated)):
        namespace, _, path = ref.partition(":")
        if not path:
            namespace, path = "minecraft", namespace
        if namespace != NS:
            continue  # vanilla's own textures come from the game
        src = java / "src/main/resources/assets" / namespace / "textures" / f"{path}.png"
        if not src.is_file():
            res.problems.append(f"{ref}: {src} does not exist")
            continue
        copy_file(src, out / "assets" / namespace / "textures" / f"{path}.png", res, check)


def check_declarations(generated: Path, out: Path, res: Result) -> None:
    """The hand-written Horizon declarations against the Java mod's own items.

    Read from the Java mod's output, so `--check` reports the same as a run.
    """
    declared_items = {
        p.stem for p in (out / KEEP / "item").glob("*.json")
    }
    declared_blocks = {
        p.stem for p in (out / KEEP / "block").glob("*.json")
    }
    # Every item definition the Java mod generated is an item or a block's item.
    defined = {p.stem for p in (generated / f"assets/{NS}/items").glob("*.json")}
    block_items = {
        name
        for name in declared_blocks
        if json.loads((out / KEEP / "block" / f"{name}.json").read_text("utf-8")).get(
            "item", True
        )
    }

    missing = defined - declared_items - block_items
    if missing:
        res.problems.append(
            f"{KEEP}: no declaration for {sorted(missing)}, which "
            f"assets/{NS}/items defines"
        )
    extra = declared_items - defined
    if extra:
        res.problems.append(
            f"{KEEP}: {sorted(extra)} declared but assets/{NS}/items has no "
            f"model definition for them"
        )
    for name in sorted(declared_blocks):
        state = generated / f"assets/{NS}/blockstates/{name}.json"
        if not state.is_file():
            res.problems.append(f"{KEEP}/block/{name}.json: no blockstate file")


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--java", type=Path, default=repo.parent / "BuildingGadgets2")
    ap.add_argument("--out", type=Path, default=repo / "bg2-content")
    ap.add_argument(
        "--check", action="store_true", help="report what would change, write nothing"
    )
    args = ap.parse_args()

    generated = args.java / "src/generated/resources"
    main_res = args.java / "src/main/resources"
    for needed in (generated, main_res):
        if not needed.is_dir():
            print(
                f"error: {needed} does not exist: is --java Building Gadgets 2's "
                f"source tree?",
                file=sys.stderr,
            )
            return 1

    out = args.out
    res = Result()

    for rel in FROM_GENERATED:
        copy_tree(generated / rel, out / rel, res, args.check)
    for rel in FROM_MAIN:
        copy_tree(main_res / rel, out / rel, res, args.check)
    copy_textures(args.java, generated, out, res, args.check)

    # The `c:` tags the recipes need and Horizon does not ship.
    for name, values in COMMON_ITEM_TAGS.items():
        write_json(
            out / f"data/c/tags/item/{name}.json", {"values": values}, res, args.check
        )
    write_json(
        out / f"assets/{NS}/sounds.json",
        {event: {"sounds": files} for event, files in SOUNDS.items()},
        res,
        args.check,
    )

    check_declarations(generated, out, res)

    verb = "would write" if args.check else "wrote"
    print(f"make_content_mod: {verb} {len(res.written)} file(s), {res.unchanged} unchanged")
    for path in res.written[:40]:
        print(f"  {Path(path).relative_to(repo) if repo in Path(path).parents else path}")
    if len(res.written) > 40:
        print(f"  ... and {len(res.written) - 40} more")
    for problem in res.problems:
        print(f"problem: {problem}")
    return 1 if res.problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
