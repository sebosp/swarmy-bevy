//! Draws the [`ReplayData`]: per aggregated cell and player a bar for the units born, in the color
//! of the player, and a darker bar for the units lost, up to the current time, on top of the
//! flattened terrain, drawn in grays so that its levels do not look like the players. The time
//! moves with the < > buttons or the Left/Right arrows (Home/End for the start/end of the game).
//! A panel shows the players with their color, result, upgrades and most effective unit and
//! ability.

use super::{AGGREGATE_CELLS, ReplayData, game_time};
use crate::MAP_SCALE_FACTOR;
use crate::SelectedObjectName;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{FLAT_TERRAIN_LEVEL, terrain_top};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::feathers::controls::*;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use std::collections::HashMap;

/// The tallest bar, for the cell with the most units born or died in the game, in world units.
pub const MAX_BAR_HEIGHT: f32 = 4.;

/// The width of a bar in map cells, the born and lost bars of a player are side by side.
const BAR_WIDTH_CELLS: f32 = AGGREGATE_CELLS as f32 * 0.4;

/// The depth of the bars of all the players of a cell in map cells, the players are one after
/// the other.
const BAR_DEPTH_CELLS: f32 = AGGREGATE_CELLS as f32 * 0.8;

/// The color of the units born of a player, its color in the game.
pub fn born_color(color: [u8; 4]) -> Color {
    let [r, g, b, _] = color;
    Color::srgb_u8(r, g, b)
}

/// The color of the units lost of a player, its color darker, so that the bars of a player have
/// the same hue.
pub fn lost_color(color: [u8; 4]) -> Color {
    born_color(color).mix(&Color::BLACK, 0.6)
}

/// The size of the upgrade and unit icons in the players panel.
const ICON_SIZE: f32 = 28.;

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IconImages>()
            .add_systems(Startup, (init_replay_time, spawn_bars))
            .add_systems(Startup, replay_panel.spawn())
            .add_systems(
                Update,
                (
                    step_time_with_keys,
                    (update_bars, update_time_text, update_players_panel)
                        .run_if(resource_changed::<ReplayTime>),
                ),
            );
    }
}

/// The time the replay is shown at, one of [`ReplayData::time_steps`].
#[derive(Resource, Debug, Default)]
pub struct ReplayTime {
    pub steps: Vec<u32>,
    pub step: usize,
}

impl ReplayTime {
    pub fn seconds(&self) -> u32 {
        self.steps.get(self.step).copied().unwrap_or_default()
    }

    fn move_by(&mut self, delta: isize) {
        let last = self.steps.len().saturating_sub(1) as isize;
        self.step = (self.step as isize + delta).clamp(0, last) as usize;
    }
}

/// The initial time, i.e. `--replay-seconds` to take a screenshot at a given time.
#[derive(Resource, Debug, Default)]
pub struct ReplayStartSeconds(pub Option<u32>);

/// A bar of a player in an aggregated cell.
#[derive(Component, Debug)]
pub struct CellBar {
    pub cell: (u32, u32),
    /// The index of the player in [`ReplayData::players`].
    pub player: usize,
    pub born: bool,
}

/// The current time label.
#[derive(Component, Default, Clone)]
struct ReplayTimeText;

/// The container of the players, rebuilt when the time changes.
#[derive(Component, Default, Clone)]
struct ReplayPlayersPanel;

/// The icon textures decoded from the DDS files embedded by s2protocol.
#[derive(Resource, Default)]
struct IconImages(HashMap<String, Option<Handle<Image>>>);

impl IconImages {
    fn get(&mut self, icon: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        self.0
            .entry(icon.to_string())
            .or_insert_with(|| {
                let dds = s2protocol::textures::icon_dds(icon)?;
                // The textures are DXT5 (BC3) compressed, supported by the desktop GPUs.
                Image::from_buffer(
                    dds,
                    ImageType::Extension("dds"),
                    CompressedImageFormats::BC,
                    true,
                    ImageSampler::default(),
                    RenderAssetUsages::default(),
                )
                .inspect_err(|err| tracing::warn!("Unable to decode the icon {icon}: {err:?}"))
                .ok()
                .map(|image| images.add(image))
            })
            .clone()
    }
}

