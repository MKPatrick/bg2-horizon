//! The gadget's own raycast: `VectorHelper.getLookingAt`.
//!
//! The Java mod does not use the block the player clicked. It calls
//! `player.pick(Config.RAYTRACE_RANGE, 0f, fluids)` itself, because the range
//! is 32 blocks by default and vanilla's reach is about 4.5 — building at a
//! distance is the point. `ClipContext.Fluid.ANY` or `NONE` comes from the
//! gadget's `raytrace_fluid` setting.
//!
//! Vanilla's `pick` walks the blocks along the ray and stops at the first
//! whose **outline** shape the ray enters. Horizon gives a server mod a
//! block's collision boxes (`Host::block_collision`) but not its outline, so
//! this uses the collision boxes where a block has them and the whole block
//! where it has none (a flower, a torch — vanilla's outline for those is
//! smaller than a full block, so the hit point can differ by less than a
//! block at their edges). Everything solid, which is what a gadget is aimed
//! at, matches.

use crate::world::{Blocks, Face};

/// Where the ray met a block: the block, and the face it entered through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub pos: [i32; 3],
    pub face: Face,
    /// Where the ray meets the block, in world coordinates.
    pub point: [f64; 3],
}

/// The largest range the Java mod's config allows (`rayTraceRange`, 1 to 64).
pub const MAX_RANGE: f64 = 64.0;

/// The first block along `dir` from `eye`, within `range` blocks.
///
/// `fluids` is the `raytrace_fluid` setting: with it off, water and lava are
/// passed through, as `ClipContext.Fluid.NONE` does.
pub fn cast(
    blocks: &dyn Blocks,
    eye: [f64; 3],
    dir: [f64; 3],
    range: f64,
    fluids: bool,
) -> Option<Hit> {
    let range = range.clamp(0.0, MAX_RANGE);
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if !len.is_finite() || len <= 0.0 || range <= 0.0 {
        return None;
    }
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];

    // A voxel walk (Amanatides and Woo): step from block to block along the
    // ray, so no block on the way is missed and none is visited twice.
    let mut block = [
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    ];
    let step = [
        if dir[0] > 0.0 { 1 } else { -1 },
        if dir[1] > 0.0 { 1 } else { -1 },
        if dir[2] > 0.0 { 1 } else { -1 },
    ];
    // How far along the ray until the next boundary on each axis, and how far
    // between boundaries. An axis the ray does not move along never comes up.
    let mut next = [0.0f64; 3];
    let mut delta = [f64::INFINITY; 3];
    for i in 0..3 {
        if dir[i] == 0.0 {
            next[i] = f64::INFINITY;
            continue;
        }
        delta[i] = (1.0 / dir[i]).abs();
        let bound = if dir[i] > 0.0 {
            f64::from(block[i]) + 1.0
        } else {
            f64::from(block[i])
        };
        next[i] = (bound - eye[i]) / dir[i];
    }

    // The face the ray entered the current block through; none for the first,
    // which holds the eye.
    let mut entered: Option<Face> = None;
    let mut travelled = 0.0f64;

    loop {
        if let Some(face) = entered {
            let point = [
                eye[0] + dir[0] * travelled,
                eye[1] + dir[1] * travelled,
                eye[2] + dir[2] * travelled,
            ];
            if let Some(hit) = test(blocks, block, face, point, fluids) {
                return Some(hit);
            }
        } else if let Some(hit) = test_first(blocks, block, eye, dir, fluids) {
            // The block holding the eye: only a hit when the eye is inside it.
            return Some(hit);
        }

        // Step to the next block, along whichever boundary comes first.
        let axis = if next[0] <= next[1] && next[0] <= next[2] {
            0
        } else if next[1] <= next[2] {
            1
        } else {
            2
        };
        travelled = next[axis];
        if travelled > range {
            return None;
        }
        block[axis] += step[axis];
        next[axis] += delta[axis];
        entered = Some(match (axis, step[axis] > 0) {
            (0, true) => Face::West,
            (0, false) => Face::East,
            (1, true) => Face::Down,
            (1, false) => Face::Up,
            (_, true) => Face::North,
            (_, false) => Face::South,
        });
    }
}

