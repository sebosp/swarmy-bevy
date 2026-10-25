//! The map cells of a ramp, ported from the ramp algorithm of sc2-map-analyzer
//! (`SC2Map::processRamp` in pathing.cpp).
//!
//! A ramp is defined by its direction (going up the ramp) and the `leftLo` and `rightLo` points,
//! the corners of the low edge of the ramp. The cells are walked from the left corner to the right
//! one, marking the cells across the ramp at each step: the ramp cells are pathable and not
//! buildable, except the top row of the straight ramps, which stays buildable (where the walls
//! are placed), and the cells at the sides of the low edge are not pathable.

use super::RampDirection;
use bevy::math::{IVec2, Vec2};

/// The ramp algorithm stops after these many steps, a ramp is never this wide.
const MAX_STEPS: usize = 1000;

/// What a cell of the ramp is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampCellKind {
    /// Pathable and not buildable.
    Ramp,
    /// The top row of a straight ramp, pathable and buildable.
    Top,
    /// The sides of the ramp, not pathable.
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampCell {
    /// The cell, its center is at `(x + 0.5, y + 0.5)` in map coordinates.
    pub cell: IVec2,
    pub kind: RampCellKind,
}

/// The vectors of the ramp algorithm: `u` goes up the ramp, `v` along it from `leftLo` to
/// `rightLo`, `w` diagonally between them for the diagonal ramps, and `s` from the ramp point
/// (a corner of 4 cells) to the first cell.
struct RampVectors {
    u: IVec2,
    v: IVec2,
    w: IVec2,
    s: IVec2,
}

impl RampVectors {
    fn new(dir: &RampDirection) -> Self {
        let (u, v, w, s) = match dir {
            RampDirection::South => ((0, -1), (-1, 0), (0, 0), (-1, -1)),
            RampDirection::North => ((0, 1), (1, 0), (0, 0), (0, 0)),
            RampDirection::West => ((-1, 0), (0, 1), (0, 0), (-1, 0)),
            RampDirection::East => ((1, 0), (0, -1), (0, 0), (0, -1)),
            RampDirection::SouthWest => ((-1, -1), (-1, 1), (-1, 0), (-1, -1)),
            RampDirection::SouthEast => ((1, -1), (-1, -1), (0, -1), (0, -1)),
            RampDirection::NorthWest => ((-1, 1), (1, 1), (0, 1), (-1, 0)),
            RampDirection::NorthEast => ((1, 1), (1, -1), (1, 0), (0, 0)),
        };
        let vec = |(x, y): (i32, i32)| IVec2::new(x, y);
        Self {
            u: vec(u),
            v: vec(v),
            w: vec(w),
            s: vec(s),
        }
    }
}

/// Collects the cells, a cell marked twice keeps the first kind except that an edge never
/// replaces a ramp cell.
#[derive(Default)]
struct RampCells(Vec<RampCell>);

impl RampCells {
    fn mark(&mut self, cell: IVec2, kind: RampCellKind) {
        match self.0.iter_mut().find(|ramp_cell| ramp_cell.cell == cell) {
            Some(existing) => {
                if existing.kind == RampCellKind::Edge && kind != RampCellKind::Edge {
                    existing.kind = kind;
                }
            }
            None => self.0.push(RampCell { cell, kind }),
        }
    }
}

/// Snaps the ramp point to the integer corner it is on, i.e. 49.9999 is 50.
fn snap(point: Vec2) -> IVec2 {
    IVec2::new(point.x.round() as i32, point.y.round() as i32)
}

