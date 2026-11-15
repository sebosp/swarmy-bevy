use crate::BevySC2MapError;
use crate::MAP_SCALE_FACTOR;
use crate::swarmy_feathers::DisplayInfoOnClick;
use crate::swarmy_feathers::update_info_on_click;
use crate::t3_height_map::{T3HeightMapResource, terrain_top};
use bevy::color::palettes;
use bevy::log::tracing;
use bevy::prelude::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until;

pub mod ramp_cells;
use ramp_cells::RampCellKind;

/// A copy of the T3Terrain that impls Reflect, Resource.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct T3TerrainResource {
    pub version: u32,
    pub ramp_list: Vec<RampResource>,
}

#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub enum RampDirection {
    #[default]
    South,
    North,
    West,
    East,
    SouthWest,
    SouthEast,
    NorthWest,
    NorthEast,
}

impl TryFrom<u8> for RampDirection {
    type Error = BevySC2MapError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::South),
            1 => Ok(Self::North),
            2 => Ok(Self::West),
            3 => Ok(Self::East),
            4 => Ok(Self::SouthWest),
            5 => Ok(Self::SouthEast),
            6 => Ok(Self::NorthWest),
            7 => Ok(Self::NorthEast),
            _ => Err(BevySC2MapError::Other(
                "Unknown Ramp Direction.".to_string(),
            )),
        }
    }
}

/// A copy of the T3Terrain that impls Reflect, Resource.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct RampResource {
    pub dir: RampDirection,
    /// Looks like cell layer/height
    pub hi: u8,
    pub lo: u8,
    // "u(-1.000000e+00, 0.000000e+00) r(0.000000e+00, 1.000000e+00) c=(1.420000e+02, 4.400000e+01) w=2.000000e+00 h=2.000000e+00"
    // Looks SVG-ish, maybe u=up r=right c=center w=width h=height ?
    pub left_lo: Vec2,
    pub left_hi: Vec2,
    pub right_lo: Vec2,
    pub right_hi: Vec2,
    pub base: String,
    pub mid: String,
    pub cid: usize,
    pub left_lo_var: u32,
    pub left_hi_var: u32,
    pub right_lo_var: u32,
    pub right_hi_var: u32,
}

impl TryFrom<s2protocol::cache_handles::t3_terrain::T3Terrain> for T3TerrainResource {
    type Error = BevySC2MapError;
    fn try_from(
        input: s2protocol::cache_handles::t3_terrain::T3Terrain,
    ) -> Result<Self, Self::Error> {
        let ramp_list = match input.height_map.ramp_list {
            Some(val) => val
                .inner
                .into_iter()
                .filter_map(|x| x.try_into().ok())
                .collect(),
            None => vec![],
        };

        Ok(Self {
            version: input.version,
            ramp_list,
        })
    }
}

impl TryFrom<s2protocol::cache_handles::t3_terrain::Ramp> for RampResource {
    type Error = BevySC2MapError;
    fn try_from(input: s2protocol::cache_handles::t3_terrain::Ramp) -> Result<Self, Self::Error> {
        // The "hi" units seem to be relative to the "lo" absolute points in the cell grid.
        let left_lo = parse_ramp_corner(&input.left_lo)?;
        let mut left_hi = parse_ramp_corner(&input.left_hi)?;
        left_hi.center = left_lo.center - left_hi.center;

        let right_lo = parse_ramp_corner(&input.right_lo)?;
        let mut right_hi = parse_ramp_corner(&input.right_hi)?;
        right_hi.center = right_lo.center - right_hi.center;
        let res = Self {
            dir: input.dir.try_into()?,
            hi: input.hi,
            lo: input.lo,
            left_lo: left_lo.center,
            left_hi: left_hi.center,
            right_lo: right_lo.center,
            right_hi: right_hi.center,
            base: input.base.clone(),
            mid: input.mid.clone(),
            cid: input.cid,
            left_lo_var: input.left_lo_var,
            left_hi_var: input.left_hi_var,
            right_lo_var: input.right_lo_var,
            right_hi_var: input.right_hi_var,
        };
        tracing::debug!("try_from: input: {input:?}, output: {res:?}",);

        Ok(res)
    }
}

/// A corner of a ramp, its center and the direction up the ramp.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct RampCorner {
    /// The center of the ramp (TODO: verify)
    center: Vec2,
    /// The unit vector pointing up the ramp.
    up: Vec2,
    /// The width of the ramp (TODO: verify)
    width: f32,
    /// The height of the ramp (TODO: verify)
    height: f32,
    /// The unit vector pointing right dir of the ramp?
    right: Vec2,
}

