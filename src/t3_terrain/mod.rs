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

///
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
    let (tail, ux) = take_until(", ")(tail)?;
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

/// The thickness of the ramp tiles drawn on top of the terrain.
pub const RAMP_TILE_THICKNESS: f32 = 0.2;

/// Draws the cells of each ramp as tiles going up from the low ground to the high ground, the
/// ramp cells in amber, the buildable top row of the straight ramps in lime and the sides of the
/// ramps, not pathable, in red, in grays for a replay.
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
    let tile_mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(
        MAP_SCALE_FACTOR,
        RAMP_TILE_THICKNESS * MAP_SCALE_FACTOR,
        MAP_SCALE_FACTOR,
    ))));
    // A replay is drawn in grays so that the ramps do not look like the bars of the players.
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
        // The low and high ground around the ramp.
        let heights: Vec<u8> = cells
            .iter()
            .flat_map(|ramp_cell| {
                let IVec2 { x, y } = ramp_cell.cell;
                [
                    (x - 1, y - 1),
                    (x + 2, y - 1),
                    (x - 1, y + 2),
                    (x + 2, y + 2),
                ]
            })
            .filter_map(|(x, y)| t3_height_map_res.height_at(x, y))
            .collect();
        let low = heights.iter().copied().min().unwrap_or_default();
        let high = heights.iter().copied().max().unwrap_or_default();
        let progress = ramp_cells::ramp_progress(&ramp.dir, &cells);
        for (ramp_cell, progress) in cells.iter().zip(progress) {
            let height = low as f32 + (high - low) as f32 * progress;
            let material = match ramp_cell.kind {
                RampCellKind::Ramp => ramp_material.clone(),
                RampCellKind::Top => top_material.clone(),
                RampCellKind::Edge => edge_material.clone(),
            };
            let IVec2 { x, y } = ramp_cell.cell;
            commands
                .spawn((
                    RampComponent,
                    DisplayInfoOnClick,
                    Name(
                        format!(
                            "Ramp {ramp_ith} going {:?} from level {} to {}: {:?} cell ({x}, {y})",
                            ramp.dir, ramp.lo, ramp.hi, ramp_cell.kind
                        )
                        .into(),
                    ),
                    tile_mesh.clone(),
                    material,
                    Transform::from_xyz(
                        (y as f32 + 0.5) * MAP_SCALE_FACTOR,
                        terrain_top(height) + RAMP_TILE_THICKNESS * 0.5 * MAP_SCALE_FACTOR,
                        (x as f32 + 0.5) * MAP_SCALE_FACTOR,
                    ),
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
