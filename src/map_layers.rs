//! Draws what happens where on the map in the games of a swarmy snapshot: the fights, the
//! proxies, the creep tumors and the early buildings, a layer each, as bars on top of the terrain.
//! The height of a bar is its count compared to the most counted spot of its layer, each layer has
//! its color and its quarter of the spots so that the layers of a spot are seen side by side. The
//! keys F1 to F4 show or hide the layers (2 toggles the round gizmos), clicking a bar shows its
//! numbers. The legend at the left names the colors, the keys and the totals.
//!
//! swarmy writes the layers as JSON, see `MapLayers` in swarmy-common, and passes the file with
//! `--map-layers`.

use crate::MAP_SCALE_FACTOR;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{T3HeightMapResource, terrain_top};
use bevy::color::palettes;
use bevy::prelude::*;
use serde::Deserialize;

/// The tallest bar of a layer, in world units.
const MAX_BAR_HEIGHT: f32 = 4.;

/// A bar covers this share of its quarter of the spot.
const BAR_SHARE: f32 = 0.8;

/// The keys that show or hide the layers, in order.
const LAYER_KEYS: [KeyCode; 4] = [KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4];

#[derive(Debug, Clone, Deserialize)]
pub struct MapLayerSpot {
    pub x: u32,
    pub y: u32,
    pub count: usize,
    pub games: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapLayer {
    pub kind: String,
    pub label: String,
    pub description: String,
    pub spots: Vec<MapLayerSpot>,
}

#[derive(Debug, Clone, Deserialize, Resource)]
pub struct MapLayers {
    pub map_title: String,
    pub spot_cells: u32,
    pub games: usize,
    pub layers: Vec<MapLayer>,
}

impl MapLayers {
    pub fn load(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
        serde_json::from_str(&json).map_err(|err| format!("{path}: {err}"))
    }
}

/// The color of a layer by its kind: four hues apart from each other and from the gray terrain.
fn layer_color(kind: &str) -> Color {
    Color::from(match kind {
        "fights" => palettes::tailwind::RED_500,
        "proxies" => palettes::tailwind::BLUE_400,
        "creep" => palettes::tailwind::FUCHSIA_400,
        _ => palettes::tailwind::AMBER_400,
    })
}

/// The name of the color of a layer, for Help and the legend.
pub fn layer_color_name(kind: &str) -> &'static str {
    match kind {
        "fights" => "red",
        "proxies" => "blue",
        "creep" => "magenta",
        _ => "amber",
    }
}

/// The quarter of the spot of the nth layer, in map cells from the corner of the spot.
fn quarter_center(layer: usize, spot_cells: f32) -> (f32, f32) {
    let quarter = spot_cells / 4.;
    let (dx, dy) = [(1., 1.), (3., 1.), (1., 3.), (3., 3.)][layer % 4];
    (dx * quarter, dy * quarter)
}

/// A bar of the nth layer.
#[derive(Component)]
struct LayerBar(usize);

pub struct MapLayersPlugin;

impl Plugin for MapLayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_legend).add_systems(
            Update,
            (
                spawn_bars.run_if(resource_exists::<T3HeightMapResource>),
                toggle_layers,
            ),
        );
    }
}

fn spawn_bars(
    mut commands: Commands,
    layers: Res<MapLayers>,
    height_map: Res<T3HeightMapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    *spawned = true;
    let cells = layers.spot_cells as f32;
    let side = cells / 2. * BAR_SHARE * MAP_SCALE_FACTOR;
    let mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(side, 1., side))));
    for (idx, layer) in layers.layers.iter().enumerate() {
        let max_count = layer
            .spots
            .iter()
            .map(|spot| spot.count)
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        let color = layer_color(&layer.kind);
        let material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 0.6,
            ..default()
        }));
        for (rank, spot) in layer.spots.iter().enumerate() {
            let (dx, dy) = quarter_center(idx, cells);
            let (map_x, map_y) = (spot.x as f32 * cells + dx, spot.y as f32 * cells + dy);
            let ground = height_map
                .height_at(map_x as i32, map_y as i32)
                .map(|height| terrain_top(height as f32))
                .unwrap_or_default();
            let height = (spot.count as f32 / max_count * MAX_BAR_HEIGHT).max(0.05);
            commands
                .spawn((
                    Name::new(format!(
                        "{} #{} at cells ({}, {}): {} in {} games",
                        layer.label,
                        rank + 1,
                        spot.x * layers.spot_cells,
                        spot.y * layers.spot_cells,
                        spot.count,
                        spot.games,
                    )),
                    LayerBar(idx),
                    DisplayInfoOnClick,
                    mesh.clone(),
                    material.clone(),
                    // The map x is the world z, as for the terrain.
                    Transform::from_xyz(
                        map_y * MAP_SCALE_FACTOR,
                        ground + height / 2.,
                        map_x * MAP_SCALE_FACTOR,
                    )
                    .with_scale(Vec3::new(1., height, 1.)),
                ))
                .observe(update_info_on_click);
        }
    }
}