fn init_replay_time(
    mut commands: Commands,
    replay: Res<ReplayData>,
    start: Res<ReplayStartSeconds>,
) {
    let steps = replay.time_steps();
    // The bars grow from the start, a screenshot can start at a given time.
    let step = start
        .0
        .map(|seconds| {
            steps
                .iter()
                .position(|step| *step >= seconds)
                .unwrap_or(steps.len() - 1)
        })
        .unwrap_or_default();
    commands.insert_resource(ReplayTime { steps, step });
}

/// The center of an aggregated cell in world coordinates, the map x is the world z.
fn cell_center(cell: (u32, u32)) -> (f32, f32) {
    let half = AGGREGATE_CELLS as f32 / 2.;
    let map_x = cell.0 as f32 * AGGREGATE_CELLS as f32 + half;
    let map_y = cell.1 as f32 * AGGREGATE_CELLS as f32 + half;
    (map_y * MAP_SCALE_FACTOR, map_x * MAP_SCALE_FACTOR)
}

/// Spawns the bars of every player in every cell with units born or died during the game, hidden
/// until they have a count.
fn spawn_bars(
    mut commands: Commands,
    replay: Res<ReplayData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let players = replay.players.len().max(1);
    let width = BAR_WIDTH_CELLS * MAP_SCALE_FACTOR;
    let depth = BAR_DEPTH_CELLS * MAP_SCALE_FACTOR / players as f32;
    let mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(width, 1., depth))));
    let player_materials: Vec<_> = replay
        .players
        .iter()
        .map(|player| {
            let born = born_color(player.color);
            let born = materials.add(StandardMaterial {
                base_color: born,
                // A little glow with the bloom, the lost bars stay dim.
                emissive: born.to_linear() * 0.6,
                ..default()
            });
            let lost = materials.add(StandardMaterial {
                base_color: lost_color(player.color),
                perceptual_roughness: 1.,
                ..default()
            });
            (MeshMaterial3d(born), MeshMaterial3d(lost))
        })
        .collect();
    for (cell, player) in replay.cell_counts(replay.duration_seconds).into_keys() {
        let (world_x, world_z) = cell_center(cell);
        // The players of a cell one after the other, centered on the cell.
        let player_offset = (player as f32 - (players - 1) as f32 / 2.) * depth;
        let Some((born_material, lost_material)) = player_materials.get(player) else {
            continue;
        };
        for born in [true, false] {
            // The born bar is to the left of the lost bar.
            let offset = if born { -0.5 } else { 0.5 } * width;
            commands
                .spawn((
                    CellBar { cell, player, born },
                    Name::new(""),
                    DisplayInfoOnClick,
                    mesh.clone(),
                    if born {
                        born_material.clone()
                    } else {
                        lost_material.clone()
                    },
                    Transform::from_xyz(
                        world_x + offset,
                        terrain_top(FLAT_TERRAIN_LEVEL),
                        world_z + player_offset,
                    ),
                    Visibility::Hidden,
                ))
                .observe(update_info_on_click);
        }
    }
}

