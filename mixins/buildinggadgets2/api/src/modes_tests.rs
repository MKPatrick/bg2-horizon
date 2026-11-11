//! The modes against the Java source, case by case.
//!
//! Each test states what `collectWorld` does for that input and checks the
//! port answers the same. They need no game, so they run in milliseconds and
//! are the only thing standing between a subtle mistake and every build being
//! wrong.

use super::*;
use crate::world::Fake;

/// A world of air from y 0 to 64, with nothing in the way.
fn air() -> Fake {
    Fake::new().bounds(0, 64)
}

fn player(x: i32, y: i32, z: i32, facing: Face) -> PlayerAt {
    PlayerAt {
        block: [x, y, z],
        eye_y: f64::from(y) + 1.62,
        y: f64::from(y),
        facing,
    }
}

/// A context with the defaults: the Building Gadget, range 1, nothing set.
fn ctx<'a>(
    blocks: &'a Fake,
    hit_side: Face,
    start: [i32; 3],
    player: PlayerAt,
    range: i32,
) -> Ctx<'a> {
    Ctx {
        blocks,
        exchanging: false,
        settings: Settings {
            range,
            ..Settings::default()
        },
        player,
        hit_side,
        start,
        state: "minecraft:stone",
    }
}

fn sorted(mut v: Vec<[i32; 3]>) -> Vec<[i32; 3]> {
    v.sort_unstable();
    v
}

// --- Build to Me ---------------------------------------------------------

#[test]
fn build_to_me_runs_to_the_player_and_stops_one_short() {
    let w = air();
    let c = ctx(&w, Face::West, [10, 0, 0], player(0, 0, 0, Face::East), 1);
    let out = collect(Mode::BuildToMe, &c);
    assert_eq!(out.len(), 9);
    assert_eq!(out.first(), Some(&[-1, 0, 0]));
    assert_eq!(out.last(), Some(&[-9, 0, 0]));
}

#[test]
fn build_to_me_clamps_at_32() {
    let w = air();
    let c = ctx(&w, Face::West, [100, 0, 0], player(0, 0, 0, Face::East), 1);
    assert_eq!(collect(Mode::BuildToMe, &c).len(), 31);
}

#[test]
fn build_to_me_one_apart_places_nothing() {
    let w = air();
    let c = ctx(&w, Face::West, [1, 0, 0], player(0, 0, 0, Face::East), 1);
    assert!(collect(Mode::BuildToMe, &c).is_empty());
}

#[test]
fn build_to_me_place_on_top_measures_to_the_eye() {
    let w = air();
    let mut c = ctx(&w, Face::Down, [0, 10, 0], player(0, 0, 0, Face::North), 1);
    assert_eq!(collect(Mode::BuildToMe, &c).len(), 9);
    c.settings.place_on_top = true;
    // (int) 1.62 = 1, |10 - 1| = 9 -> 8 blocks.
    assert_eq!(collect(Mode::BuildToMe, &c).len(), 8);
}

#[test]
fn build_to_me_truncates_the_eye_towards_zero() {
    // Java's `(int) yPos` truncates, `getBlockY()` floors; below y 0 they
    // differ. Eye -3.38 truncates to -3, floors to -4.
    let w = Fake::new().bounds(-64, 384);
    let p = PlayerAt {
        block: [0, -5, 0],
        eye_y: -3.38,
        y: -5.0,
        facing: Face::North,
    };
    let mut c = ctx(&w, Face::Down, [0, 10, 0], p, 1);
    assert_eq!(collect(Mode::BuildToMe, &c).len(), 14); // |10 - (-5)| = 15
    c.settings.place_on_top = true;
    assert_eq!(collect(Mode::BuildToMe, &c).len(), 12); // |10 - (-3)| = 13
}

// --- Vertical Wall -------------------------------------------------------

#[test]
fn vertical_wall_on_a_side_is_a_square_around_the_hit() {
    // range 3, size 1: y from start-1 to start+1, and one block each way
    // across the hit axis. Hitting a west face (axis X) spreads along z.
    let w = air();
    let c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    let out = sorted(collect(Mode::VerticalWall, &c));
    assert_eq!(out.len(), 9);
    assert!(out.contains(&[0, 0, 0]));
    assert!(out.contains(&[0, -1, -1]));
    assert!(out.contains(&[0, 1, 1]));
    // Nothing moves along x, the hit axis.
    assert!(out.iter().all(|p| p[0] == 0));
}

