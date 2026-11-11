//! What the port needs to know about the world, behind a trait.
//!
//! The raycast and the modes are the part of this port that has to match the
//! Java mod block for block, and they are also the part that is cheapest to
//! check — if they do not depend on a running game. So they are written
//! against [`Blocks`], which the server implements over `Host`
//! (`crate::host_world`) and the tests implement over a hand-written map
//! (`Fake`).
//!
//! Positions here are absolute block positions, as the Java mod's
//! `BlockPos` is.

/// A direction, as the Java mod's `Direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

/// The axis a face lies on, as `Direction.Axis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Face {
    /// The six faces in vanilla's `Direction` order, which the Java mod's
    /// `Direction.ordinal()` relies on for the bind direction.
    pub const ALL: [Face; 6] = [
        Face::Down,
        Face::Up,
        Face::North,
        Face::South,
        Face::West,
        Face::East,
    ];

    /// `Direction.getAxis`.
    pub fn axis(self) -> Axis {
        match self {
            Face::Down | Face::Up => Axis::Y,
            Face::North | Face::South => Axis::Z,
            Face::West | Face::East => Axis::X,
        }
    }

    /// The unit step along this face, as `Direction.getNormal`.
    pub fn step(self) -> [i32; 3] {
        match self {
            Face::Down => [0, -1, 0],
            Face::Up => [0, 1, 0],
            Face::North => [0, 0, -1],
            Face::South => [0, 0, 1],
            Face::West => [-1, 0, 0],
            Face::East => [1, 0, 0],
        }
    }

    /// `pos.relative(this, n)`.
    pub fn relative(self, pos: [i32; 3], n: i32) -> [i32; 3] {
        let s = self.step();
        [pos[0] + s[0] * n, pos[1] + s[1] * n, pos[2] + s[2] * n]
    }

    /// The face pointing the other way.
    pub fn opposite(self) -> Face {
        match self {
            Face::Down => Face::Up,
            Face::Up => Face::Down,
            Face::North => Face::South,
            Face::South => Face::North,
            Face::West => Face::East,
            Face::East => Face::West,
        }
    }

    /// `Direction.ordinal()`, which the Java mod stores for the bind direction.
    pub fn ordinal(self) -> usize {
        Face::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }

    /// The face from a unit vector, as the `face` a use-on hook carries.
    pub fn from_step(step: [i32; 3]) -> Option<Face> {
        Face::ALL.into_iter().find(|f| f.step() == step)
    }

    /// `Direction.fromYRot`: the horizontal direction a yaw faces, which is
    /// what `player.getDirection()` answers.
    ///
    /// Vanilla is `BY_2D[floor(yRot / 90 + 0.5) & 3]` over
    /// `{SOUTH, WEST, NORTH, EAST}`.
    pub fn from_yaw(yaw: f32) -> Face {
        let i = ((f64::from(yaw) / 90.0 + 0.5).floor() as i64) & 3;
        match i {
            0 => Face::South,
            1 => Face::West,
            2 => Face::North,
            _ => Face::East,
        }
    }

    /// `Direction.getClockWise()` for a horizontal face; a vertical one is
    /// answered unchanged, as vanilla throws there and the modes never ask.
    pub fn clockwise(self) -> Face {
        match self {
            Face::North => Face::East,
            Face::East => Face::South,
            Face::South => Face::West,
            Face::West => Face::North,
            other => other,
        }
    }

    /// Whether the face lies in the horizontal plane (`Axis.isHorizontal`).
    pub fn is_horizontal(self) -> bool {
        !matches!(self, Face::Up | Face::Down)
    }
}

impl Axis {
    /// `Direction.Axis.choose(x, y, z)`.
    pub fn choose<T>(self, x: T, y: T, z: T) -> T {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }
}

/// The world as the raycast and the modes read it.
pub trait Blocks {
    /// The block's name at `pos` (`minecraft:stone`), or `None` where the
    /// chunk is not loaded. Air is a name like any other.
    fn block(&self, pos: [i32; 3]) -> Option<String>;

    /// Whether a block of that name is air (`minecraft:air`, `cave_air`,
    /// `void_air`), as `BlockState.isAir`.
    fn is_air(&self, block: &str) -> bool {
        matches!(
            block,
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    }

    /// Whether a block of that name is a fluid the Java mod's raytrace would
    /// stop at when `raytrace_fluid` is on.
    fn is_fluid(&self, block: &str) -> bool {
        matches!(block, "minecraft:water" | "minecraft:lava")
    }

    /// The whole block state at `pos` with its properties
    /// (`minecraft:oak_stairs[facing=east,...]`), as `/setblock` writes it.
    /// The exchanger compares states, not names.
    fn state(&self, pos: [i32; 3]) -> Option<String>;

    /// Whether the block at `pos` has a block entity (a chest, a sign), which
    /// the `affect_tiles` setting decides about.
    fn has_block_entity(&self, pos: [i32; 3]) -> bool;

    /// Whether the block at `pos` can be built over, as
    /// `BlockState.canBeReplaced`. The server answers it from the
    /// `minecraft:replaceable` block tag.
    fn is_replaceable(&self, pos: [i32; 3]) -> bool;

    /// The block's collision boxes at `pos`, block-local
    /// `[min_x, min_y, min_z, max_x, max_y, max_z]`. A block with none (a
    /// flower, a torch) answers an empty list.
    fn collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]>;

