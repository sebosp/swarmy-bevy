//! Draws the [`ReplayData`]: per aggregated cell and player a bar, in the color of the player, for
//! the units died or born up to the current time (a dropdown chooses, the units died by default),
//! of every unit type or of the one chosen in the units panel, on top of the flattened terrain,
//! drawn in grays so that its levels do not look like the players. The time moves with the < >
//! buttons or the Left/Right arrows (Home/End for the start/end of the game). A panel shows the
//! players with their color, result, upgrades and most effective unit and ability.

use super::{AGGREGATE_CELLS, CellEventKind, ReplayData, game_time};
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

/// The width of a bar in map cells.
const BAR_WIDTH_CELLS: f32 = AGGREGATE_CELLS as f32 * 0.8;

/// The depth of the bars of all the players of a cell in map cells, the players are one after
/// the other.
const BAR_DEPTH_CELLS: f32 = AGGREGATE_CELLS as f32 * 0.8;

/// The unit types shown in the units panel, the most frequent ones.
const MAX_PANEL_UNITS: usize = 24;

/// The color of the bars of a player, its color in the game.
pub fn player_color(color: [u8; 4]) -> Color {
    let [r, g, b, _] = color;
    Color::srgb_u8(r, g, b)
}

/// The size of the upgrade and unit icons in the players panel.
const ICON_SIZE: f32 = 28.;

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IconImages>()
            .init_resource::<BarSelection>()
            .add_systems(Startup, (init_replay_time, spawn_bars))
            .add_systems(Startup, replay_panel.spawn())
            .add_systems(
                Update,
                (
                    step_time_with_keys,
                    (update_time_text, update_players_panel).run_if(resource_changed::<ReplayTime>),
                    // They check whether the time or the selection changed.
                    (update_bars, update_units_panel, update_bar_kind_text),
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

/// What the bars count: the units died or born, of every unit type or of one.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct BarSelection {
    pub kind: CellEventKind,
    pub unit: Option<String>,
}

impl Default for BarSelection {
    fn default() -> Self {
        Self {
            kind: CellEventKind::Died,
            unit: None,
        }
    }
}

impl BarSelection {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            CellEventKind::Died => "Units died",
            CellEventKind::Born => "Units born",
        }
    }
}

/// The bar of a player in an aggregated cell.
#[derive(Component, Debug)]
pub struct CellBar {
    pub cell: (u32, u32),
    /// The index of the player in [`ReplayData::players`].
    pub player: usize,
}

/// The caption of the dropdown of the units died or born.
#[derive(Component, Default, Clone)]
struct BarKindText;

/// The unit types of the bars, rebuilt when the time or the selection changes.
#[derive(Component, Default, Clone)]
struct UnitsPanel;

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

/// Spawns the bar of every player in every cell with units born or died during the game, hidden
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
            let color = player_color(player.color);
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                // A little glow with the bloom.
                emissive: color.to_linear() * 0.6,
                ..default()
            }))
        })
        .collect();
    for (cell, player) in replay.cell_players() {
        let (world_x, world_z) = cell_center(cell);
        // The players of a cell one after the other, centered on the cell.
        let player_offset = (player as f32 - (players - 1) as f32 / 2.) * depth;
        let Some(material) = player_materials.get(player) else {
            continue;
        };
        commands
            .spawn((
                CellBar { cell, player },
                Name::new(""),
                DisplayInfoOnClick,
                mesh.clone(),
                material.clone(),
                Transform::from_xyz(
                    world_x,
                    terrain_top(FLAT_TERRAIN_LEVEL),
                    world_z + player_offset,
                ),
                Visibility::Hidden,
            ))
            .observe(update_info_on_click);
    }
}

fn update_bars(
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    selection: Res<BarSelection>,
    mut bars: Query<(&CellBar, &mut Transform, &mut Visibility, &mut Name)>,
) {
    if !time.is_changed() && !selection.is_changed() {
        return;
    }
    let seconds = time.seconds();
    let unit = selection.unit.as_deref();
    let counts = replay.cell_counts(seconds, selection.kind, unit);
    let max_count = replay.max_cell_count(selection.kind, unit).max(1) as f32;
    let ground = terrain_top(FLAT_TERRAIN_LEVEL);
    for (bar, mut transform, mut visibility, mut name) in &mut bars {
        let count = counts
            .get(&(bar.cell, bar.player))
            .copied()
            .unwrap_or_default();
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
        // Clicking the bar shows its unit types in the selection.
        let units = replay
            .unit_counts(seconds, selection.kind, Some((bar.cell, bar.player)))
            .into_iter()
            .filter(|(name, _)| unit.is_none_or(|unit| unit == name))
            .take(6)
            .map(|(name, count)| format!("{count} {name}"))
            .collect::<Vec<_>>()
            .join(", ");
        name.set(format!(
            "Cells ({x}, {y}) to ({}, {}) at {}: {player}, {} {}: {units}",
            x + AGGREGATE_CELLS - 1,
            y + AGGREGATE_CELLS - 1,
            game_time(seconds),
            count,
            selection.kind_label().to_lowercase(),
        ));
    }
}

