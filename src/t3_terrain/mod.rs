use crate::BevySC2MapError;
use crate::MAP_SCALE_FACTOR;
use crate::swarmy_feathers::DisplayInfoOnClick;
use crate::swarmy_feathers::update_info_on_click;
use crate::t3_height_map::CELL_HEIGHT_MULTIPLIER;
use crate::t3_height_map::T3HeightMapResource;
use bevy::color::palettes;
use bevy::prelude::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until;

pub const RAMP_SIZE: f32 = 1.0;
pub const RAMP_HEIGHT: f32 = 1.0;

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
        tracing::info!("try_from: input: {input:?}, output: {res:?}",);

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
    let (tail, _) = tag(" ")(tail)?;
    let height_str = tail;
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

/// Loads the t3 terrain with ramps
pub fn load_t3_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    t3_height_map_res: Res<T3HeightMapResource>,
    t3_terrain: ResMut<T3TerrainResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut loaded: Local<bool>,
) {
    if *loaded {
        return;
    }
    *loaded = true;
    for (ramp_ith, ramp) in t3_terrain.ramp_list.iter().enumerate() {
        // low
        let left_lo_x = ramp.left_lo.x * MAP_SCALE_FACTOR;
        let left_lo_y = ramp.left_lo.y * MAP_SCALE_FACTOR;
        let right_lo_x = ramp.right_lo.x * MAP_SCALE_FACTOR;
        let right_lo_y = ramp.right_lo.y * MAP_SCALE_FACTOR;

        // high
        let left_hi_x = ramp.left_hi.x * MAP_SCALE_FACTOR;
        let left_hi_y = ramp.left_hi.y * MAP_SCALE_FACTOR;
        let right_hi_x = ramp.right_hi.x * MAP_SCALE_FACTOR;
        let right_hi_y = ramp.right_hi.y * MAP_SCALE_FACTOR;

        // Find the height position in the t3_height_map_res xy vector
        // I guess these two should be the same since they are "lo"/"hi"?
        // low
        let left_lo_target_vec_pos =
            ramp.left_lo.y as usize * t3_height_map_res.width as usize + ramp.left_lo.x as usize;
        let right_lo_target_vec_pos =
            ramp.right_lo.y as usize * t3_height_map_res.width as usize + ramp.right_lo.x as usize;
        // high
        let left_hi_target_vec_pos =
            ramp.left_hi.y as usize * t3_height_map_res.width as usize + ramp.left_hi.x as usize;
        let right_hi_target_vec_pos =
            ramp.right_hi.y as usize * t3_height_map_res.width as usize + ramp.right_hi.x as usize;

        // Our height calculation is off by a bit somewhere....
        let left_lo_cell_height =
            (t3_height_map_res.data[left_lo_target_vec_pos] as f32 * MAP_SCALE_FACTOR - 0.01)
                * CELL_HEIGHT_MULTIPLIER;
        let right_lo_cell_height =
            (t3_height_map_res.data[right_lo_target_vec_pos] as f32 * MAP_SCALE_FACTOR - 0.01)
                * CELL_HEIGHT_MULTIPLIER;
        let left_hi_cell_height =
            (t3_height_map_res.data[left_hi_target_vec_pos] as f32 * MAP_SCALE_FACTOR - 0.01)
                * CELL_HEIGHT_MULTIPLIER;
        let right_hi_cell_height =
            (t3_height_map_res.data[right_hi_target_vec_pos] as f32 * MAP_SCALE_FACTOR - 0.01)
                * CELL_HEIGHT_MULTIPLIER;

        // Create Vec3 points for each of the 4 corners of the ramp.
        // Polyline3d points
        let left_lo: Vec3 = Vec3::new(0., 0., 0.);
        let right_lo: Vec3 = Vec3::new(left_lo_y - right_lo_y, 0., left_lo_x - right_lo_x);

        let left_hi: Vec3 = Vec3::new(0., 0., 0.);
        let right_hi: Vec3 = Vec3::new(left_hi_y - right_hi_y, 0., left_hi_x - right_hi_x);

        let left_lo_transform = Transform::from_xyz(left_lo_y, left_lo_cell_height, left_lo_x);
        let right_lo_transform = Transform::from_xyz(right_lo_y, right_lo_cell_height, right_lo_x);

        let left_hi_transform = Transform::from_xyz(left_hi_y, left_hi_cell_height, left_hi_x);
        let right_hi_transform = Transform::from_xyz(right_hi_y, right_hi_cell_height, right_hi_x);

        let ramp_mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(
            MAP_SCALE_FACTOR,
            RAMP_HEIGHT * MAP_SCALE_FACTOR,
            MAP_SCALE_FACTOR,
        ))));
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("left_lo: {}", ramp_ith).into()),
                left_lo_transform,
                ramp_mesh.clone(),
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::RED_600))),
            ))
            .observe(update_info_on_click);
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("right_lo: {}", ramp_ith).into()),
                right_lo_transform,
                ramp_mesh.clone(),
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::GREEN_600))),
            ))
            .observe(update_info_on_click);
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("left_hi: {}", ramp_ith).into()),
                left_hi_transform,
                ramp_mesh.clone(),
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::ORANGE_600))),
            ))
            .observe(update_info_on_click);
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("right_hi: {}", ramp_ith).into()),
                right_hi_transform,
                ramp_mesh,
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::LIME_600))),
            ))
            .observe(update_info_on_click);
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("lo_ramp: {}", ramp_ith).into()),
                //Mesh3d(meshes.add(Segment3d::new(left_lo, right_lo))),
                right_lo_transform,
                Mesh3d(meshes.add(Polyline3d::new([right_lo, left_lo]))),
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::RED_600))),
            ))
            .observe(update_info_on_click);
        commands
            .spawn((
                DisplayInfoOnClick,
                Name(format!("hi_ramp: {}", ramp_ith).into()),
                //Mesh3d(meshes.add(Segment3d::new(left_hi, right_hi))),
                right_hi_transform,
                Mesh3d(meshes.add(Polyline3d::new([right_hi, left_hi]))),
                MeshMaterial3d(materials.add(Color::from(palettes::tailwind::RED_600))),
            ))
            .observe(update_info_on_click);
    }
}
