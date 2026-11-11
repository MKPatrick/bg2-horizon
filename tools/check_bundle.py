#!/usr/bin/env python3
"""Checks this bundle against a Horizon source tree, before a build is attempted.

A native build takes long enough that catching a bad manifest first is worth a
script. Everything here is read from the Horizon tree, nothing is guessed.

For the trait mod (`[api]`):

- the package exists, exports `hooks()`, and depends on `horizon-client`;
- every `Hooks` method it implements is a real hook, by name, in that tree's
  catalog (the generated trait takes its method names from it unchanged);
- `horizon-native.json`'s `hooks` is at least the API number of every hook the
  mod implements (HOOK_API below, from docs/native-mods.md), and no higher
  than it needs: a higher number asks for a newer Horizon for nothing.

For injections and modules, if the bundle ever grows them (it does not need
them today):

- every `hook` is in native-hooks.json with the `hook_version` the catalog
  gives, and every `helper`/`source` file exists;
- every `[[module]]`'s `file` exists in the Horizon tree and its `name` is
  prefixed so it cannot clash (E_MODULE, E_MODULE_CONFLICT).

And for the registry manifest, the checks Horizon itself applies
(`horizon_mods::index::native::Manifest::check`): schema 1, a valid id, a
non-empty version and game, `horizon` and `mixins` each exactly 40 lowercase
hex characters, `hooks` not 0, and no unknown fields.

Usage:
    python tools/check_bundle.py [--horizon PATH] [--bundle PATH]

--horizon defaults to ../horizon-main next to this repository. Exit status is
0 when there is no error, 1 otherwise.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

# The `"hooks": N` each hook this port uses needs, from docs/native-mods.md.
# A hook implemented by the mod but missing here is reported, not assumed fine.
HOOK_API: dict[str, int] = {
    # Per-stack state
    "component_types": 65,
    # Acting on the world
    "use_item_on": 13,
    "use_item": 13,
    "destroy_block": 14,
    "place_block": 14,
    "left_click_block": 13,
    "right_click_block": 21,
    "tooltip": 12,
    # Block entities and their menus
    "block_entity": 15,
    "block_entity_ticks": 15,
    "block_entity_tick": 15,
    "block_entity_container": 16,
    "menu_button": 20,
    # Commands
    "register_commands": 8,
    "perform_command": 8,
    # Networking
    "net_from_client": 40,
    "net_from_server": 40,
    # Screens and input
    "screen": 19,
    "screen_draw": 19,
    "screen_click": 20,
    "screen_tooltip": 20,
    "screen_render_post": 111,
    "screen_mouse_button_pressed_pre": 111,
    "screen_mouse_button_released_pre": 111,
    "screen_mouse_scrolled_pre": 111,
    "screen_key_pressed_pre": 111,
    "screen_char_typed_pre": 111,
    "key": 111,
    "input_key": 111,
    # Drawing
    "world_drawn": 58,
    "render_needs": 58,
    "render": 58,
    "world_primitives": 58,
}

# A helper the mod calls sets the same floor as a hook it implements: the
# launcher builds on any Horizon at `hooks` or later, and a helper that is not
# there yet does not compile. These are the numbers from docs/native-mods.md
# for the helpers this port uses. A helper whose API number is still an
# unstamped `@API:...@` placeholder (`sneaking`, `block_name_in_tag`,
# `may_build`, `component`, all under `@API:host_bridge@`) cannot be named, so
# it is listed with the catalog's own API: the only version known to have it is
# the one the mod was compiled against.
HELPER_API: dict[str, int] = {
    "acting_player": 8,
    "count_items": 9,
    "take_items": 9,
    "give_items": 9,
    "set_component": 65,
    "edit_blocks": 126,
    "block_collision": 363,
    "entity": 379,
    "living": 379,
    # "Entity and level handles" (#7387) landed together.
    "level": 379,
    "level_block": 379,
    "level_block_state": 379,
    "level_height": 379,
    "level_is_loaded": 379,
}

# Helpers with no stamped API number: they pin the mod to the catalog's own
# version, because no earlier number is known to include them.
UNSTAMPED_HELPERS = frozenset(
    {
        "sneaking",
        "block_name_in_tag",
        "may_build",
        "component",
        "living_equipment",
        "living_set_equipment",
        "action_bar",
    }
)

# horizon-mixins' injection kinds (its README).
KINDS = {"HEAD", "RETURN", "BEFORE_CALL", "AFTER_CALL", "MODIFY_ARG", "OVERWRITE"}

# horizon-native.json's fields, from horizon_mods::index::native::Manifest,
# which is serde(deny_unknown_fields): anything else makes it unreadable.
MANIFEST_FIELDS = {"schema", "id", "version", "game", "horizon", "mixins", "hooks"}


def revision(value: object) -> bool:
    """horizon_mods::index::native::revision: 40 lowercase hex characters."""
    return (
        isinstance(value, str)
        and len(value) == 40
        and all(c.isdigit() or c in "abcdef" for c in value)
    )


def valid_id(value: object) -> bool:
    """horizon_mods::index::valid_id: 1-64 of [a-z0-9_]."""
    return (
        isinstance(value, str)
        and 1 <= len(value) <= 64
        and all(c.islower() or c.isdigit() or c == "_" for c in value)
    )


class Report:
    def __init__(self) -> None:
        self.errors: list[str] = []
        self.warnings: list[str] = []

    def error(self, msg: str) -> None:
        self.errors.append(msg)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)

    def finish(self) -> int:
        for w in self.warnings:
            print(f"warning: {w}")
        for e in self.errors:
            print(f"error: {e}")
        if not self.errors and not self.warnings:
            print("check_bundle: nothing to report")
        else:
            print(
                f"check_bundle: {len(self.errors)} error(s), "
                f"{len(self.warnings)} warning(s)"
            )
        return 1 if self.errors else 0


def api_of(horizon: Path) -> int | None:
    """This Horizon tree's NATIVE_HOOKS_API."""
    path = horizon / "crates/horizon-client/src/game/native_hooks.rs"
    if not path.is_file():
        return None
    m = re.search(
        r"NATIVE_HOOKS_API:\s*u32\s*=\s*(\d+)",
        path.read_text(encoding="utf-8", errors="replace"),
    )
    return int(m.group(1)) if m else None


