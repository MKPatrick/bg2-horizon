"""Builds Building Gadgets 2's blocks and items as a Horizon content mod (API 0.2).

NOT IMPLEMENTED YET. This file is the spec for M1's conversion step; it lists
what the tool has to do and exits. See TODO.md, "M1 - It loads".

Horizon runs no Java, so a block here is a vanilla block's behaviour (its
`template`) under Building Gadgets 2's id, carrying the Java mod's models,
textures, names, recipes and tags. Nearly all of the mod's behaviour is in the
trait mod (`native/buildinggadgets2`), so the content side is small: 8 items, 2 blocks.

Source: Building Gadgets 2's jar, or its `src/main/resources` plus
`src/generated/resources` (240 asset files, 37 generated resources).

What the tool must do, going from Minecraft 26.1.2 to Horizon's 26.3:

- copy `assets/buildinggadgets2/` as it is where the formats agree: the item
  models (flat `item/generated`), the textures, the lang files, the sounds;
- write an `assets/buildinggadgets2/items/<id>.json` item definition for each of
  the 8 items, as 26.3 wants them;
- rename the data folders 26.3 renamed (`recipes` -> `recipe`,
  `loot_tables` -> `loot_table`, `tags/items` -> `tags/item`, ...);
- put the recipes in 26.3's form (ingredients as ids and `#tags`, results as
  `{"id", "count"}`), and leave out what does not apply;
- leave the `patchouli_books/` folder out: Patchouli does not exist on Horizon
  (PLAN.md, "Open questions");
- leave out the assets the Java mod no longer registers (the construction
  block, paste and paste container models are in `src/main/resources` but not
  in `Registration.java`);
- keep `data/buildinggadgets2/horizon/{item,block,creative_tab,menu}/` as this
  repository already has them by hand - they are the declarations, not
  conversions, and the tool should check them against what it writes rather
  than overwrite them.

The `.hmod` itself is packed by Horizon's own tool, not here:

    cargo horizon-mod build        # -> target/horizon-mod/buildinggadgets2.hmod
    cargo horizon-mod check target/horizon-mod/buildinggadgets2.hmod

The packed file then goes to `native/buildinggadgets2/data/bg2-content.hmod`, which the
native bundle loads at start, so one `.hnmod` holds everything.

Usage, once written: python make_content_mod.py <jar-or-resources-dir> <out dir>
Test with the game argument --dev-mod "<out dir>" (launcher Settings > Game
arguments).
"""

import sys


def main() -> int:
    sys.stderr.write(
        "make_content_mod.py is not implemented yet: it is the spec for M1's\n"
        "conversion step. Read this file's docstring and TODO.md.\n"
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
