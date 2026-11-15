use crate::MAP_SCALE_FACTOR;
use crate::map_plugin::MapInfoResource;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::color::palettes;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use s2protocol::cache_handles::map::coords::*;

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

/// A chunk of the terrain, [`CHUNK_CELLS`] cells square in one mesh, painted in the colors of its
/// cells or with the terrain textures ([`crate::terrain_textures`]).
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct TerrainChunk;

/// The cells along a side of a chunk of the terrain. A cuboid per cell made 40,000 entities on a
/// 200x200 map, all visited every frame, and their hidden sides fought with the tops of their
/// neighbors (the borders of the cells flickered); a chunk only has the faces that can be seen.
pub const CHUNK_CELLS: usize = 32;
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
    /// The level of the terrain point (not flattened for a replay nor lowered under the ramps),
    /// None outside the map.
    pub fn level_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.width {
            return None;
        }
        self.levels
            .get(y as usize * self.width + x as usize)
            .copied()
    }

    /// The height at the terrain point, None outside the map.
    pub fn height_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.width {
            return None;
        }
        self.data.get(y as usize * self.width + x as usize).copied()
    }
}

/// E lifts the camera as Space does (the camera's key to go up is Space): at the camera's speed,
/// faster with Shift.
pub fn lift_with_e(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<(
        &mut Transform,
        &FreeCamera,
        &bevy::camera_controller::free_camera::FreeCameraState,
    )>,
) {
    if !keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Space) {
        return;
    }
    for (mut transform, camera, state) in &mut cameras {
        if !state.enabled {
            continue;
        }
        let speed = if keys.pressed(camera.key_run) {
            camera.run_speed
        } else {
            camera.walk_speed
        };
        transform.translation.y += speed * state.speed_multiplier * time.delta_secs();
    }
}

/// The top of the terrain cell drawn for a height, see [`load_t3_height_map`].
pub fn terrain_top(height: f32) -> f32 {
    // The cells were cuboids centered at a quarter of their size, the objects stand on this top.
    height * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR * 0.75
}

/// The bottom of the sides of the terrain cells at the edges of what is drawn.
pub fn terrain_bottom(height: f32) -> f32 {
    -height * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR * 0.25
}

/// The cells drawn: their heights and colors, row by row from the bottom left corner, `columns`
/// cells a row.
pub struct TerrainGrid {
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<u8>,
    /// Linear RGBA.
    pub colors: Vec<[f32; 4]>,
}

impl TerrainGrid {
    fn height(&self, x: i64, y: i64) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.columns || y as usize >= self.rows {
            return None;
        }
        Some(self.heights[y as usize * self.columns + x as usize])
    }
}