def known_hooks(horizon: Path, rep: Report) -> tuple[set[str], str]:
    """Hook names a trait mod may implement, and where the list came from.

    The `Hooks` trait is generated from `native-hooks.json` at build time and
    its methods carry the catalog's names unchanged (checked by running
    Horizon's own generator against this tree: 533 methods, every name in the
    catalog). So the catalog is the list, plus `render` — the trait's accessor
    for `RenderHooks` — and the render hooks themselves, whose generated trait
    is committed in the renderer.
    """
    catalog = horizon / "native-hooks.json"
    if not catalog.is_file():
        rep.error(f"{catalog} not found: is --horizon a Horizon source tree?")
        return set(), "nothing"
    data = json.loads(catalog.read_text(encoding="utf-8"))
    names = {h["name"] for h in data.get("hooks", [])}
    names.add("render")

    render = horizon / "crates/horizon-client-render/src/gfx/native_render/generated.rs"
    if render.is_file():
        text = render.read_text(encoding="utf-8", errors="replace")
        names |= set(re.findall(r"\bfn ([a-z_0-9]+)\s*\(&self", text))
    return names, "native-hooks.json"


def catalog_versions(horizon: Path) -> dict[str, int | None]:
    path = horizon / "native-hooks.json"
    if not path.is_file():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    return {h["name"]: h.get("version") for h in data.get("hooks", [])}


def sources(api_dir: Path) -> list[Path]:
    """The mod's own .rs files, without the build output under target/."""
    return sorted(
        p for p in api_dir.rglob("*.rs") if "target" not in p.relative_to(api_dir).parts
    )


