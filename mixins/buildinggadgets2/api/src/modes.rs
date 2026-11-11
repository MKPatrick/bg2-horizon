//! The building modes: which blocks a click fills.
//!
//! This is the part of the port that has to match the Java mod exactly — a
//! mode that collects a slightly different set makes every build subtly
//! wrong — and it is also the part that needs no running game, so it is
//! written against [`Blocks`] and checked by the tests below, each read off
//! the Java source case by case.
//!
//! The Java side is `util/modes/*.java`. A mode's `collectWorld` returns
//! positions **relative to the hit block**, not absolute ones, while it checks
//! validity on the absolute position. `BuildingUtils.build` then offsets each
//! by the hit position again. This port keeps that split: [`collect`] answers
//! relative offsets.
//!
//! The same class serves the Building Gadget and the Exchanger in Java, with
//! an `isExchanging` flag from its constructor, and several modes branch on
//! it. Here that is [`Ctx::exchanging`].

use crate::world::{Axis, Blocks, Face};
use std::collections::{HashSet, VecDeque};

/// The settings a mode reads off the gadget (`GadgetNBT`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settings {
    /// `PLACE_ON_TOP`.
    pub place_on_top: bool,
    /// `RAYTRACE_FLUID`, used by the raycast rather than by a mode.
    pub raytrace_fluid: bool,
    /// `FUZZY`: the exchanger takes any block, and Surface any solid one.
    pub fuzzy: bool,
    /// `CONNECTED_AREA`: the exchanger keeps only what touches the start.
    pub connected_area: bool,
    /// `AFFECT_TILES`: the exchanger may replace blocks with a block entity.
    pub affect_tiles: bool,
    /// `gadget_range`, the tool range (`GadgetNBT.getToolRange`), 1 by default.
    pub range: i32,
}

/// Where the player is, as a mode reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerAt {
    /// `player.blockPosition()`.
    pub block: [i32; 3],
    /// `player.getEyeY()`.
    pub eye_y: f64,
    /// `player.getY()`, the exact feet height, which Stairs compares against.
    pub y: f64,
    /// `player.getDirection()`, the horizontal direction they face.
    pub facing: Face,
}

/// The modes a gadget can be in, by their `BaseMode.getId()` path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    BuildToMe,
    VerticalWall,
    VerticalColumn,
    Surface,
    HorizontalWall,
    Stairs,
    HorizontalRow,
    Grid,
    Copy,
    Cut,
    Paste,
}

impl Mode {
    /// The mode's id path, as `getId()` gives it.
    pub fn path(self) -> &'static str {
        match self {
            Mode::BuildToMe => "build_to_me",
            Mode::VerticalWall => "vertical_wall",
            Mode::VerticalColumn => "vertical_column",
            Mode::Surface => "surface",
            Mode::HorizontalWall => "horizontal_wall",
            Mode::Stairs => "stairs",
            Mode::HorizontalRow => "horizontal_row",
            Mode::Grid => "grid",
            Mode::Copy => "copy",
            Mode::Cut => "cut",
            Mode::Paste => "paste",
        }
    }

    /// The namespaced id.
    pub fn id(self) -> String {
        format!("buildinggadgets2:{}", self.path())
    }

    /// The mode of an id, with or without the namespace.
    pub fn from_id(id: &str) -> Option<Mode> {
        let path = id.strip_prefix("buildinggadgets2:").unwrap_or(id);
        Mode::ALL.into_iter().find(|m| m.path() == path)
    }

    /// The translation key, as `BaseMode.i18n()`.
    pub fn i18n(self) -> String {
        format!("buildinggadgets2.modes.{}", self.path())
    }

    /// Every mode, in the order `GadgetModes` lists them per gadget.
    pub const ALL: [Mode; 11] = [
        Mode::BuildToMe,
        Mode::VerticalWall,
        Mode::VerticalColumn,
        Mode::Surface,
        Mode::HorizontalWall,
        Mode::Stairs,
        Mode::HorizontalRow,
        Mode::Grid,
        Mode::Copy,
        Mode::Cut,
        Mode::Paste,
    ];

    /// The Building Gadget's modes, in `GadgetModes`' order.
    pub const BUILDING: [Mode; 8] = [
        Mode::BuildToMe,
        Mode::VerticalWall,
        Mode::VerticalColumn,
        Mode::Surface,
        Mode::HorizontalWall,
        Mode::Stairs,
        Mode::HorizontalRow,
        Mode::Grid,
    ];

    /// The Exchanger's modes.
    pub const EXCHANGING: [Mode; 4] = [
        Mode::Surface,
        Mode::VerticalColumn,
        Mode::HorizontalRow,
        Mode::Grid,
    ];
}

