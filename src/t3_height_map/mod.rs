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

/// A copy of the T3HeightMap that impls Reflect, Resource.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct T3HeightMapResource {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
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

/// Loads the t3 height map.
pub fn load_t3_height_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    t3_height_map: ResMut<T3HeightMapResource>,
    map_info: Res<MapInfoResource>,
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

    // Cuboids for the cells.
    for (idx, cell_height) in t3_height_map.data.iter().enumerate() {
        let x = usize::try_from(
            (t3_height_map.width as i32 - idx as i32).abs() % t3_height_map.width as i32,
        )?;
        let y = usize::try_from(
            (t3_height_map.width as i32 - idx as i32).abs() / t3_height_map.width as i32,
        )?;
        if x > map_info.cell_right + 15 || y > map_info.cell_top + 15 {
            continue;
        }
        let cell_color = compute_cell_color(*cell_height, x, y, &map_info);
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

fn compute_cell_color(cell_height: u8, x: usize, y: usize, map_info: &MapInfoResource) -> Color {
    if x > map_info.cell_right + 15
        || y > map_info.cell_top + 15
        || x + 10 < map_info.cell_left
        || y + 10 < map_info.cell_bottom
    {
        return Color::from(palettes::tailwind::SLATE_500);
    }
    let color = match cell_height {
        0 => palettes::css::BLACK,
        1 => palettes::css::LIGHT_BLUE,
        2 => palettes::tailwind::YELLOW_100,
        3 => palettes::css::DARK_GREEN,
        _ => palettes::css::DARK_RED,
    };
    Color::from(color)
}
