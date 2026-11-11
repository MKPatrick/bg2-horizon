//! The building modes: which blocks a click fills.
//!
//! This is the part of the port that has to match the Java mod exactly — a
//! mode that collects a slightly different set makes every build subtly
//! wrong — and it is also the part that needs no running game, so it is
//! written against [`Blocks`] and checked by the tests below, each read off
//! the Java source case by case.
//!
//! The Java side is `util/modes/*.java`. A mode's `collectWorld` returns
//! positions **relative to the hit block**, not absolute ones
//! (`BlockPos.ZERO.relative(hitSide, i)`), while it checks validity on the
//! absolute position (`start.relative(hitSide, i)`). `BuildingUtils.build`
//! then offsets each by the hit position again. This port keeps that split:
//! [`collect`] answers relative offsets.
//!
//! Milestone 2 has Build to Me only; the other ten follow in M3.

use crate::world::{Blocks, Face};

/// The settings a mode reads off the gadget (`GadgetNBT`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settings {
    /// `PLACE_ON_TOP`: Build to Me measures to the player's eye instead of
    /// their feet.
    pub place_on_top: bool,
    /// `RAYTRACE_FLUID`, used by the raycast rather than by a mode.
    pub raytrace_fluid: bool,
    /// `gadget_range`, the tool range the area modes use (M3).
    pub range: i32,
}

/// Where the player is, as a mode reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerAt {
    /// `player.blockPosition()`, the block the player stands in.
    pub block: [i32; 3],
    /// `player.getEyeY()`, the exact height of their eyes.
    pub eye_y: f64,
}

/// The modes a gadget can be in. Only `BuildToMe` is ported so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    BuildToMe,
}

impl Mode {
    /// The mode's id, as `BaseMode.getId()` gives it.
    pub fn id(self) -> &'static str {
        match self {
            Mode::BuildToMe => "buildinggadgets2:build_to_me",
        }
    }

    /// The mode of an id, or `None` for one this port does not have yet.
    pub fn from_id(id: &str) -> Option<Mode> {
        match id {
            "buildinggadgets2:build_to_me" | "build_to_me" => Some(Mode::BuildToMe),
            _ => None,
        }
    }

    /// The translation key, as `BaseMode.i18n()`.
    pub fn i18n(self) -> String {
        let path = self.id().split_once(':').map_or(self.id(), |(_, p)| p);
        format!("buildinggadgets2.modes.{path}")
    }
}

/// `BaseMode.collect`: the offsets from the hit block a build would fill.
///
/// `start` is the hit block and `hit_side` the face it was hit on, as
/// `BaseGadget.getHitPos` and the hit result give them. The result is
/// relative to `start`.
///
/// The anchor branch of `BaseMode.collect` (an anchor list set by the anchor
/// key) is M3's; with no anchor it goes straight to `collectWorld`.
pub fn collect(
    mode: Mode,
    blocks: &dyn Blocks,
    hit_side: Face,
    start: [i32; 3],
    player: PlayerAt,
    settings: Settings,
) -> Vec<[i32; 3]> {
    match mode {
        Mode::BuildToMe => build_to_me(blocks, hit_side, start, player, settings),
    }
}

/// `BuildToMe.collectWorld`.
///
/// ```text
/// int startCoord  = hitSide.getAxis().choose(start.getX(), start.getY(), start.getZ());
/// double yPos     = placeontop ? player.getEyeY() : player.getBlockY();
/// int playerCoord = hitSide.getAxis().choose(px, (int) yPos, pz);
/// double difference = Math.max(0, Math.min(32, Math.abs(startCoord - playerCoord)));
/// for (int i = 1; i < difference; i++) { ... }
/// ```
///
/// Two details of that are easy to lose and are kept here:
///
/// - the loop is `i < difference`, so the block at the player's own coordinate
///   is **not** placed — a run of `difference - 1` blocks;
/// - `(int) yPos` truncates towards zero, which is not `getBlockY()`'s floor.
///   Below y 0 the two differ, so the eye case truncates and the feet case
///   floors, as in Java.
fn build_to_me(
    blocks: &dyn Blocks,
    hit_side: Face,
    start: [i32; 3],
    player: PlayerAt,
    settings: Settings,
) -> Vec<[i32; 3]> {
    let axis = hit_side.axis();
    let start_coord = axis.choose(start[0], start[1], start[2]);
    let player_y = if settings.place_on_top {
        // Java's `(int) double`: towards zero.
        player.eye_y as i32
    } else {
        player.block[1]
    };
    let player_coord = axis.choose(player.block[0], player_y, player.block[2]);

    let difference = (start_coord - player_coord).abs().clamp(0, 32);
    let mut out = Vec::new();
    for i in 1..difference {
        if is_pos_valid(blocks, hit_side.relative(start, i)) {
            out.push(hit_side.relative([0, 0, 0], i));
        }
    }
    out
}