/// Everything a mode needs besides the world: the click and the gadget.
pub struct Ctx<'a> {
    pub blocks: &'a dyn Blocks,
    /// The Exchanger rather than the Building Gadget (`BaseMode.isExchanging`).
    pub exchanging: bool,
    pub settings: Settings,
    pub player: PlayerAt,
    pub hit_side: Face,
    /// The block that was hit, which every offset is relative to.
    pub start: [i32; 3],
    /// The state the gadget places (`GadgetNBT.getGadgetBlockState`).
    pub state: &'a str,
}

impl Ctx<'_> {
    /// `BaseMode.isPosValid`, on an absolute position.
    ///
    /// The Java checks, in order:
    ///
    /// 1. `isExchangingValid` (below), for the Exchanger only;
    /// 2. `y >= level.getMaxY() || y < level.getMinY()`;
    /// 3. `blockState.canSurvive(level, pos)` — **not ported**: Horizon has no
    ///    helper that asks a state whether it can stay at a position, so a
    ///    build of blocks that need support is not filtered and those blocks
    ///    drop when placed. Nothing a full block does is affected;
    /// 4. `level.mayInteract(player, pos)`;
    /// 5. for the Building Gadget only, the block there `canBeReplaced()`.
    pub fn is_pos_valid(&self, pos: [i32; 3]) -> bool {
        if !self.is_exchanging_valid(pos) {
            return false;
        }
        let (min_y, height) = self.blocks.height();
        if pos[1] >= min_y + height || pos[1] < min_y {
            return false;
        }
        if !self.blocks.may_place(pos) {
            return false;
        }
        if !self.exchanging && !self.blocks.is_replaceable(pos) {
            return false;
        }
        true
    }

    /// `BaseMode.isExchangingValid`: what the Exchanger may replace.
    ///
    /// True for everything when the gadget is not the Exchanger, as in Java.
    fn is_exchanging_valid(&self, pos: [i32; 3]) -> bool {
        if !self.exchanging {
            return true;
        }
        let Some(old) = self.blocks.state(pos) else {
            return false;
        };
        let name = old.split('[').next().unwrap_or(&old);
        if self.blocks.is_air(name) {
            return false;
        }
        // `GadgetUtils.isValidBlockState`: the port's own render block is
        // never exchanged, and neither is a block the gadget could not place.
        if name == "buildinggadgets2:render_block" {
            return false;
        }
        if self.blocks.has_block_entity(pos) && !self.settings.affect_tiles {
            return false;
        }
        if self.settings.fuzzy {
            // Fuzzy takes any block but the one it would place.
            old != self.state
        } else {
            // Otherwise only blocks like the one being looked at.
            self.blocks.state(self.start).is_some_and(|look| look == old)
        }
    }
}

/// `BaseMode.collect` without an anchor: the offsets from the hit block a
/// build would fill.
///
/// The anchor branch (`GadgetNBT.getAnchorList` non-empty) is handled by the
/// caller, since it replaces the mode entirely.
pub fn collect(mode: Mode, ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    match mode {
        Mode::BuildToMe => build_to_me(ctx),
        Mode::VerticalWall => vertical_wall(ctx),
        Mode::VerticalColumn => vertical_column(ctx),
        Mode::Surface => surface(ctx),
        Mode::HorizontalWall => horizontal_wall(ctx),
        Mode::Stairs => stairs(ctx),
        Mode::HorizontalRow => horizontal_row(ctx),
        Mode::Grid => grid(ctx),
        // Copy, Cut and Paste do not collect from the world: they work on a
        // stored snapshot, which is M6.
        Mode::Copy | Mode::Cut | Mode::Paste => Vec::new(),
    }
}

/// `pos - start`.
fn rel(pos: [i32; 3], start: [i32; 3]) -> [i32; 3] {
    [pos[0] - start[0], pos[1] - start[1], pos[2] - start[2]]
}