#[test]
fn vertical_wall_up_builds_a_column_above_the_block() {
    // Hitting the top face: startY = start+1, endY = start + (range-1) + 1.
    let w = air();
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 3, Face::North), 3);
    let out = sorted(collect(Mode::VerticalWall, &c));
    // Facing north, its opposite is south (axis Z), so the wall spreads along
    // x by size 1: 3 wide and 3 tall.
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| (1..=3).contains(&p[1])));
    assert!(out.iter().all(|p| p[2] == 0));
}

#[test]
fn vertical_wall_down_builds_below() {
    let w = air();
    let c = ctx(&w, Face::Down, [0, 10, 0], player(0, 10, 3, Face::North), 3);
    let out = sorted(collect(Mode::VerticalWall, &c));
    assert_eq!(out.len(), 9);
    // endY = start + (range-1)*-1 + 1 = start - 1, startY = start + 1, so the
    // box runs from -1 to +1 relative once normalised.
    assert!(out.iter().all(|p| (-1..=1).contains(&p[1])));
}

// --- Vertical Column -----------------------------------------------------

#[test]
fn vertical_column_on_a_side_is_centred_on_the_hit() {
    let w = air();
    let c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 5);
    let out = sorted(collect(Mode::VerticalColumn, &c));
    // half = 2, so -2..=2 up the Y axis: five blocks.
    assert_eq!(out, vec![[0, -2, 0], [0, -1, 0], [0, 0, 0], [0, 1, 0], [0, 2, 0]]);
}

#[test]
fn vertical_column_up_runs_the_full_range_above() {
    let w = air();
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 3, Face::North), 4);
    let out = sorted(collect(Mode::VerticalColumn, &c));
    // Not exchanging: i in 1..=range above the block.
    assert_eq!(out, vec![[0, 1, 0], [0, 2, 0], [0, 3, 0], [0, 4, 0]]);
}

#[test]
fn the_exchanger_runs_a_column_along_the_players_facing() {
    let mut w = air();
    // The exchanger needs something to exchange.
    for i in -3..=3 {
        w.blocks.insert([i, 10, 0], "minecraft:dirt".to_owned());
    }
    let mut c = ctx(&w, Face::Up, [0, 10, 0], player(0, 12, 5, Face::West), 5);
    c.exchanging = true;
    c.settings.fuzzy = true;
    let out = sorted(collect(Mode::VerticalColumn, &c));
    // Facing west (-x), half = 2: the row runs along x, not up.
    assert_eq!(out.len(), 5);
    assert!(out.iter().all(|p| p[1] == 0 && p[2] == 0));
    assert_eq!(out.first(), Some(&[-2, 0, 0]));
    assert_eq!(out.last(), Some(&[2, 0, 0]));
}

// --- Surface -------------------------------------------------------------

#[test]
fn surface_covers_the_face_in_front_of_a_matching_floor() {
    // A 5x5 floor of stone at y 9; hitting its top face should offer the 3x3
    // (range 3, bound 1) of air above it.
    let mut w = air();
    for x in -2..=2 {
        for z in -2..=2 {
            w.blocks.insert([x, 9, z], "minecraft:stone".to_owned());
        }
    }
    let c = ctx(&w, Face::Up, [0, 9, 0], player(0, 12, 0, Face::North), 3);
    let out = sorted(collect(Mode::Surface, &c));
    assert_eq!(out.len(), 9);
    // One above the hit block, spread over x and z.
    assert!(out.iter().all(|p| p[1] == 1));
    assert!(out.contains(&[-1, 1, -1]));
    assert!(out.contains(&[1, 1, 1]));
}

#[test]
fn surface_stops_where_the_floor_is_a_different_block() {
    let mut w = air();
    for x in -2..=2 {
        for z in -2..=2 {
            w.blocks.insert([x, 9, z], "minecraft:stone".to_owned());
        }
    }
    // One block of the floor is dirt: not fuzzy, so the square skips above it.
    w.blocks.insert([1, 9, 1], "minecraft:dirt".to_owned());
    let c = ctx(&w, Face::Up, [0, 9, 0], player(0, 12, 0, Face::North), 3);
    let out = sorted(collect(Mode::Surface, &c));
    assert_eq!(out.len(), 8);
    assert!(!out.contains(&[1, 1, 1]));
}

#[test]
fn surface_fuzzy_takes_any_solid_floor() {
    let mut w = air();
    for x in -2..=2 {
        for z in -2..=2 {
            w.blocks.insert([x, 9, z], "minecraft:stone".to_owned());
        }
    }
    w.blocks.insert([1, 9, 1], "minecraft:dirt".to_owned());
    let mut c = ctx(&w, Face::Up, [0, 9, 0], player(0, 12, 0, Face::North), 3);
    c.settings.fuzzy = true;
    assert_eq!(collect(Mode::Surface, &c).len(), 9);
}