def called_helpers(api_dir: Path, catalog_helpers: set[str]) -> set[str]:
    """Helper names the mod calls: `host.name(` or `.name(host`."""
    found: set[str] = set()
    for rs in sources(api_dir):
        text = rs.read_text(encoding="utf-8", errors="replace")
        for m in re.finditer(r"\bhost\.([a-z_0-9]+)\s*\(", text):
            if m.group(1) in catalog_helpers:
                found.add(m.group(1))
        for m in re.finditer(r"\.([a-z_0-9]+)\s*\(\s*host\b", text):
            if m.group(1) in catalog_helpers:
                found.add(m.group(1))
    return found


def catalog_helpers(horizon: Path) -> set[str]:
    path = horizon / "native-hooks.json"
    if not path.is_file():
        return set()
    data = json.loads(path.read_text(encoding="utf-8"))
    return {h["name"] for h in data.get("helpers", [])}


def implemented_hooks(api_dir: Path) -> dict[str, Path]:
    """`fn <name>(&self, ...)` in `impl Hooks for ...` blocks, by name."""
    found: dict[str, Path] = {}
    for rs in sources(api_dir):
        text = rs.read_text(encoding="utf-8", errors="replace")
        for block in re.finditer(r"impl\s+Hooks\s+for\s+\w+\s*\{", text):
            # Walk braces from the impl's opening brace to its match.
            i = block.end() - 1
            depth, end = 0, len(text)
            for j in range(i, len(text)):
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                    if depth == 0:
                        end = j
                        break
            for m in re.finditer(r"\bfn\s+([a-z_0-9]+)\s*\(\s*&self", text[i:end]):
                found[m.group(1)] = rs
    return found


def check_api_package(bundle: Path, spec: dict, rep: Report) -> Path | None:
    api = spec.get("api")
    if not api:
        return None
    path = api.get("path")
    if not path:
        rep.error("[api] has no `path`")
        return None
    api_dir = bundle / path
    manifest = api_dir / "Cargo.toml"
    if not manifest.is_file():
        rep.error(f"[api] {path!r}: {manifest} does not exist")
        return None

    cargo = tomllib.loads(manifest.read_text(encoding="utf-8"))
    deps = cargo.get("dependencies", {})
    if "horizon-client" not in deps:
        rep.error(
            f"[api] {path!r}: Cargo.toml does not depend on horizon-client, "
            f"so it cannot implement Hooks"
        )
    lib = api_dir / "src/lib.rs"
    if not lib.is_file():
        rep.error(f"[api] {path!r}: src/lib.rs does not exist")
    else:
        text = lib.read_text(encoding="utf-8", errors="replace")
        if not re.search(r"pub\s+fn\s+hooks\s*\(\s*\)", text):
            rep.error(
                f"[api] {path!r}: src/lib.rs does not export "
                f"`pub fn hooks() -> Box<dyn Hooks>`, which the launcher calls"
            )
    return api_dir