/// Whether the ray entering `block` through `face` at `point` stops there.
fn test(
    blocks: &dyn Blocks,
    block: [i32; 3],
    face: Face,
    point: [f64; 3],
    fluids: bool,
) -> Option<Hit> {
    let name = blocks.block(block)?; // an unloaded chunk ends the ray
    if blocks.is_air(&name) {
        return None;
    }
    if blocks.is_fluid(&name) && !fluids {
        return None;
    }
    let boxes = blocks.collision(block);
    if boxes.is_empty() {
        // No collision shape: treated as a whole block (see the module note).
        return Some(Hit {
            pos: block,
            face,
            point,
        });
    }
    let local = [
        point[0] - f64::from(block[0]),
        point[1] - f64::from(block[1]),
        point[2] - f64::from(block[2]),
    ];
    boxes
        .iter()
        .any(|b| inside(local, *b, face))
        .then_some(Hit {
            pos: block,
            face,
            point,
        })
}

/// Whether the entry point lies on the face of one of the block's boxes.
///
/// The point is on the block's own boundary, so the box is tested on the two
/// axes across the face only; the third is the face itself.
fn inside(local: [f64; 3], b: [f64; 6], face: Face) -> bool {
    const SLACK: f64 = 1e-7;
    let across = match face.axis() {
        crate::world::Axis::X => [1, 2],
        crate::world::Axis::Y => [0, 2],
        crate::world::Axis::Z => [0, 1],
    };
    // The box must reach the face the ray came in through.
    let (lo, hi) = match face {
        Face::West => (b[0], b[0]),
        Face::East => (b[3], b[3]),
        Face::Down => (b[1], b[1]),
        Face::Up => (b[4], b[4]),
        Face::North => (b[2], b[2]),
        Face::South => (b[5], b[5]),
    };
    let touches = match face {
        Face::West | Face::Down | Face::North => lo <= SLACK,
        Face::East | Face::Up | Face::South => hi >= 1.0 - SLACK,
    };
    if !touches {
        return false;
    }
    across.into_iter().all(|i| {
        local[i] >= b[i] - SLACK && local[i] <= b[i + 3] + SLACK
    })
}

/// The block the eye is in: a hit only when the eye is inside its shape, as
/// vanilla's clip does for a player standing in a block.
fn test_first(
    blocks: &dyn Blocks,
    block: [i32; 3],
    eye: [f64; 3],
    _dir: [f64; 3],
    fluids: bool,
) -> Option<Hit> {
    let name = blocks.block(block)?;
    if blocks.is_air(&name) {
        return None;
    }
    if blocks.is_fluid(&name) && !fluids {
        return None;
    }
    let local = [
        eye[0] - f64::from(block[0]),
        eye[1] - f64::from(block[1]),
        eye[2] - f64::from(block[2]),
    ];
    let boxes = blocks.collision(block);
    let within = |b: &[f64; 6]| {
        (0..3).all(|i| local[i] >= b[i] && local[i] <= b[i + 3])
    };
    if boxes.is_empty() || !boxes.iter().any(within) {
        return None;
    }
    Some(Hit {
        pos: block,
        face: Face::Up,
        point: eye,
    })
}