/// `BuildToMe.collectWorld`.
///
/// Two details that are easy to lose and are kept here: the loop is
/// `i < difference`, so the block at the player's own coordinate is **not**
/// placed, and `(int) yPos` truncates towards zero, which is not
/// `getBlockY()`'s floor — below y 0 the two differ.
fn build_to_me(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let axis = ctx.hit_side.axis();
    let start_coord = axis.choose(ctx.start[0], ctx.start[1], ctx.start[2]);
    let player_y = if ctx.settings.place_on_top {
        ctx.player.eye_y as i32 // Java's `(int)`: towards zero.
    } else {
        ctx.player.block[1]
    };
    let player_coord = axis.choose(ctx.player.block[0], player_y, ctx.player.block[2]);

    let difference = (start_coord - player_coord).abs().clamp(0, 32);
    let mut out = Vec::new();
    for i in 1..difference {
        if ctx.is_pos_valid(ctx.hit_side.relative(ctx.start, i)) {
            out.push(ctx.hit_side.relative([0, 0, 0], i));
        }
    }
    out
}

/// `VerticalWall.collectWorld`: a wall standing up from the hit block.
fn vertical_wall(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let range = ctx.settings.range;
    let size = range / 2;
    let vertical = ctx.hit_side.axis() == Axis::Y;

    // Looking up or down, the wall faces the way the player does.
    let side = if vertical {
        ctx.player.facing.opposite().axis()
    } else {
        ctx.hit_side.axis()
    };

    let (start_y, end_y) = if vertical {
        (
            ctx.start[1] + 1,
            ctx.start[1] + (range - 1) * if ctx.hit_side == Face::Down { -1 } else { 1 } + 1,
        )
    } else {
        (ctx.start[1] - size, ctx.start[1] + size)
    };

    let spread_x = if side == Axis::Z { size } else { 0 };
    let spread_z = if side == Axis::X { size } else { 0 };
    let box_ = Box3 {
        min: [ctx.start[0] - spread_x, start_y.min(end_y), ctx.start[2] - spread_z],
        max: [ctx.start[0] + spread_x, start_y.max(end_y), ctx.start[2] + spread_z],
    };
    box_.positions()
        .filter(|p| ctx.is_pos_valid(*p))
        .map(|p| rel(p, ctx.start))
        .collect()
}

/// `VerticalColumn.collectWorld`: a column, which the Exchanger reads
/// differently on a horizontal face.
fn vertical_column(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let range = ctx.settings.range;
    let half = range / 2;
    let mut out = Vec::new();

    if ctx.hit_side.axis() == Axis::Y {
        if ctx.exchanging {
            // The Exchanger runs along the way the player faces instead.
            let facing = ctx.player.facing;
            for i in -half..=half {
                if ctx.is_pos_valid(facing.relative(ctx.start, i)) {
                    out.push(facing.relative([0, 0, 0], i));
                }
            }
        } else {
            for i in 1..range + 1 {
                if ctx.is_pos_valid(ctx.hit_side.relative(ctx.start, i)) {
                    out.push(ctx.hit_side.relative([0, 0, 0], i));
                }
            }
        }
    } else {
        for i in -half..=half {
            if ctx.is_pos_valid(Face::Up.relative(ctx.start, i)) {
                out.push(Face::Up.relative([0, 0, 0], i));
            }
        }
    }

    if ctx.exchanging && ctx.settings.connected_area {
        return remove_unconnected(ctx, [0, 0, 0], out);
    }
    out
}

/// `Surface.collectWorld`: the square face around the hit block.
fn surface(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let bound = ctx.settings.range / 2;
    let looking_at = ctx.blocks.state(ctx.start);
    let start_at = if ctx.exchanging {
        ctx.start
    } else {
        ctx.hit_side.relative(ctx.start, 1)
    };
    let out: Vec<[i32; 3]> = square_area(start_at, ctx.hit_side, bound)
        .positions()
        .filter(|p| ctx.is_pos_valid(*p) && surface_backed(ctx, *p, looking_at.as_deref()))
        .map(|p| rel(p, ctx.start))
        .collect();

    if ctx.exchanging && ctx.settings.connected_area {
        return remove_unconnected(ctx, rel(start_at, ctx.start), out);
    }
    out
}