def check(repo: Path, bundle: Path, horizon: Path, rep: Report) -> None:
    toml_path = bundle / "horizon-mixin.toml"
    if not toml_path.is_file():
        rep.error(f"{toml_path} not found")
        return
    spec = tomllib.loads(toml_path.read_text(encoding="utf-8"))

    mod = spec.get("mod", {})
    mod_id = mod.get("id")
    if not valid_id(mod_id):
        rep.error(f"[mod] id {mod_id!r} is not 1-64 characters of [a-z0-9_]")
    if not mod.get("version"):
        rep.error("[mod] has no version")

    hooks_known, source = known_hooks(horizon, rep)
    versions = catalog_versions(horizon)
    apis_used: dict[str, int] = {}
    referenced: set[str] = set()

    # --- the trait mod ------------------------------------------------------
    api_dir = check_api_package(bundle, spec, rep)
    if api_dir is not None:
        impls = implemented_hooks(api_dir)
        if not impls:
            rep.warn(
                "[api]: no `impl Hooks` method found; the mod does nothing yet"
            )
        for name, where in sorted(impls.items()):
            rel = where.relative_to(repo)
            if hooks_known and name not in hooks_known:
                rep.error(
                    f"{rel}: `{name}` is not a hook in this Horizon "
                    f"(checked against {source})"
                )
            elif name in HOOK_API:
                apis_used[name] = HOOK_API[name]
            elif name != "render":
                rep.warn(
                    f"{rel}: hook {name!r} has no API number in HOOK_API; "
                    f"look it up in docs/native-mods.md and add it"
                )

        # The helpers the mod calls set the same floor as the hooks.
        helpers = called_helpers(api_dir, catalog_helpers(horizon))
        api = api_of(horizon)
        for helper in sorted(helpers):
            if helper in HELPER_API:
                apis_used[f"helper {helper}"] = HELPER_API[helper]
            elif helper in UNSTAMPED_HELPERS:
                if api is not None:
                    apis_used[f"helper {helper} (unstamped)"] = api
            else:
                rep.warn(
                    f"helper {helper!r} is called but has no API number in "
                    f"HELPER_API; look it up in docs/native-mods.md and add it, "
                    f"or list it in UNSTAMPED_HELPERS"
                )

    # --- injections, if any -------------------------------------------------
    for inj in spec.get("injection", []):
        who = f"injection {inj.get('id', '<no id>')!r}"
        if inj.get("kind") not in KINDS:
            rep.error(f"{who}: kind {inj.get('kind')!r} is not one of {sorted(KINDS)}")
        hook, target = inj.get("hook"), inj.get("target")
        if not hook and not target:
            rep.error(f"{who}: neither `hook` nor `target`")
        if hook:
            if versions and hook not in versions:
                rep.error(f"{who}: hook {hook!r} is not in native-hooks.json")
            else:
                want, have = inj.get("hook_version"), versions.get(hook)
                if want is None:
                    rep.error(f"{who}: no hook_version (the catalog says {have})")
                elif have is not None and want != have:
                    rep.error(
                        f"{who}: hook_version {want} but {hook!r} is version "
                        f"{have} here (E_HOOK_VERSION)"
                    )
                apis_used[hook] = HOOK_API.get(hook, apis_used.get(hook, 0))
        if target:
            rep.warn(
                f"{who}: a raw `target` is not matched against the catalog, "
                f"so the mod must pin a Horizon commit; prefer `hook`"
            )
        helper = inj.get("helper")
        if helper:
            referenced.add(helper)
            if not (bundle / helper).is_file():
                rep.error(f"{who}: helper {helper!r} does not exist")

    # --- modules, if any ----------------------------------------------------
    for module in spec.get("module", []):
        who = f"module {module.get('id', '<no id>')!r}"
        src = module.get("source")
        if not src:
            rep.error(f"{who}: no `source`")
        else:
            referenced.add(src)
            if not (bundle / src).is_file():
                rep.error(f"{who}: source {src!r} does not exist")
        file = module.get("file")
        if not file:
            rep.error(f"{who}: no `file`")
        elif not (horizon / file).is_file():
            rep.error(f"{who}: file {file!r} is not in this Horizon (E_MODULE)")
        name = module.get("name")
        if not name:
            rep.error(f"{who}: no `name`")
        elif not any(
            name.startswith(p + short)
            for p in ("port_", "hz_")
            for short in {mod_id or "", bundle.name}
            if short
        ):
            rep.warn(
                f"{who}: name {name!r} is not `port_`/`hz_` plus the mod id or "
                f"the bundle folder name; risks E_MODULE_CONFLICT"
            )

    # --- crates, if any -----------------------------------------------------
    for crate in spec.get("crate", []):
        who = f"crate {crate.get('id', '<no id>')!r}"
        path = crate.get("path")
        if not path:
            rep.error(f"{who}: no `path`")
        elif not (bundle / path / "Cargo.toml").is_file():
            rep.error(f"{who}: {path}/Cargo.toml does not exist")

    for rs in sorted(bundle.glob("*.rs")):
        if rs.name not in referenced:
            rep.warn(
                f"{rs.name} is in the bundle root but nothing references it: "
                f"it is not compiled in"
            )

    check_manifest(repo, mod, mod_id, apis_used, horizon, rep)