/// The state of a layer in the legend, shown or hidden.
#[derive(Component)]
struct LayerState(usize);

fn toggle_layers(
    keys: Res<ButtonInput<KeyCode>>,
    mut bars: Query<(&LayerBar, &mut Visibility)>,
    mut states: Query<(&LayerState, &mut Text)>,
) {
    for (idx, key) in LAYER_KEYS.iter().enumerate() {
        if !keys.just_pressed(*key) {
            continue;
        }
        let mut shown = true;
        for (bar, mut visibility) in &mut bars {
            if bar.0 == idx {
                visibility.toggle_visible_hidden();
                shown = *visibility != Visibility::Hidden;
            }
        }
        for (state, mut text) in &mut states {
            if state.0 == idx {
                text.0 = if shown { "shown" } else { "hidden" }.to_string();
            }
        }
    }
}

/// The legend at the left: what each color is, its key, its total and whether it is shown.
fn spawn_legend(mut commands: Commands, layers: Res<MapLayers>) {
    let text_color = Color::srgb(0.93, 0.94, 0.96);
    let muted = Color::srgb(0.68, 0.71, 0.76);
    commands
        .spawn((
            Name::new("Map lessons legend"),
            Node {
                position_type: PositionType::Absolute,
                top: px(56),
                left: px(8),
                max_width: px(460),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(8),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.08, 0.1, 0.88)),
        ))
        .with_children(|legend| {
            legend.spawn((
                Text::new(format!(
                    "Map lessons of {}, {} games",
                    layers.map_title, layers.games
                )),
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(16.),
                    ..default()
                },
                TextColor(text_color),
            ));
            legend.spawn((
                Text::new("F1 to F4 show or hide a layer, ? at the top right explains the rest"),
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(13.),
                    ..default()
                },
                TextColor(muted),
            ));
            for (idx, layer) in layers.layers.iter().enumerate() {
                let total: usize = layer.spots.iter().map(|spot| spot.count).sum();
                legend
                    .spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(8),
                            ..default()
                        })
                        .with_children(|line| {
                            line.spawn((
                                Node {
                                    width: px(14),
                                    height: px(14),
                                    border_radius: BorderRadius::all(px(3)),
                                    ..default()
                                },
                                BackgroundColor(layer_color(&layer.kind)),
                            ));
                            line.spawn((
                                Text::new(format!("F{}  {}  {total}", idx + 1, layer.label)),
                                TextFont {
                                    font_size: bevy::prelude::FontSize::Px(15.),
                                    ..default()
                                },
                                TextColor(text_color),
                            ));
                            line.spawn((
                                Text::new("shown"),
                                LayerState(idx),
                                TextFont {
                                    font_size: bevy::prelude::FontSize::Px(13.),
                                    ..default()
                                },
                                TextColor(muted),
                            ));
                        });
                        row.spawn((
                            Text::new(layer.description.clone()),
                            TextFont {
                                font_size: bevy::prelude::FontSize::Px(13.),
                                ..default()
                            },
                            TextColor(muted),
                            Node {
                                margin: UiRect::left(px(22)),
                                ..default()
                            },
                        ));
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_layers_of_swarmy() {
        let json = r#"{"map_title":"Pylon LE","spot_cells":4,"games":109,"layers":[
            {"kind":"fights","label":"Fights","description":"Where units died",
             "spots":[{"x":26,"y":41,"count":288,"games":46}]}]}"#;
        let layers: MapLayers = serde_json::from_str(json).unwrap();
        assert_eq!(layers.layers[0].spots[0].count, 288);
        assert_eq!(quarter_center(0, 4.), (1., 1.));
        assert_eq!(quarter_center(3, 4.), (3., 3.));
    }
}
