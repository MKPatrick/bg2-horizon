//! The ghost preview: what a click would build, drawn before it happens.
//!
//! The thing Building Gadgets is known for. The Java mod bakes translucent
//! block models into a VBO (`VBORenderer`, `DireBufferBuilder`,
//! `FakeRenderingWorld`) and draws them in the world.
//!
//! This draws **translucent boxes** instead, through Horizon's
//! `world_primitives`: the right blocks in the right places, in the gadget's
//! colour, with the build's bounds outlined. Textured block models are the
//! other half of M4 and need a pass of the mod's own (`world_drawn` with
//! `block_sprites`, the way AE2's `ae2_draw.rs` does it). What the preview
//! *shows* is already right; what is left is how it looks.
//!
//! ## It runs on the client, with the client's own answers
//!
//! The build runs on the server, where `level_block_state` and the
//! `minecraft:replaceable` tag are. A client has neither: `Host::level` is
//! None there, so the preview reads the world with `Host::block`, which gives
//! a block's name without its state, and decides "can I build over this?"
//! from its collision rather than from the tag.
//!
//! For air and for solid blocks — every ordinary case — the two agree. They
//! can differ for a block whose tag says replaceable but which collides, and
//! for Surface's "is the block behind the same one?" test, which compares
//! names here and whole states on the server. The server always has the last
//! word: the preview can be a block out, the build cannot.

use crate::ids;
use crate::modes::{self, Ctx, PlayerAt};
use crate::state;
use crate::world::{Blocks, Face};
use horizon_client::mod_api::{EquipmentSlot, Host, ItemStack, WorldPrimitives};

/// The Java mod's `Config.RAYTRACE_RANGE` default.
const RANGE: f64 = 32.0;

/// The ghosts, a soft green (`0xAARRGGBB`).
const GHOST: u32 = 0x5566_ff66;
/// The ghosts of the Exchanger, which replaces rather than adds.
const GHOST_EXCHANGE: u32 = 0x5566_aaff;
/// The outline around the whole build.
const BOUNDS: u32 = 0xaaff_ffff;
/// How far a ghost is inset from a full block, so neighbouring ghosts read as
/// separate and a ghost over an existing block does not z-fight with it.
const INSET: f64 = 0.03;

/// Draw the preview for whatever the player is holding.
///
/// Called from `Hooks::world_primitives`, once a frame while the world draws.
pub fn draw(host: &dyn Host, primitives: &mut WorldPrimitives) {
    let Some((gadget, exchanging)) = held_gadget(host) else {
        return;
    };
    let Some(state) = state::block_state(host, &gadget) else {
        return; // nothing picked yet
    };
    let Some((start, face)) = looking_at(host) else {
        return;
    };
    let Some(player) = player_at(host) else {
        return;
    };

    let blocks = ClientBlocks { host };
    let ctx = Ctx {
        blocks: &blocks,
        exchanging,
        settings: state::settings(host, &gadget),
        player,
        hit_side: face,
        start,
        state: &state,
    };
    let offsets = modes::collect(state::mode(host, &gadget), &ctx);
    if offsets.is_empty() {
        return;
    }

    let colour = if exchanging { GHOST_EXCHANGE } else { GHOST };
    let mut lo = [i32::MAX; 3];
    let mut hi = [i32::MIN; 3];
    for o in &offsets {
        let p = [start[0] + o[0], start[1] + o[1], start[2] + o[2]];
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
        primitives.filled_box(
            [
                f64::from(p[0]) + INSET,
                f64::from(p[1]) + INSET,
                f64::from(p[2]) + INSET,
            ],
            [
                f64::from(p[0]) + 1.0 - INSET,
                f64::from(p[1]) + 1.0 - INSET,
                f64::from(p[2]) + 1.0 - INSET,
            ],
            colour,
        );
    }

    // One outline around the lot, so the shape reads at a glance.
    if offsets.len() > 1 {
        primitives.box_outline(
            [f64::from(lo[0]), f64::from(lo[1]), f64::from(lo[2])],
            [
                f64::from(hi[0]) + 1.0,
                f64::from(hi[1]) + 1.0,
                f64::from(hi[2]) + 1.0,
            ],
            BOUNDS,
            2.0,
        );
    }
}

/// The gadget in the player's own hand, and whether it is the Exchanger.
///
/// `BaseGadget.getGadget` looks in the main hand, then the off hand.
fn held_gadget(host: &dyn Host) -> Option<(ItemStack, bool)> {
    for slot in [EquipmentSlot::MainHand, EquipmentSlot::OffHand] {
        if let Some(stack) = host.equipment(slot) {
            if ids::GADGETS.contains(&stack.item.as_ref()) {
                let exchanging = stack.item.as_ref() == ids::EXCHANGER;
                return Some((stack, exchanging));
            }
        }
    }
    None
}