def check_manifest(
    repo: Path,
    mod: dict,
    mod_id: str | None,
    apis_used: dict[str, int],
    horizon: Path,
    rep: Report,
) -> None:
    path = repo / "horizon-native.json"
    if not path.is_file():
        rep.error(f"{path} not found")
        return
    native = json.loads(path.read_text(encoding="utf-8"))

    for key in sorted(set(native) - MANIFEST_FIELDS):
        rep.error(
            f"horizon-native.json: unknown field {key!r} (the manifest is "
            f"deny_unknown_fields, so this makes it unreadable)"
        )
    if native.get("schema") != 1:
        rep.error(f"horizon-native.json: schema must be 1, not {native.get('schema')!r}")
    if not valid_id(native.get("id")):
        rep.error(f"horizon-native.json: id {native.get('id')!r} is not valid")
    elif mod_id and native["id"] != mod_id:
        rep.error(
            f"horizon-native.json id {native['id']!r} != [mod] id {mod_id!r}"
        )
    if not native.get("version"):
        rep.error("horizon-native.json: version is empty")
    elif mod.get("version") and native["version"] != mod["version"]:
        rep.error(
            f"horizon-native.json version {native['version']!r} != "
            f"[mod] version {mod['version']!r}"
        )
    if not native.get("game"):
        rep.error("horizon-native.json: game is empty")

    for key in ("horizon", "mixins"):
        value = native.get(key)
        if revision(value):
            continue
        # A placeholder is the honest state while nothing has been built and
        # tested yet: the launcher builds the bundle from horizon-mixin.toml,
        # and this file is only needed to publish to Horizon's registry. A
        # value that is neither a revision nor a placeholder is a mistake.
        if isinstance(value, str) and value.startswith("TODO"):
            rep.warn(
                f"horizon-native.json: {key} is still {value!r}; the manifest "
                f"cannot be published to the registry until it is a real "
                f"40-character revision"
            )
        else:
            rep.error(
                f"horizon-native.json: {key} must be 40 lowercase hex "
                f"characters, not {value!r}"
            )

    declared, needed = native.get("hooks"), max(apis_used.values(), default=0)
    if "hooks" not in native:
        rep.warn(
            "horizon-native.json has no `hooks`: the mod would be pinned to "
            "its `horizon` commit instead of building on any newer Horizon"
        )
    elif declared == 0:
        rep.error("horizon-native.json: hooks must not be 0")
    elif needed and declared < needed:
        worst = sorted(h for h, a in apis_used.items() if a == needed)
        rep.error(f"horizon-native.json: hooks is {declared} but {worst} need {needed}")
    elif needed and declared > needed:
        rep.warn(
            f"horizon-native.json: hooks is {declared} but the mod only needs "
            f"{needed}; a higher number asks for a newer Horizon for nothing"
        )

    api = api_of(horizon)
    if api is not None and isinstance(declared, int) and declared > api:
        rep.error(
            f"horizon-native.json: hooks is {declared} but this Horizon's "
            f"NATIVE_HOOKS_API is {api}"
        )


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--horizon", type=Path, default=repo.parent / "horizon-main")
    ap.add_argument("--bundle", type=Path, default=repo / "mixins/buildinggadgets2")
    args = ap.parse_args()

    rep = Report()
    api = api_of(args.horizon)
    print(
        f"check_bundle: bundle {args.bundle.name}, Horizon {args.horizon} "
        f"(NATIVE_HOOKS_API {api if api is not None else 'unknown'})"
    )
    check(repo, args.bundle, args.horizon, rep)
    return rep.finish()


if __name__ == "__main__":
    raise SystemExit(main())