/// The Ramp contains x,y inside c=(x,y)
/// u(0.000000e+00, -1.000000e+00) r(-1.000000e+00, 0.000000e+00) c=(4.800000e+01, 2.600000e+01) w=2.000000e+00 h=2.000000e+00
/// There are maybe 10 maybe 100 ramps per maps and it's only read once, maybe String is fine by now.
fn parse_ramp_corner(s: &str) -> Result<RampCorner, BevySC2MapError> {
    let (tail, _) = tag("u(")(s)?;
    let (tail, _ux) = take_until(", ")(tail)?;
    let (tail, _) = take_until("c=(")(tail)?;
    let (tail, _) = tag("c=(")(tail)?;
    let (tail, cx) = take_until(", ")(tail)?;
    let (tail, _) = tag(", ")(tail)?;
    let (tail, cy) = take_until(") ")(tail)?;
    let (tail, _) = tag(") w=")(tail)?;
    let (tail, width_str) = take_until(" ")(tail)?;
    let (tail, _) = tag(" h=")(tail)?;
    let height_str = tail.trim();
    let center = x_y_to_vec2(cx, cy)?;
    let height = height_str.parse()?;
    let width = width_str.parse()?;
    Ok(RampCorner {
        center,
        height,
        width,
        ..Default::default()
    })
}

fn x_y_to_vec2(x_str: &str, y_str: &str) -> Result<Vec2, BevySC2MapError> {
    Ok(Vec2::new(x_str.parse()?, y_str.parse()?))
}

/// Marks the ramp cells, to show or hide them from the View menu.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct RampComponent;

/// How high above the terrain the ramp surface is drawn, so that it is not hidden in the cells.
pub const RAMP_LIFT: f32 = 0.05;

/// The cells away from a ramp cell, down and up the ramp, where its low and high ground are read.
const GROUND_DISTANCE: i32 = 3;

/// The low and high levels of a ramp: the most common level of the terrain a few cells down and
/// up the ramp from its cells; the levels around a ramp may be of other plateaus.
fn ramp_levels(
    dir: &RampDirection,
    cells: &[ramp_cells::RampCell],
    height_map: &T3HeightMapResource,
    flat: bool,
) -> Option<(u8, u8)> {
    let u = ramp_cells::up_vector(dir);
    let mode = |offset: i32| {
        let mut counts = [0usize; 256];
        for cell in cells {
            let IVec2 { x, y } = cell.cell + u * offset;
            // The drawn heights of a replay are flat, else the levels: the drawn heights are
            // lowered under the ramps.
            let level = if flat {
                height_map.height_at(x, y)
            } else {
                height_map.level_at(x, y)
            };
            if let Some(level) = level {
                counts[level as usize] += 1;
            }
        }
        let (level, count) = counts.iter().enumerate().max_by_key(|(_, count)| **count)?;
        (*count > 0).then_some(level as u8)
    };
    let (low, high) = (mode(-GROUND_DISTANCE)?, mode(GROUND_DISTANCE)?);
    Some((low.min(high), low.max(high)))
}