#[test]
fn surface_leaves_a_hole_where_the_floor_is_missing() {
    let mut w = air();
    for x in -2..=2 {
        for z in -2..=2 {
            w.blocks.insert([x, 9, z], "minecraft:stone".to_owned());
        }
    }
    w.blocks.remove(&[0, 9, 1]);
    let c = ctx(&w, Face::Up, [0, 9, 0], player(0, 12, 0, Face::North), 3);
    let out = collect(Mode::Surface, &c);
    assert_eq!(out.len(), 8);
    assert!(!out.contains(&[0, 1, 1]));
}

// --- Horizontal Wall -----------------------------------------------------

#[test]
fn horizontal_wall_on_the_top_face_is_a_square_floor() {
    let w = air();
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 4, Face::North), 3);
    let out = sorted(collect(Mode::HorizontalWall, &c));
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| p[1] == 0));
    assert!(out.contains(&[-1, 0, -1]));
    assert!(out.contains(&[1, 0, 1]));
}

#[test]
fn horizontal_wall_place_on_top_lifts_it_by_one() {
    let w = air();
    let mut c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 4, Face::North), 3);
    c.settings.place_on_top = true;
    let out = collect(Mode::HorizontalWall, &c);
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| p[1] == 1));
}

#[test]
fn horizontal_wall_on_a_side_runs_away_from_the_face() {
    let w = air();
    let c = ctx(&w, Face::East, [0, 10, 0], player(-4, 10, 0, Face::East), 3);
    let out = sorted(collect(Mode::HorizontalWall, &c));
    // range 3 away from the face, 3 wide across it, all on one level.
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| p[1] == 0));
    assert!(out.iter().all(|p| (1..=3).contains(&p[0])));
    assert!(out.iter().all(|p| (-1..=1).contains(&p[2])));
}

#[test]
fn horizontal_wall_inverts_on_a_west_face() {
    let w = air();
    let c = ctx(&w, Face::West, [0, 10, 0], player(4, 10, 0, Face::West), 3);
    let out = sorted(collect(Mode::HorizontalWall, &c));
    assert_eq!(out.len(), 9);
    // invertOnFace makes WEST run towards -x.
    assert!(out.iter().all(|p| (-3..=-1).contains(&p[0])));
}

// --- Stairs --------------------------------------------------------------

#[test]
fn stairs_climb_away_from_the_player() {
    let w = air();
    // Standing level with the hit block, hitting its west face.
    let c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 4);
    let out = collect(Mode::Stairs, &c);
    assert_eq!(out.len(), 4);
    // Each step rises by one and moves one further along x.
    for (i, p) in out.iter().enumerate() {
        let i = i as i32;
        assert_eq!(p[1], i, "step {i} rises wrong");
        assert_eq!(p[0], i, "step {i} moves wrong");
    }
}

#[test]
fn stairs_below_your_feet_descend() {
    let w = Fake::new().bounds(0, 64);
    // The hit block is below the player, on a horizontal face: the special
    // case in Java, which digs down and away.
    let c = ctx(&w, Face::East, [0, 5, 0], player(-3, 12, 0, Face::East), 3);
    let out = collect(Mode::Stairs, &c);
    assert_eq!(out.len(), 3);
    for (i, p) in out.iter().enumerate() {
        let i = i as i32;
        assert_eq!(p[1], -(i + 1), "step {i}");
        assert_eq!(p[0], i + 1, "step {i}");
    }
}

#[test]
fn stairs_below_your_feet_with_place_on_top_start_level() {
    let w = Fake::new().bounds(0, 64);
    let mut c = ctx(&w, Face::East, [0, 5, 0], player(-3, 12, 0, Face::East), 3);
    c.settings.place_on_top = true;
    let out = collect(Mode::Stairs, &c);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0][1], 0);
    assert_eq!(out[1][1], -1);
}

// --- Horizontal Row ------------------------------------------------------

#[test]
fn horizontal_row_runs_away_from_the_face() {
    let w = air();
    let c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 4);
    let out = collect(Mode::HorizontalRow, &c);
    // side = hitSide.getOpposite() = EAST, i in 0..range from the hit block.
    assert_eq!(out, vec![[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]);
}

#[test]
fn horizontal_row_on_the_top_face_follows_the_player() {
    let w = air();
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 5, Face::North), 3);
    let out = collect(Mode::HorizontalRow, &c);
    // Facing north (-z), the row runs that way from the hit block.
    assert_eq!(out, vec![[0, 0, 0], [0, 0, -1], [0, 0, -2]]);
}