/// The vertices of a mesh being built, a quad at a time.
#[derive(Default)]
struct Quads {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Quads {
    /// A quad of 4 corners in order around it, its front towards `normal`.
    fn push(&mut self, corners: [Vec3; 4], normal: Vec3, color: [f32; 4]) {
        let first = self.positions.len() as u32;
        let facing = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .dot(normal);
        for corner in corners {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.colors.push(color);
        }
        let order: [u32; 6] = if facing >= 0. {
            [0, 1, 2, 0, 2, 3]
        } else {
            [0, 2, 1, 0, 3, 2]
        };
        self.indices.extend(order.map(|corner| first + corner));
    }
}

/// The mesh of the cells of a chunk, from (`x0`, `y0`) on: the top of each cell, and its sides
/// where its neighbor is lower (down to the neighbor's top) or not drawn (down to its bottom).
pub fn chunk_mesh(grid: &TerrainGrid, x0: usize, y0: usize) -> Option<Mesh> {
    let mut quads = Quads::default();
    for y in y0..(y0 + CHUNK_CELLS).min(grid.rows) {
        for x in x0..(x0 + CHUNK_CELLS).min(grid.columns) {
            let (xi, yi) = (x as i64, y as i64);
            let Some(height) = grid.height(xi, yi) else {
                continue;
            };
            let color = grid.colors[y * grid.columns + x];
            let top = terrain_top(height as f32);
            // The cells are drawn at (map y, height, map x).
            let (x0, x1) = (
                (y as f32 - 0.5) * MAP_SCALE_FACTOR,
                (y as f32 + 0.5) * MAP_SCALE_FACTOR,
            );
            let (z0, z1) = (
                (x as f32 - 0.5) * MAP_SCALE_FACTOR,
                (x as f32 + 0.5) * MAP_SCALE_FACTOR,
            );
            quads.push(
                [
                    Vec3::new(x0, top, z0),
                    Vec3::new(x0, top, z1),
                    Vec3::new(x1, top, z1),
                    Vec3::new(x1, top, z0),
                ],
                Vec3::Y,
                color,
            );
            let sides = [
                (xi + 1, yi, Vec3::Z),
                (xi - 1, yi, Vec3::NEG_Z),
                (xi, yi + 1, Vec3::X),
                (xi, yi - 1, Vec3::NEG_X),
            ];
            for (nx, ny, normal) in sides {
                let low = grid
                    .height(nx, ny)
                    .map_or(terrain_bottom(height as f32), |h| terrain_top(h as f32));
                if low >= top {
                    continue;
                }
                let (a, b) = if normal.x != 0. {
                    let wx = if normal.x > 0. { x1 } else { x0 };
                    (Vec3::new(wx, 0., z0), Vec3::new(wx, 0., z1))
                } else {
                    let wz = if normal.z > 0. { z1 } else { z0 };
                    (Vec3::new(x0, 0., wz), Vec3::new(x1, 0., wz))
                };
                quads.push(
                    [a.with_y(low), b.with_y(low), b.with_y(top), a.with_y(top)],
                    normal,
                    color,
                );
            }
        }
    }
    if quads.positions.is_empty() {
        return None;
    }
    Some(
        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, quads.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, quads.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, quads.colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(quads.indices)),
    )
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
    // In grays when the bars of a replay are drawn on top, so that their colors stand out.
    let grays = replay.is_some();
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
        // Space lifts the camera, E too (see `lift_with_e`), Q lowers it.
        FreeCamera {
            key_up: KeyCode::Space,
            ..default()
        },
    ));
    // The light, without shadows: the terrain is a grid of cells, shadows cost much and say little.
    commands.spawn((
        PointLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // The cells drawn: up to 15 past the playable area (the data may be larger).
    let columns = t3_height_map.width.min(map_info.cell_right + 16);
    let rows = (t3_height_map.data.len() / t3_height_map.width.max(1)).min(map_info.cell_top + 16);
    let mut grid = TerrainGrid {
        columns,
        rows,
        heights: Vec::with_capacity(columns * rows),
        colors: Vec::with_capacity(columns * rows),
    };
    // The data is row by row from the bottom left corner, the same index the objects and ramps use
    // to find their height.
    for y in 0..rows {
        for x in 0..columns {
            let idx = y * t3_height_map.width + x;
            let cell_height = t3_height_map.data[idx];
            let level = t3_height_map
                .levels
                .get(idx)
                .copied()
                .unwrap_or(cell_height);
            grid.heights.push(cell_height);
            grid.colors.push(
                compute_cell_color(level, x, y, &map_info, grays)
                    .to_linear()
                    .to_f32_array(),
            );
        }
    }
    // The colors of the cells are in the vertices, one material for all.
    let material = materials.add(StandardMaterial::from(Color::WHITE));
    for y0 in (0..rows).step_by(CHUNK_CELLS) {
        for x0 in (0..columns).step_by(CHUNK_CELLS) {
            if let Some(mesh) = chunk_mesh(&grid, x0, y0) {
                commands.spawn((
                    TerrainChunk,
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.clone()),
                    Transform::default(),
                ));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    fn grid(heights: Vec<u8>, columns: usize) -> TerrainGrid {
        TerrainGrid {
            columns,
            rows: heights.len() / columns,
            colors: vec![[1.; 4]; heights.len()],
            heights,
        }
    }

    fn positions(mesh: &Mesh) -> Vec<Vec3> {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        positions.iter().map(|p| Vec3::from_array(*p)).collect()
    }

    #[test]
    fn it_draws_only_the_faces_that_can_be_seen() {
        // Two cells side by side, the second higher: its side over the first is drawn, not the
        // side of the first against it. 2 tops, 3 sides of the first at the edges, 4 of the second.
        let mesh = chunk_mesh(&grid(vec![1, 2], 2), 0, 0).unwrap();
        assert_eq!(positions(&mesh).len(), 9 * 4);
        // Two equal cells: no side between them.
        let mesh = chunk_mesh(&grid(vec![2, 2], 2), 0, 0).unwrap();
        assert_eq!(positions(&mesh).len(), 8 * 4);
    }

    #[test]
    fn it_turns_the_faces_outward() {
        let mesh = chunk_mesh(&grid(vec![1, 3, 2, 0], 2), 0, 0).unwrap();
        let positions = positions(&mesh);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("no normals");
        };
        let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
            panic!("no indices");
        };
        for triangle in indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| positions[triangle[i] as usize]);
            let normal = Vec3::from_array(normals[triangle[0] as usize]);
            assert!((b - a).cross(c - a).dot(normal) > 0.);
        }
    }

    #[test]
    fn it_lowers_a_side_to_the_top_of_its_neighbor() {
        let mesh = chunk_mesh(&grid(vec![1, 2], 2), 0, 0).unwrap();
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("no normals");
        };
        // The side of the second cell over the first.
        let lowest_between = positions(&mesh)
            .into_iter()
            .zip(normals)
            .filter(|(p, n)| {
                (p.z - 0.5 * MAP_SCALE_FACTOR).abs() < 1e-6 && Vec3::from_array(**n) == Vec3::NEG_Z
            })
            .map(|(p, _)| p.y)
            .fold(f32::MAX, f32::min);
        assert_eq!(lowest_between, terrain_top(1.));
    }
}