/// `BaseMode.isPosValid` for a mode that is not exchanging.
///
/// The Java checks, in order:
///
/// 1. `isExchangingValid` — exchanging only, so not here;
/// 2. `y >= level.getMaxY() || y < level.getMinY()`;
/// 3. `blockState.canSurvive(level, pos)` — **not ported yet**: Horizon has no
///    helper that asks a state whether it can stay at a position, so a build
///    of blocks that need support (a torch, a sapling) is not filtered here
///    and those blocks drop when placed. It matters for nothing a full block
///    does, which is what M2 places;
/// 4. `level.mayInteract(player, pos)` — `Host::may_build`;
/// 5. the block there `canBeReplaced()`.
pub fn is_pos_valid(blocks: &dyn Blocks, pos: [i32; 3]) -> bool {
    let (min_y, height) = blocks.height();
    if pos[1] >= min_y + height || pos[1] < min_y {
        return false;
    }
    if !blocks.may_place(pos) {
        return false;
    }
    blocks.is_replaceable(pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Fake;

    fn player_at(x: i32, y: i32, z: i32) -> PlayerAt {
        PlayerAt {
            block: [x, y, z],
            eye_y: f64::from(y) + 1.62,
        }
    }

    #[test]
    fn builds_a_line_towards_the_player_and_stops_one_short() {
        // Standing at x 0, looking at a block's west face at x 10. The run
        // goes 1..difference, difference = |10 - 0| = 10, so offsets 1..9 —
        // nine blocks, ending one short of the player's own column.
        let w = Fake::new().bounds(0, 16).with([10, 0, 0], "minecraft:stone");
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [10, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        assert_eq!(out.len(), 9);
        assert_eq!(out.first(), Some(&[-1, 0, 0]));
        assert_eq!(out.last(), Some(&[-9, 0, 0]));
    }

    #[test]
    fn the_range_is_clamped_to_32() {
        let w = Fake::new().bounds(0, 16);
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [100, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        // min(32, 100) = 32, loop 1..32 → 31 blocks.
        assert_eq!(out.len(), 31);
        assert_eq!(out.last(), Some(&[-31, 0, 0]));
    }

    #[test]
    fn standing_on_the_hit_column_places_nothing() {
        let w = Fake::new().bounds(0, 16);
        for face in [Face::West, Face::Up, Face::North] {
            let out = collect(
                Mode::BuildToMe,
                &w,
                face,
                [5, 5, 5],
                player_at(5, 5, 5),
                Settings::default(),
            );
            assert!(out.is_empty(), "{face:?} should place nothing");
        }
    }

    #[test]
    fn one_block_apart_places_nothing() {
        // difference = 1, loop `1 < 1` never runs. The Java mod behaves the
        // same way: Build to Me never fills the single gap.
        let w = Fake::new().bounds(0, 16);
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [1, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn place_on_top_measures_to_the_eye() {
        let w = Fake::new().bounds(0, 16);
        // Feet at y 0, eyes at 1.62. Hitting the top face of a block at y 10.
        let down = collect(
            Mode::BuildToMe,
            &w,
            Face::Down,
            [0, 10, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        // |10 - 0| = 10 → 9 blocks.
        assert_eq!(down.len(), 9);

        let on_top = collect(
            Mode::BuildToMe,
            &w,
            Face::Down,
            [0, 10, 0],
            player_at(0, 0, 0),
            Settings {
                place_on_top: true,
                ..Settings::default()
            },
        );
        // (int) 1.62 = 1, |10 - 1| = 9 → 8 blocks.
        assert_eq!(on_top.len(), 8);
    }

    #[test]
    fn the_eye_cast_truncates_towards_zero_below_y_zero() {
        // Java's `(int) yPos` truncates, `getBlockY()` floors. At eye
        // -3.38 the two differ: (int) gives -3, floor gives -4.
        let w = Fake::new().bounds(-64, 384);
        let player = PlayerAt {
            block: [0, -5, 0],
            eye_y: -3.38,
        };
        let feet = collect(
            Mode::BuildToMe,
            &w,
            Face::Down,
            [0, 10, 0],
            player,
            Settings::default(),
        );
        // |10 - (-5)| = 15 → 14 blocks.
        assert_eq!(feet.len(), 14);

        let eye = collect(
            Mode::BuildToMe,
            &w,
            Face::Down,
            [0, 10, 0],
            player,
            Settings {
                place_on_top: true,
                ..Settings::default()
            },
        );
        // (int) -3.38 = -3 (towards zero), |10 - (-3)| = 13 → 12 blocks.
        // A floor would give -4 and 13 blocks, so this pins the difference.
        assert_eq!(eye.len(), 12);
    }

    #[test]
    fn every_face_runs_the_right_way() {
        let w = Fake::new().bounds(0, 64);
        let cases = [
            (Face::West, [10, 0, 0], [0, 0, 0], [-1, 0, 0]),
            (Face::East, [0, 0, 0], [10, 0, 0], [1, 0, 0]),
            (Face::Down, [0, 10, 0], [0, 0, 0], [0, -1, 0]),
            (Face::Up, [0, 0, 0], [0, 10, 0], [0, 1, 0]),
            (Face::North, [0, 0, 10], [0, 0, 0], [0, 0, -1]),
            (Face::South, [0, 0, 0], [0, 0, 10], [0, 0, 1]),
        ];
        for (face, start, player, first) in cases {
            let out = collect(
                Mode::BuildToMe,
                &w,
                face,
                start,
                player_at(player[0], player[1], player[2]),
                Settings::default(),
            );
            assert_eq!(out.len(), 9, "{face:?}");
            assert_eq!(out.first(), Some(&first), "{face:?}");
        }
    }

    #[test]
    fn blocks_in_the_way_are_left_out_but_do_not_stop_the_run() {
        // isPosValid is checked per position, and an invalid one is skipped
        // rather than ending the run: the Java loop has no break.
        let w = Fake::new()
            .bounds(0, 16)
            .with([10, 0, 0], "minecraft:stone")
            .with([5, 0, 0], "minecraft:dirt");
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [10, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        // Nine candidates at x 9..1; x 5 is taken by dirt, which is not
        // replaceable, so eight remain and the run continues past it.
        assert_eq!(out.len(), 8);
        assert!(!out.contains(&[-5, 0, 0]));
        assert!(out.contains(&[-6, 0, 0]));
        assert!(out.contains(&[-4, 0, 0]));
    }

    #[test]
    fn replaceable_blocks_are_built_over() {
        let mut w = Fake::new().bounds(0, 16).with([5, 0, 0], "minecraft:short_grass");
        w.replaceable.insert("minecraft:short_grass".to_owned());
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [10, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        assert_eq!(out.len(), 9);
        assert!(out.contains(&[-5, 0, 0]));
    }

    #[test]
    fn the_worlds_floor_and_ceiling_cut_the_run() {
        // A world from y 0 to 16, building down from y 10 towards a player
        // far below: positions under y 0 are left out.
        let w = Fake::new().bounds(0, 16);
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::Down,
            [0, 10, 0],
            player_at(0, -20, 0),
            Settings::default(),
        );
        // Candidates y 9 down to 10-29; only y 9..0 are in the world.
        assert_eq!(out.len(), 10);
        assert_eq!(out.last(), Some(&[0, -10, 0]));
    }

    #[test]
    fn protected_positions_are_left_out() {
        let mut w = Fake::new().bounds(0, 16);
        w.protected.insert([5, 0, 0]);
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [10, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        assert_eq!(out.len(), 8);
        assert!(!out.contains(&[-5, 0, 0]));
    }

    #[test]
    fn an_unloaded_position_is_not_built() {
        let mut w = Fake::new().bounds(0, 16);
        w.unloaded.insert([5, 0, 0]);
        let out = collect(
            Mode::BuildToMe,
            &w,
            Face::West,
            [10, 0, 0],
            player_at(0, 0, 0),
            Settings::default(),
        );
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn mode_ids_round_trip() {
        assert_eq!(Mode::from_id("buildinggadgets2:build_to_me"), Some(Mode::BuildToMe));
        assert_eq!(Mode::from_id("buildinggadgets2:stairs"), None);
        assert_eq!(Mode::BuildToMe.i18n(), "buildinggadgets2.modes.build_to_me");
    }
}