#[test]
fn horizontal_row_place_on_top_lifts_the_row() {
    let w = air();
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.settings.place_on_top = true;
    let out = collect(Mode::HorizontalRow, &c);
    assert!(out.iter().all(|p| p[1] == 1));
}

#[test]
fn the_exchanger_runs_a_row_across_the_face() {
    let mut w = air();
    for z in -3..=3 {
        w.blocks.insert([10, 10, z], "minecraft:dirt".to_owned());
    }
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 5);
    c.exchanging = true;
    c.settings.fuzzy = true;
    let out = sorted(collect(Mode::HorizontalRow, &c));
    // side = EAST turned clockwise = SOUTH (+z), half = 2.
    assert_eq!(out.len(), 5);
    assert!(out.iter().all(|p| p[0] == 0 && p[1] == 0));
    assert_eq!(out.first(), Some(&[0, 0, -2]));
    assert_eq!(out.last(), Some(&[0, 0, 2]));
}

// --- Grid ----------------------------------------------------------------

#[test]
fn grid_lays_a_lattice_with_the_right_spacing() {
    let w = air();
    // range 1 -> the mode's own range is 2, spacing ((2-2)%6)+2 = 2,
    // extent 2*7/5 = 2. So x and z in -2..=2 at multiples of 2.
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 4, Face::North), 1);
    let out = sorted(collect(Mode::Grid, &c));
    assert_eq!(out.len(), 9);
    assert!(out.iter().all(|p| p[0] % 2 == 0 && p[2] % 2 == 0));
    assert!(out.contains(&[0, 0, 0]));
    assert!(out.contains(&[-2, 0, -2]));
    assert!(out.contains(&[2, 0, 2]));
}

#[test]
fn grid_place_on_top_lifts_it() {
    let w = air();
    let mut c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 4, Face::North), 1);
    c.settings.place_on_top = true;
    assert!(collect(Mode::Grid, &c).iter().all(|p| p[1] == 1));
}

// --- the connected-area filter -------------------------------------------

#[test]
fn connected_area_drops_what_the_start_cannot_reach() {
    // A row of dirt with a gap: with connected area on, only the part the
    // start touches is kept.
    let mut w = air();
    for z in [-2, -1, 0, 1] {
        w.blocks.insert([10, 10, z], "minecraft:dirt".to_owned());
    }
    // z = 2 is air, so the run breaks there; z = 3 is dirt but cut off.
    w.blocks.insert([10, 10, 3], "minecraft:dirt".to_owned());

    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 9);
    c.exchanging = true;
    c.settings.fuzzy = true;
    let all = sorted(collect(Mode::HorizontalRow, &c));
    assert!(all.contains(&[0, 0, 3]), "without the filter z=3 is in");

    c.settings.connected_area = true;
    let kept = sorted(collect(Mode::HorizontalRow, &c));
    assert!(!kept.contains(&[0, 0, 3]), "z=3 is cut off by the gap");
    assert!(kept.contains(&[0, 0, 1]));
    assert!(kept.contains(&[0, 0, -2]));
}

// --- what the exchanger may replace --------------------------------------

#[test]
fn the_exchanger_refuses_air() {
    let w = air();
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.exchanging = true;
    assert!(collect(Mode::VerticalColumn, &c).is_empty());
}

#[test]
fn the_exchanger_matches_the_block_it_looks_at() {
    let mut w = air();
    for y in 9..=11 {
        w.blocks.insert([10, y, 0], "minecraft:stone".to_owned());
    }
    w.blocks.insert([10, 11, 0], "minecraft:dirt".to_owned());
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.exchanging = true;
    let out = sorted(collect(Mode::VerticalColumn, &c));
    // Looking at stone: the dirt one block up is left alone.
    assert_eq!(out, vec![[0, -1, 0], [0, 0, 0]]);
}

#[test]
fn the_exchanger_keeps_block_entities_unless_told() {
    let mut w = air();
    for y in 9..=11 {
        w.blocks.insert([10, y, 0], "minecraft:stone".to_owned());
    }
    w.block_entities.insert([10, 11, 0]);
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.exchanging = true;
    assert_eq!(collect(Mode::VerticalColumn, &c).len(), 2);
    c.settings.affect_tiles = true;
    assert_eq!(collect(Mode::VerticalColumn, &c).len(), 3);
}