/// `Surface.isPosValidCustom`: the block behind each one must match.
fn surface_backed(ctx: &Ctx<'_>, pos: [i32; 3], looking_at: Option<&str>) -> bool {
    if ctx.exchanging {
        return true; // handled by isExchangingValid
    }
    let behind = ctx.hit_side.opposite().relative(pos, 1);
    let Some(below) = ctx.blocks.state(behind) else {
        return false;
    };
    let name = below.split('[').next().unwrap_or(&below);
    if ctx.settings.fuzzy {
        !ctx.blocks.is_air(name)
    } else {
        looking_at == Some(below.as_str())
    }
}

/// `HorizontalWall.collectWorld`: a floor or ceiling, or a slab out of a wall.
fn horizontal_wall(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let range = ctx.settings.range;
    let half = range / 2;
    let mut out = Vec::new();

    if ctx.hit_side.axis() == Axis::Y {
        let lift = i32::from(ctx.settings.place_on_top);
        for i in -half..=half {
            for j in -half..=half {
                // Note the minus on i, as in Java.
                let coord = [ctx.start[0] - i, ctx.start[1] + lift, ctx.start[2] + j];
                if ctx.is_pos_valid(coord) {
                    out.push(rel(coord, ctx.start));
                }
            }
        }
        return out;
    }

    let axis = ctx.hit_side.axis();
    for i in 1..range + 1 {
        for j in -half..=half {
            let value = invert_on_face(ctx.hit_side, i);
            // Java gates the whole row on the position along the axis first.
            if !ctx.is_pos_valid(ctx.hit_side.relative(ctx.start, i)) {
                continue;
            }
            let coord = if axis == Axis::X {
                [ctx.start[0] + value, ctx.start[1], ctx.start[2] + j]
            } else {
                [ctx.start[0] + j, ctx.start[1], ctx.start[2] + value]
            };
            if ctx.is_pos_valid(coord) {
                out.push(rel(coord, ctx.start));
            }
        }
    }
    out
}

/// `MagicHelpers.invertOnFace`.
fn invert_on_face(face: Face, value: i32) -> i32 {
    value * if matches!(face, Face::North | Face::Down | Face::West) { -1 } else { 1 }
}

/// `Stairs.collectWorld`: a staircase climbing away from the player.
fn stairs(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let place_on_top = ctx.settings.place_on_top;
    let side = if ctx.hit_side.axis() == Axis::Y {
        ctx.player.facing.opposite()
    } else {
        ctx.hit_side
    };
    let range = ctx.settings.range;
    let mut out = Vec::new();

    for i in 0..range {
        let shift = (i + 1)
            * if matches!(ctx.hit_side, Face::East | Face::South) { 1 } else { -1 };

        // Looking at a block below your feet, on a horizontal face.
        if f64::from(ctx.start[1]) < ctx.player.y && ctx.hit_side.is_horizontal() {
            let x_axis = ctx.hit_side.axis() == Axis::X;
            let z_axis = ctx.hit_side.axis() == Axis::Z;
            let coord = [
                ctx.start[0] + if x_axis { shift } else { 0 },
                ctx.start[1] + if place_on_top { -i } else { -(i + 1) },
                ctx.start[2] + if z_axis { shift } else { 0 },
            ];
            if ctx.is_pos_valid(coord) {
                out.push(rel(coord, ctx.start));
            }
            continue;
        }

        let mut shift = i * if matches!(side, Face::East | Face::South) { -1 } else { 1 };
        if f64::from(ctx.start[1]) < ctx.player.y - 2.0 {
            shift = -shift;
        }
        let rise = if f64::from(ctx.start[1]) > ctx.player.y + 1.0 { -i } else { i }
            + i32::from(place_on_top);
        let coord = [
            ctx.start[0] + if side.axis() == Axis::X { shift } else { 0 },
            ctx.start[1] + rise,
            ctx.start[2] + if side.axis() == Axis::Z { shift } else { 0 },
        ];
        if ctx.is_pos_valid(coord) {
            out.push(rel(coord, ctx.start));
        }
    }
    out
}

