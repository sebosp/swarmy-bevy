//! The ground pathing drawn over the terrain, the View menu's Pathing (`--pathing`): the cells a
//! ground unit can walk on, read from the map by `s2protocol::cache_handles::pathing`. With a
//! replay, the buildings, minerals, geysers and rocks standing at the time shown block their
//! cells, and the areas they close off from the open ground (a base walled in) stand out: the
//! units born there cannot leave them on foot.
//!
//! One mesh for all the cells, rebuilt when the time changes, with one unlit material: the
//! colors are in the vertices.

use crate::MAP_SCALE_FACTOR;
use crate::replay::ReplayData;
use crate::replay::plugin::ReplayTime;
use crate::t3_height_map::{T3HeightMapResource, terrain_top};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use s2protocol::cache_handles::pathing::{PathingBlockers, PathingGrid};

/// The ground a unit can walk on, connected to the open ground: aqua, faint.
pub const WALKABLE_COLOR: Srgba =
    Srgba::rgb(0x19 as f32 / 255., 0x9e as f32 / 255., 0x70 as f32 / 255.);
/// The cells taken by a building, a mineral, a geyser or a rock: orange.
pub const BLOCKED_COLOR: Srgba =
    Srgba::rgb(0xd9 as f32 / 255., 0x59 as f32 / 255., 0x26 as f32 / 255.);
/// The ground closed off from the open ground: violet.
pub const CLOSED_COLOR: Srgba =
    Srgba::rgb(0x90 as f32 / 255., 0x85 as f32 / 255., 0xe9 as f32 / 255.);

/// Closed areas smaller than this many cells are left out: the gaps between buildings are not
/// pockets a unit is born in, and the footprints are rounded to the cells.
pub const MIN_CLOSED_CELLS: usize = 4;

/// How far above the terrain the cells are drawn, so that they do not fight with its top.
const LIFT: f32 = 0.02;

/// The pathing grid of the map, when its cache has the cliff levels.
#[derive(Resource, Debug)]
pub struct PathingResource(pub PathingGrid);

/// Whether the pathing is drawn, and the time it was drawn for.
#[derive(Resource, Debug, Default)]
pub struct PathingOverlay {
    pub enabled: bool,
    drawn: Option<u32>,
}

impl PathingOverlay {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            drawn: None,
        }
    }
}

#[derive(Component, Debug)]
pub struct PathingOverlayMesh;

/// The View menu's Pathing checkbox, checked at the start with `--pathing`.
#[derive(Component, Default, Clone, Debug)]
pub struct PathingCheckbox;

/// A unit that blocks the ground appears or changes (`unit` its new type), or goes (`None`).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockerEvent {
    pub seconds: u32,
    pub tag_index: u32,
    pub unit: Option<String>,
    pub x: i32,
    pub y: i32,
}

pub struct PathingPlugin {
    pub enabled: bool,
}

impl Plugin for PathingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PathingOverlay::new(self.enabled))
            .add_systems(Update, (update_pathing_overlay, check_the_checkbox));
    }
}

fn check_the_checkbox(
    mut commands: Commands,
    overlay: Res<PathingOverlay>,
    checkboxes: Query<Entity, Added<PathingCheckbox>>,
) {
    for checkbox in &checkboxes {
        if overlay.enabled {
            commands.entity(checkbox).insert(bevy::ui::Checked);
        }
    }
}

/// Shows or hides the pathing, from the View menu.
pub fn toggle_pathing(
    value_change: On<bevy::ui_widgets::ValueChange<bool>>,
    mut commands: Commands,
    mut overlay: ResMut<PathingOverlay>,
) {
    overlay.enabled = value_change.value;
    if value_change.value {
        commands
            .entity(value_change.source)
            .insert(bevy::ui::Checked);
    } else {
        commands
            .entity(value_change.source)
            .remove::<bevy::ui::Checked>();
    }
}

/// The blockers standing at `seconds`.
pub fn blockers_at(events: &[BlockerEvent], seconds: u32) -> PathingBlockers {
    let mut blockers = PathingBlockers::default();
    for event in events.iter().take_while(|event| event.seconds <= seconds) {
        match &event.unit {
            Some(unit) => blockers.set(event.tag_index, unit, event.x, event.y),
            None => blockers.remove(event.tag_index),
        }
    }
    blockers
}