/// The unit vector a yaw and pitch in degrees look along
/// (`Entity.getViewVector`).
pub fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let yaw = f64::from(yaw).to_radians();
    let pitch = f64::from(pitch).to_radians();
    let (sy, cy) = (yaw.sin(), yaw.cos());
    let (sp, cp) = (pitch.sin(), pitch.cos());
    [-sy * cp, -sp, cy * cp]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Fake;

    /// Looking straight down the +x axis from the middle of a block.
    const EYE: [f64; 3] = [0.5, 0.5, 0.5];
    const EAST: [f64; 3] = [1.0, 0.0, 0.0];

    #[test]
    fn finds_the_first_block_along_the_ray() {
        let w = Fake::new().with([4, 0, 0], "minecraft:stone");
        let hit = cast(&w, EYE, EAST, 32.0, false).expect("a hit");
        assert_eq!(hit.pos, [4, 0, 0]);
        // Entered through its west face, coming from -x.
        assert_eq!(hit.face, Face::West);
    }

    #[test]
    fn stops_at_the_nearest_of_several() {
        let w = Fake::new()
            .with([2, 0, 0], "minecraft:stone")
            .with([6, 0, 0], "minecraft:dirt");
        assert_eq!(cast(&w, EYE, EAST, 32.0, false).unwrap().pos, [2, 0, 0]);
    }

    #[test]
    fn range_is_respected() {
        let w = Fake::new().with([10, 0, 0], "minecraft:stone");
        assert!(cast(&w, EYE, EAST, 5.0, false).is_none());
        assert!(cast(&w, EYE, EAST, 32.0, false).is_some());
    }

    #[test]
    fn air_is_passed_through() {
        let w = Fake::new();
        assert!(cast(&w, EYE, EAST, 32.0, false).is_none());
    }

    #[test]
    fn fluids_only_stop_the_ray_when_asked() {
        let w = Fake::new()
            .with([3, 0, 0], "minecraft:water")
            .with([8, 0, 0], "minecraft:stone");
        // raytrace_fluid off: the water is passed through, as
        // ClipContext.Fluid.NONE does.
        assert_eq!(cast(&w, EYE, EAST, 32.0, false).unwrap().pos, [8, 0, 0]);
        // raytrace_fluid on: it stops at the water.
        assert_eq!(cast(&w, EYE, EAST, 32.0, true).unwrap().pos, [3, 0, 0]);
    }

    #[test]
    fn an_unloaded_chunk_is_passed_through() {
        // Vanilla's `Level.getBlockState` answers void air outside the loaded
        // chunks and `clip` walks on, so the ray does too. Nothing is built
        // there regardless: `is_pos_valid` refuses a position whose block
        // cannot be read (see `modes::tests::an_unloaded_position_is_not_built`).
        let mut w = Fake::new().with([6, 0, 0], "minecraft:stone");
        w.unloaded.insert([3, 0, 0]);
        assert_eq!(cast(&w, EYE, EAST, 32.0, false).unwrap().pos, [6, 0, 0]);
    }

    #[test]
    fn a_block_without_collision_is_still_hit() {
        // A flower has no collision box. Vanilla's outline pick hits it, so
        // this does too, as a whole block.
        let mut w = Fake::new().with([3, 0, 0], "minecraft:poppy");
        w.no_collision.insert("minecraft:poppy".to_owned());
        assert_eq!(cast(&w, EYE, EAST, 32.0, false).unwrap().pos, [3, 0, 0]);
    }

    #[test]
    fn faces_come_out_right_from_every_side() {
        let w = Fake::new().with([0, 0, 0], "minecraft:stone");
        let cases = [
            ([0.5, 0.5, -4.0], [0.0, 0.0, 1.0], Face::North),
            ([0.5, 0.5, 4.5], [0.0, 0.0, -1.0], Face::South),
            ([-4.0, 0.5, 0.5], [1.0, 0.0, 0.0], Face::West),
            ([4.5, 0.5, 0.5], [-1.0, 0.0, 0.0], Face::East),
            ([0.5, -4.0, 0.5], [0.0, 1.0, 0.0], Face::Down),
            ([0.5, 4.5, 0.5], [0.0, -1.0, 0.0], Face::Up),
        ];
        for (eye, dir, face) in cases {
            let hit = cast(&w, eye, dir, 32.0, false)
                .unwrap_or_else(|| panic!("no hit from {eye:?} along {dir:?}"));
            assert_eq!(hit.pos, [0, 0, 0], "from {eye:?}");
            assert_eq!(hit.face, face, "from {eye:?}");
        }
    }

    #[test]
    fn a_diagonal_ray_misses_nothing_on_the_way() {
        // The block at [2,2,0] is on the diagonal; the one past it is not
        // reached.
        let w = Fake::new()
            .with([2, 2, 0], "minecraft:stone")
            .with([4, 4, 0], "minecraft:dirt");
        let hit = cast(&w, [0.5, 0.5, 0.5], [1.0, 1.0, 0.0], 32.0, false).unwrap();
        assert_eq!(hit.pos, [2, 2, 0]);
    }

    #[test]
    fn a_zero_or_bad_direction_is_no_hit() {
        let w = Fake::new().with([1, 0, 0], "minecraft:stone");
        assert!(cast(&w, EYE, [0.0, 0.0, 0.0], 32.0, false).is_none());
        assert!(cast(&w, EYE, [f64::NAN, 0.0, 0.0], 32.0, false).is_none());
        assert!(cast(&w, EYE, EAST, 0.0, false).is_none());
    }

    #[test]
    fn view_vector_matches_vanillas_angles() {
        // Vanilla's angles: yaw 0 looks south (+z), 90 west (-x), 180 north
        // (-z), -90 east (+x); pitch -90 is straight up.
        let close = |a: [f64; 3], b: [f64; 3]| {
            (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
        };
        assert!(close(view_vector(0.0, 0.0), [0.0, 0.0, 1.0]));
        assert!(close(view_vector(180.0, 0.0), [0.0, 0.0, -1.0]));
        assert!(close(view_vector(90.0, 0.0), [-1.0, 0.0, 0.0]));
        assert!(close(view_vector(-90.0, 0.0), [1.0, 0.0, 0.0]));
        assert!(close(view_vector(0.0, -90.0), [0.0, 1.0, 0.0]));
        assert!(close(view_vector(0.0, 90.0), [0.0, -1.0, 0.0]));
    }
}