/// The cells of the ramp, None if the walk along the ramp never reaches `right_lo`.
pub fn ramp_cells(dir: &RampDirection, left_lo: Vec2, right_lo: Vec2) -> Option<Vec<RampCell>> {
    let RampVectors { u, v, w, s } = RampVectors::new(dir);
    let mut p0 = snap(left_lo) + s;
    let p2 = snap(right_lo) + s;
    let mut cells = RampCells::default();
    let mut steps = 0;
    use RampCellKind::*;
    let is_straight = w == IVec2::ZERO;
    if is_straight {
        cells.mark(p0 - v, Edge);
        cells.mark(p0 - v - u, Edge);
        while p0 != p2 {
            steps += 1;
            if steps > MAX_STEPS {
                return None;
            }
            cells.mark(p0, Ramp);
            cells.mark(p0 + u, Top);
            cells.mark(p0 - u, Ramp);
            cells.mark(p0 - 2 * u, Ramp);
            p0 += v;
        }
        cells.mark(p0, Edge);
        cells.mark(p0 - u, Edge);
        return Some(cells.0);
    }

    // The diagonal ramps alternate between two shapes, `w` moves to the next diagonal.
    let mark_across = |cells: &mut RampCells, p0: IVec2, offsets: &[i32]| {
        for offset in offsets {
            cells.mark(p0 + *offset * u, Ramp);
        }
    };
    cells.mark(p0 - w, Edge);
    cells.mark(p0 - w - u, Edge);
    cells.mark(p0 - 2 * w, Edge);
    cells.mark(p0 - 2 * w + u, Edge);
    mark_across(&mut cells, p0, &[0, 1, -1, -2]);
    p0 += w - u;
    mark_across(&mut cells, p0, &[0, 1, 2, -1, -2]);
    p0 += w;
    while p0 != p2 {
        steps += 1;
        if steps > MAX_STEPS {
            return None;
        }
        mark_across(&mut cells, p0, &[0, 1, -1, -2]);
        p0 += w - u;
        mark_across(&mut cells, p0, &[0, 1, 2, -1, -2]);
        p0 += w;
    }
    mark_across(&mut cells, p0, &[0, 1, -1, -2]);
    p0 += w;
    let edge = p0 - u;
    cells.mark(edge, Edge);
    cells.mark(edge - u, Edge);
    cells.mark(edge - u + w, Edge);
    cells.mark(edge + w, Edge);
    Some(cells.0)
}

/// How far up the ramp the cell is, from 0 at the lowest cells to 1 at the highest ones.
pub fn ramp_progress(dir: &RampDirection, cells: &[RampCell]) -> Vec<f32> {
    let u = RampVectors::new(dir).u.as_vec2().normalize();
    let along: Vec<f32> = cells
        .iter()
        .map(|ramp_cell| (ramp_cell.cell.as_vec2() + Vec2::splat(0.5)).dot(u))
        .collect();
    let min = along.iter().copied().fold(f32::INFINITY, f32::min);
    let max = along.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    along
        .iter()
        .map(|value| {
            if max > min {
                (value - min) / (max - min)
            } else {
                0.
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(cells: &[RampCell], kind: RampCellKind) -> usize {
        cells.iter().filter(|cell| cell.kind == kind).count()
    }

    #[test]
    fn it_fills_a_straight_ramp() {
        // A ramp going up to the south, the low edge from x 81 to 69 at y 42.
        let cells = ramp_cells(
            &RampDirection::South,
            Vec2::new(81., 42.),
            Vec2::new(69., 42.),
        )
        .unwrap();
        // 12 steps of 4 cells across, the top row is buildable.
        assert_eq!(count(&cells, RampCellKind::Ramp), 36);
        assert_eq!(count(&cells, RampCellKind::Top), 12);
        assert_eq!(count(&cells, RampCellKind::Edge), 4);
        // The first cell is the cell below and left of the ramp point.
        assert!(cells.contains(&RampCell {
            cell: IVec2::new(80, 41),
            kind: RampCellKind::Ramp
        }));
        // The top row is up the ramp, to the south.
        assert!(cells.contains(&RampCell {
            cell: IVec2::new(80, 40),
            kind: RampCellKind::Top
        }));
    }

    #[test]
    fn it_fills_a_diagonal_ramp() {
        // A ramp going up to the north west, from 52,20 to 60,28.
        let cells = ramp_cells(
            &RampDirection::NorthWest,
            Vec2::new(52., 20.),
            Vec2::new(60., 28.),
        )
        .unwrap();
        assert!(count(&cells, RampCellKind::Ramp) > 30);
        assert_eq!(count(&cells, RampCellKind::Top), 0);
        let progress = ramp_progress(&RampDirection::NorthWest, &cells);
        // Going up to the north west, the cells to the north west are higher.
        let index = |x: i32, y: i32| {
            cells
                .iter()
                .position(|cell| cell.cell == IVec2::new(x, y))
                .unwrap()
        };
        assert!(progress[index(51, 21)] > progress[index(53, 19)]);
    }

    #[test]
    fn it_gives_up_on_unreachable_corners() {
        // The right corner is not along the ramp.
        assert!(
            ramp_cells(
                &RampDirection::South,
                Vec2::new(81., 42.),
                Vec2::new(69., 50.)
            )
            .is_none()
        );
    }
}