    /// The level's lowest `y` and its height, as `Level.getMinY` and the
    /// height above it.
    fn height(&self) -> (i32, i32);

    /// Whether the player may edit the block at `pos`, as
    /// `Level.mayInteract` — spawn protection and region-protection mods.
    fn may_place(&self, pos: [i32; 3]) -> bool;
}

/// A world written out by hand, for the tests.
#[cfg(test)]
#[derive(Default)]
pub struct Fake {
    /// The blocks that are not air, by position.
    pub blocks: std::collections::HashMap<[i32; 3], String>,
    /// What `is_replaceable` answers for a block name; air is always true.
    pub replaceable: std::collections::HashSet<String>,
    /// Positions whose chunk is not loaded.
    pub unloaded: std::collections::HashSet<[i32; 3]>,
    /// Positions `may_place` refuses.
    pub protected: std::collections::HashSet<[i32; 3]>,
    /// Block names with no collision box (a flower, a torch).
    pub no_collision: std::collections::HashSet<String>,
    /// Positions holding a block entity.
    pub block_entities: std::collections::HashSet<[i32; 3]>,
    pub min_y: i32,
    pub world_height: i32,
}

#[cfg(test)]
impl Fake {
    /// A world of air from y 0 to 16.
    pub fn new() -> Self {
        Self {
            min_y: 0,
            world_height: 16,
            replaceable: ["minecraft:air".to_owned()].into_iter().collect(),
            ..Default::default()
        }
    }

    /// Put a block at `pos`.
    pub fn with(mut self, pos: [i32; 3], block: &str) -> Self {
        self.blocks.insert(pos, block.to_owned());
        self
    }

    /// Set the level's lowest y and height.
    pub fn bounds(mut self, min_y: i32, height: i32) -> Self {
        self.min_y = min_y;
        self.world_height = height;
        self
    }
}

#[cfg(test)]
impl Blocks for Fake {
    fn block(&self, pos: [i32; 3]) -> Option<String> {
        if self.unloaded.contains(&pos) {
            return None;
        }
        Some(
            self.blocks
                .get(&pos)
                .cloned()
                .unwrap_or_else(|| "minecraft:air".to_owned()),
        )
    }

    fn state(&self, pos: [i32; 3]) -> Option<String> {
        self.block(pos)
    }

    fn has_block_entity(&self, pos: [i32; 3]) -> bool {
        self.block_entities.contains(&pos)
    }

    fn is_replaceable(&self, pos: [i32; 3]) -> bool {
        match self.block(pos) {
            Some(b) => self.is_air(&b) || self.replaceable.contains(&b),
            None => false,
        }
    }

    fn collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        match self.block(pos) {
            Some(b) if self.is_air(&b) || self.no_collision.contains(&b) => Vec::new(),
            Some(_) => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
            None => Vec::new(),
        }
    }

    fn height(&self) -> (i32, i32) {
        (self.min_y, self.world_height)
    }

    fn may_place(&self, pos: [i32; 3]) -> bool {
        !self.protected.contains(&pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_keep_vanillas_order() {
        // The Java mod stores Direction.ordinal() for the bind direction, so
        // this order is part of the save format.
        assert_eq!(Face::Down.ordinal(), 0);
        assert_eq!(Face::Up.ordinal(), 1);
        assert_eq!(Face::North.ordinal(), 2);
        assert_eq!(Face::South.ordinal(), 3);
        assert_eq!(Face::West.ordinal(), 4);
        assert_eq!(Face::East.ordinal(), 5);
    }

    #[test]
    fn relative_steps_along_the_face() {
        assert_eq!(Face::Up.relative([1, 2, 3], 4), [1, 6, 3]);
        assert_eq!(Face::West.relative([1, 2, 3], 2), [-1, 2, 3]);
        assert_eq!(Face::North.relative([0, 0, 0], 3), [0, 0, -3]);
    }

    #[test]
    fn axis_choose_picks_the_component() {
        assert_eq!(Face::East.axis().choose(7, 8, 9), 7);
        assert_eq!(Face::Up.axis().choose(7, 8, 9), 8);
        assert_eq!(Face::South.axis().choose(7, 8, 9), 9);
    }

    #[test]
    fn a_face_round_trips_through_its_step() {
        for face in Face::ALL {
            assert_eq!(Face::from_step(face.step()), Some(face));
            assert_eq!(face.opposite().opposite(), face);
        }
    }
}