/// The block the player looks at and the face they look at it on.
///
/// `Host::raycast` is the client's own view pick, which crosses a block's
/// outline as vanilla's does, so this matches what the crosshair highlights —
/// and reaches the gadget's full range, not the player's.
fn looking_at(host: &dyn Host) -> Option<([i32; 3], Face)> {
    let hit = host.raycast(RANGE as f32)?;
    let pos = [hit.block_x, hit.block_y, hit.block_z];
    Some((pos, face_of(pos, [hit.x, hit.y, hit.z])))
}

/// Which face of `pos` the point lies on.
///
/// `NativeHit` carries where the ray met the block but not which side, so the
/// side is the boundary the point sits on: the axis whose coordinate is at
/// the block's edge.
fn face_of(pos: [i32; 3], point: [f64; 3]) -> Face {
    const EDGE: f64 = 1e-4;
    let mut best = (f64::INFINITY, Face::Up);
    for (axis, (low, high)) in [
        (0, (Face::West, Face::East)),
        (1, (Face::Down, Face::Up)),
        (2, (Face::North, Face::South)),
    ] {
        let local = point[axis] - f64::from(pos[axis]);
        if local.abs() < best.0 {
            best = (local.abs(), low);
        }
        if (local - 1.0).abs() < best.0 {
            best = ((local - 1.0).abs(), high);
        }
    }
    if best.0 > 0.5 + EDGE {
        // The point is not on a boundary at all (the eye inside a block):
        // the top face is as good an answer as any.
        return Face::Up;
    }
    best.1
}

/// Where the player is, from the client's own state.
fn player_at(host: &dyn Host) -> Option<PlayerAt> {
    let state = host.client_state()?;
    let eye = state.eye_position?;
    // `eye_position` is the eyes; the feet are below by the eye height, which
    // the camera reports.
    let eye_height = f64::from(host.camera().map_or(1.62, |c| c.eye_height));
    let feet_y = eye[1] - eye_height;
    Some(PlayerAt {
        block: [
            eye[0].floor() as i32,
            feet_y.floor() as i32,
            eye[2].floor() as i32,
        ],
        eye_y: eye[1],
        y: feet_y,
        facing: Face::from_yaw(state.yaw),
    })
}

/// [`Blocks`] over the client's own world.
struct ClientBlocks<'a> {
    host: &'a dyn Host,
}

impl Blocks for ClientBlocks<'_> {
    fn block(&self, pos: [i32; 3]) -> Option<String> {
        // `Host::block` answers air for anything not loaded, which is what a
        // client sees anyway.
        Some(self.host.block(pos))
    }

    fn state(&self, pos: [i32; 3]) -> Option<String> {
        // The client has no whole-state read, so Surface's "same block
        // behind?" test compares names here and whole states on the server.
        self.block(pos)
    }

    fn has_block_entity(&self, _pos: [i32; 3]) -> bool {
        // Only the Exchanger's `affect_tiles` asks, and only the server's
        // answer decides what is really replaced.
        false
    }

    fn is_replaceable(&self, pos: [i32; 3]) -> bool {
        let Some(block) = self.block(pos) else {
            return false;
        };
        if self.is_air(&block) {
            return true;
        }
        // No `minecraft:replaceable` tag on a client, so collision stands in:
        // grass, flowers and fluids do not collide and are built over, and
        // everything solid is not. See the note at the top.
        match self.host.client_block_obstruction(pos) {
            Some((full_collision, _occludes)) => !full_collision,
            None => false,
        }
    }

    fn collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        self.host.block_collision(pos)
    }

    fn height(&self) -> (i32, i32) {
        // The client has no level handle to ask, and every vanilla dimension
        // this port builds in is within these bounds.
        (-64, 384)
    }

    fn may_place(&self, _pos: [i32; 3]) -> bool {
        // Protection is the server's to judge, and it is not readable here.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hit_point_names_the_face_it_lies_on() {
        let pos = [10, 20, 30];
        let cases = [
            ([10.0, 20.5, 30.5], Face::West),
            ([11.0, 20.5, 30.5], Face::East),
            ([10.5, 20.0, 30.5], Face::Down),
            ([10.5, 21.0, 30.5], Face::Up),
            ([10.5, 20.5, 30.0], Face::North),
            ([10.5, 20.5, 31.0], Face::South),
        ];
        for (point, face) in cases {
            assert_eq!(face_of(pos, point), face, "point {point:?}");
        }
    }

    #[test]
    fn a_face_is_named_even_for_a_point_just_off_the_edge() {
        // A real hit point carries rounding; the nearest boundary still wins.
        assert_eq!(face_of([0, 0, 0], [0.0001, 0.5, 0.5]), Face::West);
        assert_eq!(face_of([0, 0, 0], [0.9999, 0.5, 0.5]), Face::East);
    }

    #[test]
    fn negative_blocks_name_their_faces_too() {
        assert_eq!(face_of([-5, -3, -8], [-5.0, -2.5, -7.5]), Face::West);
        assert_eq!(face_of([-5, -3, -8], [-4.5, -2.0, -7.5]), Face::Up);
    }
}
