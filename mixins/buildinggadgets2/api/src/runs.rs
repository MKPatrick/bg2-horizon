//! Turning the positions a mode collected into one bulk edit.
//!
//! `Host::edit_blocks` takes a `NativeBlockEdit`: a palette of block states
//! and `runs`, each a vertical column `height` blocks tall from `(x, y, z)`
//! set to one palette entry. One run per block would work, but a Build to Me
//! straight up is naturally one run, so neighbouring blocks in the same
//! column are grouped. That matters: the edit writes each section once
//! however many of its blocks change, and fewer runs means less to scan.
//!
//! The grouping is pure, so it is tested here without a game.

/// A run of blocks: `height` blocks up from `(x, y, z)`, all the same state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub height: u32,
}

/// Group positions into vertical runs, fewest first by column.
///
/// Every position gets into exactly one run, duplicates are dropped, and the
/// result is in a stable order (by x, then z, then y) so a test can read it.
pub fn runs(positions: &[[i32; 3]]) -> Vec<Run> {
    let mut sorted: Vec<[i32; 3]> = positions.to_vec();
    // By column, then up it: the order the runs come out in.
    sorted.sort_unstable_by_key(|p| (p[0], p[2], p[1]));
    sorted.dedup();

    let mut out: Vec<Run> = Vec::new();
    for pos in sorted {
        match out.last_mut() {
            // Carry on the run above when this block sits on top of it.
            Some(run)
                if run.x == pos[0]
                    && run.z == pos[2]
                    && i64::from(run.y) + i64::from(run.height) == i64::from(pos[1]) =>
            {
                run.height += 1;
            }
            _ => out.push(Run {
                x: pos[0],
                y: pos[1],
                z: pos[2],
                height: 1,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_block_is_one_run() {
        assert_eq!(
            runs(&[[1, 2, 3]]),
            vec![Run {
                x: 1,
                y: 2,
                z: 3,
                height: 1
            }]
        );
    }

    #[test]
    fn a_vertical_line_is_one_run() {
        let positions: Vec<[i32; 3]> = (0..9).map(|y| [0, y, 0]).collect();
        assert_eq!(
            runs(&positions),
            vec![Run {
                x: 0,
                y: 0,
                z: 0,
                height: 9
            }]
        );
    }

    #[test]
    fn a_horizontal_line_is_one_run_per_block() {
        let positions: Vec<[i32; 3]> = (0..4).map(|x| [x, 0, 0]).collect();
        let out = runs(&positions);
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|r| r.height == 1));
    }

    #[test]
    fn a_gap_in_a_column_splits_the_run() {
        let out = runs(&[[0, 0, 0], [0, 1, 0], [0, 3, 0], [0, 4, 0]]);
        assert_eq!(
            out,
            vec![
                Run {
                    x: 0,
                    y: 0,
                    z: 0,
                    height: 2
                },
                Run {
                    x: 0,
                    y: 3,
                    z: 0,
                    height: 2
                },
            ]
        );
    }

    #[test]
    fn unsorted_input_still_groups() {
        let out = runs(&[[0, 2, 0], [0, 0, 0], [0, 1, 0]]);
        assert_eq!(
            out,
            vec![Run {
                x: 0,
                y: 0,
                z: 0,
                height: 3
            }]
        );
    }

    #[test]
    fn duplicates_are_dropped() {
        let out = runs(&[[0, 0, 0], [0, 0, 0], [0, 1, 0]]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].height, 2);
    }

    #[test]
    fn columns_stay_apart() {
        let out = runs(&[[0, 0, 0], [0, 1, 0], [1, 0, 0], [0, 0, 1]]);
        assert_eq!(out.len(), 3);
        // Ordered by x, then z.
        assert_eq!((out[0].x, out[0].z, out[0].height), (0, 0, 2));
        assert_eq!((out[1].x, out[1].z, out[1].height), (0, 1, 1));
        assert_eq!((out[2].x, out[2].z, out[2].height), (1, 0, 1));
    }

    #[test]
    fn negative_coordinates_group_too() {
        let positions: Vec<[i32; 3]> = (-5..-1).map(|y| [-3, y, -7]).collect();
        assert_eq!(
            runs(&positions),
            vec![Run {
                x: -3,
                y: -5,
                z: -7,
                height: 4
            }]
        );
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(runs(&[]).is_empty());
    }

    #[test]
    fn every_position_lands_in_exactly_one_run() {
        let positions: Vec<[i32; 3]> = (0..5)
            .flat_map(|x| (0..5).flat_map(move |y| (0..5).map(move |z| [x, y, z])))
            .collect();
        let out = runs(&positions);
        let covered: u32 = out.iter().map(|r| r.height).sum();
        assert_eq!(covered as usize, positions.len());
    }
}