/// `HorizontalRow.collectWorld`: a row away from the player, or across for
/// the Exchanger.
fn horizontal_row(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let range = ctx.settings.range;
    let start_at = if ctx.settings.place_on_top {
        Face::Up.relative(ctx.start, 1)
    } else {
        ctx.start
    };
    let mut side = if ctx.hit_side.axis() == Axis::Y {
        ctx.player.facing
    } else {
        ctx.hit_side.opposite()
    };
    let mut out = Vec::new();

    if !ctx.exchanging {
        for i in 0..range {
            let pos = side.relative(start_at, i);
            if ctx.is_pos_valid(pos) {
                out.push(rel(pos, ctx.start));
            }
        }
    } else {
        side = side.clockwise();
        let half = range / 2;
        for i in -half..=half {
            let pos = side.relative(start_at, i);
            if ctx.is_pos_valid(pos) {
                out.push(rel(pos, ctx.start));
            }
        }
    }

    if ctx.exchanging && ctx.settings.connected_area {
        return remove_unconnected(ctx, rel(start_at, ctx.start), out);
    }
    out
}

/// `Grid.collectWorld`: a lattice of single blocks.
///
/// The spacing is `((range - 2) % 6) + 2` and the extent `range * 7 / 5`, both
/// with Java's truncating division and remainder, which Rust's `/` and `%`
/// match for these values.
fn grid(ctx: &Ctx<'_>) -> Vec<[i32; 3]> {
    let range = ctx.settings.range + 1;
    let start_at = if ctx.settings.place_on_top {
        Face::Up.relative(ctx.start, 1)
    } else {
        ctx.start
    };
    let spacing = ((range - 2) % 6) + 2;
    let extent = range * 7 / 5;
    let mut out = Vec::new();

    for x in (range * -7 / 5)..=extent {
        for z in (range * -7 / 5)..=extent {
            if spacing == 0 || x % spacing != 0 || z % spacing != 0 {
                continue;
            }
            let coord = [start_at[0] + x, start_at[1], start_at[2] + z];
            if ctx.is_pos_valid(coord) {
                out.push(rel(coord, ctx.start));
            }
        }
    }
    out
}

/// `BaseMode.removeUnConnected`: keep only what the start reaches.
///
/// A breadth-first walk from `start_at` through the collected positions, in
/// the four directions across the hit face. Java returns a `HashSet`, so its
/// order is arbitrary; this answers them sorted, which only matters when
/// materials run short and some positions are dropped.
fn remove_unconnected(
    ctx: &Ctx<'_>,
    start_at: [i32; 3],
    coordinates: Vec<[i32; 3]>,
) -> Vec<[i32; 3]> {
    if coordinates.is_empty() {
        return coordinates;
    }
    let known: HashSet<[i32; 3]> = coordinates.iter().copied().collect();
    let mut visited: HashSet<[i32; 3]> = HashSet::new();
    let mut queue: VecDeque<[i32; 3]> = VecDeque::new();
    queue.push_back(start_at);

    let across: Vec<Face> = Face::ALL
        .into_iter()
        .filter(|f| f.axis() != ctx.hit_side.axis())
        .collect();

    while let Some(current) = queue.pop_front() {
        if visited.contains(&current) || !known.contains(&current) {
            continue;
        }
        visited.insert(current);
        for face in &across {
            let next = face.relative(current, 1);
            if known.contains(&next) {
                queue.push_back(next);
            }
        }
    }

    let mut out: Vec<[i32; 3]> = visited.into_iter().collect();
    out.sort_unstable();
    out
}

/// A box of block positions, as `BlockPos.betweenClosedStream(AABB)` walks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Box3 {
    min: [i32; 3],
    max: [i32; 3],
}

impl Box3 {
    fn positions(self) -> impl Iterator<Item = [i32; 3]> {
        let Box3 { min, max } = self;
        (min[0]..=max[0]).flat_map(move |x| {
            (min[1]..=max[1])
                .flat_map(move |y| (min[2]..=max[2]).map(move |z| [x, y, z]))
        })
    }
}

/// `GadgetUtils.getSquareArea`: the square on the plane of `face`.
fn square_area(pos: [i32; 3], face: Face, range: i32) -> Box3 {
    match face.axis() {
        Axis::Y => Box3 {
            min: [pos[0] - range, pos[1], pos[2] - range],
            max: [pos[0] + range, pos[1], pos[2] + range],
        },
        Axis::Z => Box3 {
            min: [pos[0] - range, pos[1] - range, pos[2]],
            max: [pos[0] + range, pos[1] + range, pos[2]],
        },
        Axis::X => Box3 {
            min: [pos[0], pos[1] - range, pos[2] - range],
            max: [pos[0], pos[1] + range, pos[2] + range],
        },
    }
}

#[cfg(test)]
#[path = "modes_tests.rs"]
mod tests;