fn update_bars(
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    mut bars: Query<(&CellBar, &mut Transform, &mut Visibility, &mut Name)>,
) {
    let seconds = time.seconds();
    let counts = replay.cell_counts(seconds);
    let max_count = replay.max_cell_count().max(1) as f32;
    let ground = terrain_top(FLAT_TERRAIN_LEVEL);
    for (bar, mut transform, mut visibility, mut name) in &mut bars {
        let (born, died) = counts
            .get(&(bar.cell, bar.player))
            .copied()
            .unwrap_or_default();
        let count = if bar.born { born } else { died };
        if count == 0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        let height = count as f32 / max_count * MAX_BAR_HEIGHT;
        transform.scale.y = height;
        transform.translation.y = ground + height / 2.;
        let (x, y) = (bar.cell.0 * AGGREGATE_CELLS, bar.cell.1 * AGGREGATE_CELLS);
        let player = replay
            .players
            .get(bar.player)
            .map(|player| player.name.as_str())
            .unwrap_or_default();
        name.set(format!(
            "Cells ({x}, {y}) to ({}, {}) at {}: {player} {born} units born, {died} lost",
            x + AGGREGATE_CELLS - 1,
            y + AGGREGATE_CELLS - 1,
            game_time(seconds),
        ));
    }
}

fn step_time_with_keys(keys: Res<ButtonInput<KeyCode>>, mut time: ResMut<ReplayTime>) {
    if keys.just_pressed(KeyCode::ArrowRight) {
        time.move_by(1);
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        time.move_by(-1);
    } else if keys.just_pressed(KeyCode::Home) {
        time.step = 0;
    } else if keys.just_pressed(KeyCode::End) {
        time.step = time.steps.len().saturating_sub(1);
    }
}

fn step_backward(_activate: On<Activate>, mut time: ResMut<ReplayTime>) {
    time.move_by(-1);
}

fn step_forward(_activate: On<Activate>, mut time: ResMut<ReplayTime>) {
    time.move_by(1);
}

fn update_time_text(
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    mut text: Single<&mut Text, With<ReplayTimeText>>,
) {
    text.0 = format!(
        "{} / {}",
        game_time(time.seconds()),
        game_time(replay.duration_seconds)
    );
}

/// The time controls and the players, at the top right.
fn replay_panel() -> impl SceneList {
    bsn_list! {
        Node {
            position_type: PositionType::Absolute,
            top: px(40),
            right: px(8),
            width: px(380),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            padding: px(8),
            row_gap: px(8),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("<") ThemedText }
                }
                on(step_backward)
                --
                Text("") ThemedText ReplayTimeText
                --
                @FeathersButton {
                    @caption: bsn! { Text(">") ThemedText }
                }
                on(step_forward)
            ]
            --
            Text("Per 4x4 cells and player: the units born in the color of the player, the units lost in a darker shade next to them. Left/Right arrows to move in time, Home/End for the start/end.")
            ThemedText
            TextFont {
                font_size: bevy::prelude::FontSize::Px(11.),
            }
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
            }
            ReplayPlayersPanel
        ]
    }
}

/// Shows the name of a hovered icon in the selection label.
fn show_name_on_hover(
    over: On<PointerOver>,
    names: Query<&Name>,
    mut selected: Single<&mut Text, With<SelectedObjectName>>,
) {
    if let Ok(name) = names.get(over.entity) {
        selected.0 = name.to_string();
    }
}

fn small_text(text: impl Into<String>, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextColor(color),
        TextFont {
            font_size: bevy::prelude::FontSize::Px(11.),
            ..default()
        },
    )
}