/// The surface of the cells of a ramp of one kind, each cell a quad whose corners are as high as
/// they are far up the ramp: the ramp is a slope from its low edge to its high edge, not stairs.
/// The positions are in the axes of the map (x, the height, z are the map's y, height and x).
fn ramp_surface(
    cells: &[IVec2],
    up: Vec2,
    (along_min, along_max): (f32, f32),
    (low, high): (f32, f32),
) -> Mesh {
    let height = |corner: Vec2| {
        let progress = if along_max > along_min {
            ((corner.dot(up) - along_min) / (along_max - along_min)).clamp(0., 1.)
        } else {
            0.
        };
        low + (high - low) * progress + RAMP_LIFT * MAP_SCALE_FACTOR
    };
    let mut positions: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    let mut indices: Vec<u32> = vec![];
    for cell in cells {
        let corners = [
            cell.as_vec2(),
            cell.as_vec2() + Vec2::X,
            cell.as_vec2() + Vec2::ONE,
            cell.as_vec2() + Vec2::Y,
        ];
        let points: Vec<Vec3> = corners
            .iter()
            .map(|corner| {
                Vec3::new(
                    corner.y * MAP_SCALE_FACTOR,
                    height(*corner),
                    corner.x * MAP_SCALE_FACTOR,
                )
            })
            .collect();
        let mut normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        let start = positions.len() as u32;
        // The quad faces up.
        let order: [u32; 6] = if normal.y >= 0. {
            [0, 1, 2, 0, 2, 3]
        } else {
            normal = -normal;
            [0, 2, 1, 0, 3, 2]
        };
        for (point, corner) in points.iter().zip(corners) {
            positions.push(point.to_array());
            normals.push(normal.to_array());
            uvs.push(corner.to_array());
        }
        indices.extend(order.iter().map(|i| start + i));
    }
    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

/// Lowers the terrain cubes under the ramps to the low level of their ramp, the ramp surface is
/// drawn instead: a cube (centered on a point of the height map) is under a ramp when at least 3 of
/// the 4 cells around its point are ramp cells. Only the drawn heights change, not the levels.
pub fn lower_ground_under_ramps(terrain: &T3TerrainResource, height_map: &mut T3HeightMapResource) {
    for ramp in &terrain.ramp_list {
        let Some(cells) = ramp_cells::ramp_cells(&ramp.dir, ramp.left_lo, ramp.right_lo) else {
            continue;
        };
        let Some((low, _)) = ramp_levels(&ramp.dir, &cells, height_map, false) else {
            continue;
        };
        let in_ramp = |cell: IVec2| cells.iter().any(|ramp_cell| ramp_cell.cell == cell);
        for ramp_cell in &cells {
            for point in [
                ramp_cell.cell,
                ramp_cell.cell + IVec2::X,
                ramp_cell.cell + IVec2::Y,
                ramp_cell.cell + IVec2::ONE,
            ] {
                let around = [IVec2::ZERO, IVec2::NEG_X, IVec2::NEG_Y, IVec2::NEG_ONE]
                    .iter()
                    .filter(|offset| in_ramp(point + **offset))
                    .count();
                if around < 3 || point.x < 0 || point.y < 0 || point.x as usize >= height_map.width
                {
                    continue;
                }
                let index = point.y as usize * height_map.width + point.x as usize;
                if let Some(height) = height_map.data.get_mut(index) {
                    *height = (*height).min(low);
                }
            }
        }
    }
}

/// Draws each ramp as a slope from the low ground to the high ground: the ramp cells in amber, the
/// buildable top row of the straight ramps in lime and the sides of the ramps, not pathable, in
/// red, in grays for a replay.
pub fn load_t3_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    t3_height_map_res: Res<T3HeightMapResource>,
    t3_terrain: ResMut<T3TerrainResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut loaded: Local<bool>,
    replay: Option<Res<crate::replay::ReplayData>>,
) {
    if *loaded {
        return;
    }
    *loaded = true;
    // A replay is drawn in grays so that the ramps do not look like its bars.
    let (ramp_color, top_color, edge_color) = if replay.is_some() {
        (
            palettes::tailwind::STONE_400,
            palettes::tailwind::STONE_200,
            palettes::tailwind::STONE_600,
        )
    } else {
        (
            palettes::tailwind::AMBER_400,
            palettes::tailwind::LIME_400,
            palettes::tailwind::RED_600,
        )
    };
    let ramp_material = MeshMaterial3d(materials.add(Color::from(ramp_color)));
    let top_material = MeshMaterial3d(materials.add(Color::from(top_color)));
    let edge_material = MeshMaterial3d(materials.add(Color::from(edge_color)));
    for (ramp_ith, ramp) in t3_terrain.ramp_list.iter().enumerate() {
        let Some(cells) = ramp_cells::ramp_cells(&ramp.dir, ramp.left_lo, ramp.right_lo) else {
            tracing::warn!("Ramp {ramp_ith} {:?}: the ramp algorithm failed", ramp.dir);
            continue;
        };
        let Some((low, high)) =
            ramp_levels(&ramp.dir, &cells, &t3_height_map_res, replay.is_some())
        else {
            tracing::warn!("Ramp {ramp_ith} {:?}: no ground around it", ramp.dir);
            continue;
        };
        // The slope goes from the lowest corners of the cells to the highest ones, up the ramp.
        let up = ramp_cells::up_vector(&ramp.dir).as_vec2().normalize();
        let along: Vec<f32> = cells
            .iter()
            .flat_map(|cell| {
                let c = cell.cell.as_vec2();
                [c, c + Vec2::X, c + Vec2::Y, c + Vec2::ONE].map(|corner| corner.dot(up))
            })
            .collect();
        let span = (
            along.iter().copied().fold(f32::INFINITY, f32::min),
            along.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        );
        let heights = (terrain_top(low as f32), terrain_top(high as f32));
        for (kind, material) in [
            (RampCellKind::Ramp, &ramp_material),
            (RampCellKind::Top, &top_material),
            (RampCellKind::Edge, &edge_material),
        ] {
            let kind_cells: Vec<IVec2> = cells
                .iter()
                .filter(|cell| cell.kind == kind)
                .map(|cell| cell.cell)
                .collect();
            if kind_cells.is_empty() {
                continue;
            }
            commands
                .spawn((
                    RampComponent,
                    DisplayInfoOnClick,
                    Name(
                        format!(
                            "Ramp {ramp_ith} going {:?} from level {low} to {high}: {kind:?} cells",
                            ramp.dir
                        )
                        .into(),
                    ),
                    Mesh3d(meshes.add(ramp_surface(&kind_cells, up, span, heights))),
                    material.clone(),
                    Transform::default(),
                ))
                .observe(update_info_on_click);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_the_ramp_corners() {
        let corner = parse_ramp_corner(
            "u(-1.000000e+00, 0.000000e+00) r(0.000000e+00, 1.000000e+00) c=(5.200000e+01, 2.000000e+01) w=2.000000e+00 h=3.000000e+00",
        )
        .unwrap();
        assert_eq!(corner.center, Vec2::new(52., 20.));
        assert_eq!((corner.width, corner.height), (2., 3.));
    }
}
