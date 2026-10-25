use crate::MAP_SCALE_FACTOR;
use crate::map_plugin::MapInfoResource;
use crate::t3_terrain::T3TerrainResource;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::color::palettes;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::log::tracing;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use s2protocol::cache_handles::map::coords::*;
use s2protocol::cache_handles::map_info::*;
use serde::{Deserialize, Serialize};

pub const CELL_HEIGHT_MULTIPLIER: f32 = 5.;

/// The level of the terrain when it is flattened to show a replay, the colors keep the levels.
pub const FLAT_TERRAIN_LEVEL: f32 = 1.;

/// A copy of the T3HeightMap that impls Reflect, Resource.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct T3HeightMapResource {
    pub width: usize,
    pub height: usize,
    /// The heights the terrain and the objects are drawn at.
    pub data: Vec<u8>,
    /// The terrain levels for the colors, the same as `data` unless the terrain is flattened.
    pub levels: Vec<u8>,
}

/// A so-far unused TerrainCell.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
struct TerrainCell {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    scl_x: f32,
    scl_y: f32,
    scl_z: f32,
}
#[derive(Resource, Default, Reflect, Debug, Clone)]
pub struct MapDimension {
    pub x: f32,
    pub y: f32,
}

impl MapDimension {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<MapCellCoord> for MapDimension {
    fn from(src: MapCellCoord) -> Self {
        Self {
            x: src.x as f32,
            y: src.y as f32,
        }
    }
}

impl T3HeightMapResource {
    /// The height at the terrain point, None outside the map.
    pub fn height_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.width {
            return None;
        }
        self.data.get(y as usize * self.width + x as usize).copied()
    }
}

/// The top of the terrain cuboid drawn for a height, see [`load_t3_height_map`].
pub fn terrain_top(height: f32) -> f32 {
    // The cuboid is centered at a quarter of its size.
    height * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR * 0.75
}

/// Loads the t3 height map.
pub fn load_t3_height_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    t3_height_map: ResMut<T3HeightMapResource>,
    map_info: Res<MapInfoResource>,
    replay: Option<Res<crate::replay::ReplayData>>,
) -> Result<(), BevyError> {
    let max_map_dim = t3_height_map.width.max(t3_height_map.height);
    let map_size = max_map_dim as f32 * MAP_SCALE_FACTOR;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(map_size, map_size * 0.75, map_size)
            .looking_at(Vec3::new(map_size / 2., 0.0, map_size / 2.), Vec3::Y),
        Hdr,
        DepthPrepass,
        NormalPrepass,
        Tonemapping::TonyMcMapface,
        Bloom::default(),
        FreeCamera::default(),
    ));
    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Cuboids for the cells, the data is row by row from the bottom left corner, the same
    // index the objects and ramps use to find their height.
    for (idx, cell_height) in t3_height_map.data.iter().enumerate() {
        let x = idx % t3_height_map.width;
        let y = idx / t3_height_map.width;
        if x > map_info.cell_right + 15 || y > map_info.cell_top + 15 {
            continue;
        }
        let level = t3_height_map
            .levels
            .get(idx)
            .copied()
            .unwrap_or(*cell_height);
        let cell_color = compute_cell_color(level, x, y, &map_info, replay.is_some());
        let (x, y) = (x as f32, y as f32);
        commands.spawn((
            TerrainCell {
                pos_x: x * MAP_SCALE_FACTOR,
                pos_y: 1.,
                pos_z: *cell_height as f32 * MAP_SCALE_FACTOR,
                scl_x: MAP_SCALE_FACTOR,
                scl_y: *cell_height as f32,
                scl_z: MAP_SCALE_FACTOR,
            },
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(
                MAP_SCALE_FACTOR,
                *cell_height as f32 * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR,
                MAP_SCALE_FACTOR,
            )))),
            MeshMaterial3d(materials.add(cell_color)),
            //SceneRoot(asset_server.load(GltfAssetLabel::Mesh(0).from_asset("swarmy-objects.gltf"))),
            Transform::from_xyz(
                y * MAP_SCALE_FACTOR,
                *cell_height as f32 * (CELL_HEIGHT_MULTIPLIER / 4.) * MAP_SCALE_FACTOR,
                x * MAP_SCALE_FACTOR,
            ),
        ));
    }
    Ok(())
}

/// The color of a cell by its level. A replay is drawn in grays, darker the lower, so that the
/// bars in the colors of the players stand out.
fn compute_cell_color(
    cell_height: u8,
    x: usize,
    y: usize,
    map_info: &MapInfoResource,
    replay: bool,
) -> Color {
    if x > map_info.cell_right + 15
        || y > map_info.cell_top + 15
        || x + 10 < map_info.cell_left
        || y + 10 < map_info.cell_bottom
    {
        return Color::from(if replay {
            palettes::tailwind::NEUTRAL_900
        } else {
            palettes::tailwind::SLATE_500
        });
    }
    let color = match (replay, cell_height) {
        (true, 0) => palettes::tailwind::NEUTRAL_950,
        (true, 1) => palettes::tailwind::NEUTRAL_700,
        (true, 2) => palettes::tailwind::NEUTRAL_500,
        (true, 3) => palettes::tailwind::NEUTRAL_400,
        (true, _) => palettes::tailwind::NEUTRAL_300,
        (false, 0) => palettes::css::BLACK,
        (false, 1) => palettes::css::LIGHT_BLUE,
        (false, 2) => palettes::tailwind::YELLOW_100,
        (false, 3) => palettes::css::DARK_GREEN,
        (false, _) => palettes::css::DARK_RED,
    };
    Color::from(color)
}