/// An icon with a label under it, hovering it shows `name` in the selection label.
fn spawn_icon(
    parent: &mut ChildSpawnerCommands,
    image: Option<Handle<Image>>,
    label: String,
    name: String,
) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new(name),
        ))
        .observe(show_name_on_hover)
        .with_children(|icon| {
            match image {
                Some(image) => {
                    icon.spawn((
                        ImageNode::new(image),
                        Node {
                            width: px(ICON_SIZE),
                            height: px(ICON_SIZE),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
                None => {
                    icon.spawn((
                        Node {
                            width: px(ICON_SIZE),
                            height: px(ICON_SIZE),
                            ..default()
                        },
                        BackgroundColor(Color::from(palettes::tailwind::GRAY_700)),
                        Pickable::IGNORE,
                    ));
                }
            }
            icon.spawn((small_text(label, Color::WHITE), Pickable::IGNORE));
        });
}

/// Rebuilds the players: their name in their color, the result, the upgrades finished so far with
/// their time, and the unit and the ability with the most kills so far.
fn update_players_panel(
    mut commands: Commands,
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    panel: Single<Entity, With<ReplayPlayersPanel>>,
    mut icons: ResMut<IconImages>,
    mut images: ResMut<Assets<Image>>,
) {
    let seconds = time.seconds();
    commands.entity(*panel).despawn_related::<Children>();
    commands.entity(*panel).with_children(|panel| {
        for (idx, player) in replay.players.iter().enumerate() {
            let [r, g, b, a] = player.color;
            let color = Color::srgba_u8(r, g, b, a);
            let (marker, result_color) = match player.result.as_str() {
                "Win" => ("Win", palettes::tailwind::GREEN_500),
                "Loss" => ("Loss", palettes::tailwind::RED_500),
                _ => (player.result.as_str(), palettes::tailwind::GRAY_400),
            };
            let upgrades = replay.upgrades_until(idx, seconds);
            let upgrade_icons: Vec<_> = upgrades
                .iter()
                .map(|upgrade| {
                    let image = upgrade
                        .icon
                        .as_deref()
                        .and_then(|icon| icons.get(icon, &mut images));
                    (image, *upgrade)
                })
                .collect();
            let unit = replay
                .most_effective_unit(idx, seconds)
                .map(|(unit, kills)| {
                    let image = replay
                        .unit_icons
                        .get(&unit)
                        .and_then(|icon| icons.get(icon, &mut images));
                    (image, unit, kills)
                });
            let ability = replay
                .most_effective_ability(idx, seconds)
                .map(|(ability, kills)| {
                    let image = replay
                        .ability_icons
                        .get(&ability)
                        .and_then(|icon| icons.get(icon, &mut images));
                    (image, ability, kills)
                });
            panel
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            Node {
                                width: px(12),
                                height: px(12),
                                ..default()
                            },
                            BackgroundColor(color),
                        ));
                        header.spawn((
                            Text::new(player.name.clone()),
                            TextColor(color),
                            TextFont {
                                font_size: bevy::prelude::FontSize::Px(14.),
                                ..default()
                            },
                        ));
                        header.spawn(small_text(
                            player.race.clone(),
                            Color::from(palettes::tailwind::GRAY_400),
                        ));
                        header.spawn(small_text(marker, Color::from(result_color)));
                        // The colors of the bars of the player.
                        for (label, bar_color) in [
                            ("born", born_color(player.color)),
                            ("lost", lost_color(player.color)),
                        ] {
                            header.spawn((
                                Node {
                                    width: px(10),
                                    height: px(10),
                                    margin: UiRect::left(px(4)),
                                    ..default()
                                },
                                BackgroundColor(bar_color),
                            ));
                            header.spawn(small_text(
                                label.to_string(),
                                Color::from(palettes::tailwind::GRAY_400),
                            ));
                        }
                    });
                    row.spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(4),
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|effective| {
                        if let Some((image, unit, kills)) = unit {
                            spawn_icon(
                                effective,
                                image,
                                format!("{kills} kills"),
                                format!("Most effective unit: {unit}, {kills} kills"),
                            );
                        }
                        if let Some((image, ability, kills)) = ability {
                            spawn_icon(
                                effective,
                                image,
                                format!("{kills} kills"),
                                format!(
                                    "Most effective ability: {ability}, {kills} kills by units \
                                     last commanded to use it"
                                ),
                            );
                        }
                        for (image, upgrade) in upgrade_icons {
                            spawn_icon(
                                effective,
                                image,
                                game_time(upgrade.seconds),
                                format!(
                                    "{} finished at {}",
                                    upgrade.name,
                                    game_time(upgrade.seconds)
                                ),
                            );
                        }
                    });
                });
        }
    });
}