#[test]
fn the_exchanger_never_replaces_the_render_block() {
    let mut w = air();
    for y in 9..=11 {
        w.blocks.insert([10, y, 0], "buildinggadgets2:render_block".to_owned());
    }
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.exchanging = true;
    c.settings.fuzzy = true;
    assert!(collect(Mode::VerticalColumn, &c).is_empty());
}

#[test]
fn fuzzy_exchanging_skips_the_block_it_would_place() {
    let mut w = air();
    for y in 9..=11 {
        w.blocks.insert([10, y, 0], "minecraft:dirt".to_owned());
    }
    // One is already the gadget's block, so there is nothing to do there.
    w.blocks.insert([10, 11, 0], "minecraft:stone".to_owned());
    let mut c = ctx(&w, Face::West, [10, 10, 0], player(0, 10, 0, Face::East), 3);
    c.exchanging = true;
    c.settings.fuzzy = true;
    let out = sorted(collect(Mode::VerticalColumn, &c));
    assert_eq!(out, vec![[0, -1, 0], [0, 0, 0]]);
}

// --- the shared checks ---------------------------------------------------

#[test]
fn the_worlds_floor_and_ceiling_cut_every_mode() {
    let w = Fake::new().bounds(0, 16);
    let c = ctx(&w, Face::Down, [0, 10, 0], player(0, -20, 0, Face::North), 1);
    let out = collect(Mode::BuildToMe, &c);
    assert_eq!(out.len(), 10);
    assert_eq!(out.last(), Some(&[0, -10, 0]));
}

#[test]
fn protected_positions_are_left_out() {
    let mut w = air();
    w.protected.insert([5, 0, 0]);
    let c = ctx(&w, Face::West, [10, 0, 0], player(0, 0, 0, Face::East), 1);
    let out = collect(Mode::BuildToMe, &c);
    assert_eq!(out.len(), 8);
    assert!(!out.contains(&[-5, 0, 0]));
}

#[test]
fn a_block_in_the_way_is_skipped_not_a_stopping_point() {
    let w = air().with([5, 0, 0], "minecraft:dirt");
    let c = ctx(&w, Face::West, [10, 0, 0], player(0, 0, 0, Face::East), 1);
    let out = collect(Mode::BuildToMe, &c);
    assert_eq!(out.len(), 8);
    assert!(out.contains(&[-4, 0, 0]));
    assert!(out.contains(&[-6, 0, 0]));
}

// --- the mode list itself ------------------------------------------------

#[test]
fn mode_ids_round_trip() {
    for mode in Mode::ALL {
        assert_eq!(Mode::from_id(&mode.id()), Some(mode));
        assert_eq!(Mode::from_id(mode.path()), Some(mode));
    }
    assert_eq!(Mode::from_id("buildinggadgets2:nonsense"), None);
    assert_eq!(Mode::BuildToMe.i18n(), "buildinggadgets2.modes.build_to_me");
}

#[test]
fn the_gadgets_mode_lists_match_gadget_modes() {
    // GadgetModes registers 8 for BUILDING and 4 for EXCHANGING.
    assert_eq!(Mode::BUILDING.len(), 8);
    assert_eq!(Mode::EXCHANGING.len(), 4);
    for m in Mode::EXCHANGING {
        assert!(Mode::ALL.contains(&m));
    }
}

#[test]
fn a_yaw_names_the_direction_the_player_faces() {
    // Direction.fromYRot: 0 south, 90 west, 180 north, 270 east.
    assert_eq!(Face::from_yaw(0.0), Face::South);
    assert_eq!(Face::from_yaw(90.0), Face::West);
    assert_eq!(Face::from_yaw(180.0), Face::North);
    assert_eq!(Face::from_yaw(-90.0), Face::East);
    assert_eq!(Face::from_yaw(270.0), Face::East);
    // Rounding to the nearest quarter turn.
    assert_eq!(Face::from_yaw(44.0), Face::South);
    assert_eq!(Face::from_yaw(46.0), Face::West);
}

#[test]
fn clockwise_turns_the_horizontal_faces() {
    assert_eq!(Face::North.clockwise(), Face::East);
    assert_eq!(Face::East.clockwise(), Face::South);
    assert_eq!(Face::South.clockwise(), Face::West);
    assert_eq!(Face::West.clockwise(), Face::North);
}

#[test]
fn copy_cut_and_paste_collect_nothing_yet() {
    let w = air();
    let c = ctx(&w, Face::Up, [0, 10, 0], player(0, 10, 4, Face::North), 3);
    for mode in [Mode::Copy, Mode::Cut, Mode::Paste] {
        assert!(collect(mode, &c).is_empty(), "{mode:?}");
    }
}