fn update_bar_kind_text(
    selection: Res<BarSelection>,
    mut text: Single<&mut Text, With<BarKindText>>,
) {
    if !selection.is_changed() {
        return;
    }
    text.0 = selection.kind_label().to_string();
}

fn show_units_died(_activate: On<Activate>, mut selection: ResMut<BarSelection>) {
    selection.kind = CellEventKind::Died;
}

fn show_units_born(_activate: On<Activate>, mut selection: ResMut<BarSelection>) {
    selection.kind = CellEventKind::Born;
}

/// The unit type of a units panel entry, clicking it shows only its bars, clicking it again shows
/// every unit type.
#[derive(Component, Debug, Clone)]
struct UnitFilterButton(Option<String>);

fn toggle_unit_filter(
    click: On<PointerClick>,
    buttons: Query<&UnitFilterButton>,
    parents: Query<&ChildOf>,
    mut selection: ResMut<BarSelection>,
) {
    // The click can be on the icon or the label of the entry.
    let Some(UnitFilterButton(unit)) = std::iter::once(click.entity)
        .chain(parents.iter_ancestors(click.entity))
        .find_map(|entity| buttons.get(entity).ok())
    else {
        return;
    };
    selection.unit = if selection.unit == *unit {
        None
    } else {
        unit.clone()
    };
}

/// Rebuilds the unit types of the bars up to the time, with their count, the selected one framed.
fn update_units_panel(
    mut commands: Commands,
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    selection: Res<BarSelection>,
    panel: Single<Entity, With<UnitsPanel>>,
    mut icons: ResMut<IconImages>,
    mut images: ResMut<Assets<Image>>,
) {
    if !time.is_changed() && !selection.is_changed() {
        return;
    }
    let units: Vec<_> = replay
        .unit_counts(time.seconds(), selection.kind, None)
        .into_iter()
        .take(MAX_PANEL_UNITS)
        .map(|(unit, count)| {
            let image = replay
                .unit_icons
                .get(&unit)
                .and_then(|icon| icons.get(icon, &mut images));
            (image, unit, count)
        })
        .collect();
    commands.entity(*panel).despawn_related::<Children>();
    commands.entity(*panel).with_children(|panel| {
        let all_color = if selection.unit.is_none() {
            Color::WHITE
        } else {
            Color::from(palettes::tailwind::GRAY_400)
        };
        panel
            .spawn((
                Node {
                    padding: UiRect::axes(px(6), px(4)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(all_color),
                UnitFilterButton(None),
                Name::new("Show every unit type"),
            ))
            .observe(show_name_on_hover)
            .observe(toggle_unit_filter)
            .with_child((small_text("All", all_color), Pickable::IGNORE));
        for (image, unit, count) in units {
            let selected = selection.unit.as_deref() == Some(unit.as_str());
            let mut entry = panel.spawn((
                Node {
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(if selected { Color::WHITE } else { Color::NONE }),
                UnitFilterButton(Some(unit.clone())),
            ));
            entry.observe(toggle_unit_filter);
            entry.with_children(|entry| {
                spawn_icon(
                    entry,
                    image,
                    count.to_string(),
                    format!("{unit}: {count}, click to show only its bars"),
                );
            });
        }
    });
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
            @FeathersMenu
            Children [
                @FeathersMenuButton {
                    @caption: bsn! { Text("Units died") ThemedText BarKindText }
                }
                AccessibleLabel("Units died or born")
                Node {
                    flex_grow: 1.0,
                }
                --
                @FeathersMenuPopup
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { Text("Units died") ThemedText }
                    }
                    on(show_units_died)
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { Text("Units born") ThemedText }
                    }
                    on(show_units_born)
                ]
            ]
            --
            Text("The bars count the units of each player per 4x4 cells, in the color of the player. Click on a unit to show only its bars, on a bar to see its units. Left/Right arrows to move in time, Home/End for the start/end.")
            ThemedText
            TextFont {
                font_size: bevy::prelude::FontSize::Px(11.),
            }
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px(4),
                row_gap: px(4),
            }
            UnitsPanel
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