/// The color of each cell (row by row from the bottom), None where nobody can stand.
pub fn cell_colors(grid: &PathingGrid, blockers: &PathingBlockers) -> Vec<Option<Srgba>> {
    let regions = grid.regions(|x, y| blockers.is_blocked(x, y));
    let open = regions.largest();
    (0..grid.width * grid.height)
        .map(|index| {
            let (x, y) = ((index % grid.width) as i32, (index / grid.width) as i32);
            if !grid.pathable[index] {
                return None;
            }
            if blockers.is_blocked(x, y) {
                return Some(BLOCKED_COLOR);
            }
            let label = regions.labels[index];
            if label == open {
                Some(WALKABLE_COLOR.with_alpha(0.35))
            } else if regions.sizes[label as usize - 1] >= MIN_CLOSED_CELLS {
                Some(CLOSED_COLOR.with_alpha(0.85))
            } else {
                None
            }
        })
        .collect()
}

/// The top of the terrain at a cell: the highest of its corners in the height map.
fn cell_top(height_map: &T3HeightMapResource, x: usize, y: usize) -> f32 {
    let height_at = |x: usize, y: usize| {
        let (x, y) = (
            x.min(height_map.width.saturating_sub(1)),
            y.min(height_map.height.saturating_sub(1)),
        );
        height_map
            .data
            .get(y * height_map.width + x)
            .copied()
            .unwrap_or_default()
    };
    let highest = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
        .into_iter()
        .map(|(x, y)| height_at(x, y))
        .max()
        .unwrap_or_default();
    terrain_top(highest as f32) + LIFT
}

/// One quad per colored cell, a cell spans (x, y) to (x + 1, y + 1) of the map, drawn at
/// (map y, height, map x) as the terrain.
fn overlay_mesh(
    grid: &PathingGrid,
    colors: &[Option<Srgba>],
    height_map: &T3HeightMapResource,
) -> Mesh {
    let (mut positions, mut normals, mut vertex_colors, mut indices) =
        (vec![], vec![], vec![], vec![]);
    for (index, color) in colors.iter().enumerate() {
        let Some(color) = color else {
            continue;
        };
        let (x, y) = (index % grid.width, index / grid.width);
        let top = cell_top(height_map, x, y);
        let (x0, x1) = (
            y as f32 * MAP_SCALE_FACTOR,
            (y + 1) as f32 * MAP_SCALE_FACTOR,
        );
        let (z0, z1) = (
            x as f32 * MAP_SCALE_FACTOR,
            (x + 1) as f32 * MAP_SCALE_FACTOR,
        );
        let first = positions.len() as u32;
        positions.extend([[x0, top, z0], [x0, top, z1], [x1, top, z1], [x1, top, z0]]);
        normals.extend([[0., 1., 0.]; 4]);
        vertex_colors.extend([LinearRgba::from(*color).to_f32_array(); 4]);
        // Counter-clockwise seen from above.
        indices.extend([0, 1, 2, 0, 2, 3].map(|corner| first + corner));
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// Draws the pathing at the time shown, again when the time changes; removes it when disabled.
#[allow(clippy::too_many_arguments)]
fn update_pathing_overlay(
    mut commands: Commands,
    mut overlay: ResMut<PathingOverlay>,
    grid: Option<Res<PathingResource>>,
    height_map: Option<Res<T3HeightMapResource>>,
    replay: Option<Res<ReplayData>>,
    time: Option<Res<ReplayTime>>,
    drawn: Query<Entity, With<PathingOverlayMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !overlay.enabled {
        if overlay.drawn.take().is_some() {
            drawn
                .iter()
                .for_each(|entity| commands.entity(entity).despawn());
        }
        return;
    }
    let (Some(grid), Some(height_map)) = (grid, height_map) else {
        return;
    };
    let seconds = time.map(|time| time.seconds()).unwrap_or_default();
    if overlay.drawn == Some(seconds) {
        return;
    }
    overlay.drawn = Some(seconds);
    drawn
        .iter()
        .for_each(|entity| commands.entity(entity).despawn());
    let blockers = replay
        .map(|replay| blockers_at(&replay.blockers, seconds))
        .unwrap_or_default();
    let colors = cell_colors(&grid.0, &blockers);
    commands.spawn((
        PathingOverlayMesh,
        Name::new("Pathing"),
        Mesh3d(meshes.add(overlay_mesh(&grid.0, &colors, &height_map))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::default(),
        Pickable::IGNORE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_keeps_the_blockers_standing_at_the_time() {
        let event = |seconds, unit: Option<&str>| BlockerEvent {
            seconds,
            tag_index: 1,
            unit: unit.map(str::to_string),
            x: 10,
            y: 10,
        };
        let events = vec![
            event(0, Some("SupplyDepot")),
            event(30, Some("SupplyDepotLowered")),
            event(60, Some("SupplyDepot")),
            event(90, None),
        ];
        assert!(blockers_at(&events, 10).is_blocked(10, 10));
        assert!(!blockers_at(&events, 40).is_blocked(10, 10));
        assert!(blockers_at(&events, 60).is_blocked(9, 9));
        assert!(!blockers_at(&events, 100).is_blocked(10, 10));
    }
}
