//! Draws the [`ReplayData`]: per aggregated cell and player a bar, in the color of the player, for
//! the units died, born or the kills (at the killer) up to the current time (a dropdown chooses,
//! the deaths by default), of every unit type or of the one chosen in the units panel, on top of
//! the flattened terrain, drawn in grays so that its levels do not look like the players. The time
//! moves with the < > buttons or the Left/Right arrows (Home/End for the start/end of the game). A
//! panel shows the players with their color, result and most effective unit and ability, the
//! [`timeline`](super::timeline) their upgrades.

use super::{AGGREGATE_CELLS, CellEventKind, RangeCircle, ReplayData, game_time};
use crate::collapsible::{Pointing, triangle_image};
use crate::swarmy_feathers::SelectionIcons;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{FLAT_TERRAIN_LEVEL, terrain_top};
use crate::{MAP_SCALE_FACTOR, SelectedEntity, SelectedObjectName};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::feathers::controls::*;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::math::Isometry3d;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use std::collections::HashMap;

/// The tallest bar, for the cell with the most units born or died in the game, in world units.
pub const MAX_BAR_HEIGHT: f32 = 4.;

/// The side of the square base of a bar as a part of the side of its cell: a tenth of the area
/// of the cell, so that the bars of the players in the same cell stand apart.
const BAR_SIDE: f32 = 0.316;

/// A bar never takes more than this part of its slot, there is a gap between the bars.
const BAR_SLOT_FILL: f32 = 0.8;

/// The glow of the bars through the bloom, and of the bars of a chosen unit type at the top of
/// their pulse, which lasts [`PULSE_SECONDS`].
const BAR_GLOW: f32 = 0.6;
const CHOSEN_GLOW: f32 = 2.4;
const PULSE_SECONDS: f32 = 1.6;

/// The shared material of the bars of each player and its color, the bars of a chosen unit type
/// pulse by changing these few materials, not the bars.
#[derive(Resource, Default)]
struct BarMaterials(Vec<(Handle<StandardMaterial>, LinearRgba)>);

/// The unit types shown in the units panel, the most frequent ones.
const MAX_PANEL_UNITS: usize = 24;

/// The color of the bars of a player, its color in the game.
pub fn player_color(color: [u8; 4]) -> Color {
    let [r, g, b, _] = color;
    Color::srgb_u8(r, g, b)
}

/// The size of the upgrade and unit icons in the players panel.
const ICON_SIZE: f32 = 32.;

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IconImages>()
            .init_resource::<BarSelection>()
            .init_resource::<RangeCircles>()
            .init_resource::<SelectedEntity>()
            .add_systems(Startup, (init_replay_time, spawn_bars))
            .add_systems(Startup, replay_panel.spawn())
            .add_systems(
                Update,
                (
                    step_time_with_keys,
                    update_time_text.run_if(resource_changed::<ReplayTime>),
                    // It checks whether the time or the selection changed.
                    update_players_panel,
                    // They check whether the time or the selection changed.
                    (update_bars, update_units_panel, update_bar_kind_text),
                    (update_range_circles, draw_range_circles).chain(),
                    pulse_chosen_bars,
                ),
            )
            .add_plugins(super::timeline::TimelinePlugin);
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

/// What the bars count: the units died or born or the kills, of every unit type or of one.
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
        kind_label(self.kind)
    }

    /// The count of a bar, i.e. `3 units died`.
    fn count_label(&self, count: u32) -> String {
        match self.kind {
            CellEventKind::Died => format!("{count} units died"),
            CellEventKind::Born => format!("{count} units born"),
            CellEventKind::Kill => format!("{count} kills"),
        }
    }
}

/// The caption of the dropdown and of its choices.
fn kind_label(kind: CellEventKind) -> &'static str {
    match kind {
        CellEventKind::Died => "Deaths",
        CellEventKind::Born => "Births",
        CellEventKind::Kill => "Kills",
    }
}

/// Where the bar of a player stands in its cell, from the center of the cell, and the side of its
/// base, in parts of the side of the cell. The cell has a slot per player, their number rounded up
/// to a power of 2 (3 players have 4 slots, 2 by 2): a player's bar is always in the same slot and
/// never hides another.
pub fn bar_slot(player: usize, players: usize) -> (Vec2, f32) {
    let slots = players.max(1).next_power_of_two();
    let columns = 1usize << slots.trailing_zeros().div_ceil(2);
    let rows = slots / columns;
    let (column, row) = (player % columns, (player / columns) % rows);
    let offset = Vec2::new(
        (column as f32 + 0.5) / columns as f32 - 0.5,
        (row as f32 + 0.5) / rows as f32 - 0.5,
    );
    let side = BAR_SIDE
        .min(BAR_SLOT_FILL / columns as f32)
        .min(BAR_SLOT_FILL / rows as f32);
    (offset, side)
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
pub(super) struct IconImages(HashMap<String, Option<Handle<Image>>>);

impl IconImages {
    pub(super) fn get(&mut self, icon: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
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
    let cell_side = AGGREGATE_CELLS as f32 * MAP_SCALE_FACTOR;
    let mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE)));
    let player_materials: Vec<_> = replay
        .players
        .iter()
        .map(|player| {
            let color = player_color(player.color);
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                // A little glow with the bloom.
                emissive: color.to_linear() * BAR_GLOW,
                ..default()
            }))
        })
        .collect();
    commands.insert_resource(BarMaterials(
        player_materials
            .iter()
            .zip(&replay.players)
            .map(|(material, player)| (material.0.clone(), player_color(player.color).to_linear()))
            .collect(),
    ));
    for (cell, player) in replay.cell_players() {
        let (world_x, world_z) = cell_center(cell);
        let Some(material) = player_materials.get(player) else {
            continue;
        };
        let (offset, side) = bar_slot(player, replay.players.len());
        commands
            .spawn((
                CellBar { cell, player },
                Name::new(""),
                DisplayInfoOnClick,
                mesh.clone(),
                material.clone(),
                Transform::from_xyz(
                    world_x + offset.x * cell_side,
                    terrain_top(FLAT_TERRAIN_LEVEL),
                    world_z + offset.y * cell_side,
                )
                .with_scale(Vec3::new(side * cell_side, 1., side * cell_side)),
                Visibility::Hidden,
            ))
            .observe(update_info_on_click)
            .observe(show_bar_units);
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
            "Cells ({x}, {y}) to ({}, {}) at {}: {player}, {}: {units}",
            x + AGGREGATE_CELLS - 1,
            y + AGGREGATE_CELLS - 1,
            game_time(seconds),
            selection.count_label(count),
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

fn show_deaths(_activate: On<Activate>, mut selection: ResMut<BarSelection>) {
    selection.kind = CellEventKind::Died;
}

fn show_births(_activate: On<Activate>, mut selection: ResMut<BarSelection>) {
    selection.kind = CellEventKind::Born;
}

fn show_kills(_activate: On<Activate>, mut selection: ResMut<BarSelection>) {
    selection.kind = CellEventKind::Kill;
}

/// The unit type of a units panel entry, clicking it shows only its bars, clicking it again shows
/// every unit type. The most effective unit of a player shows its kills.
#[derive(Component, Debug, Clone)]
struct UnitFilterButton {
    unit: String,
    kills: bool,
}

fn toggle_unit_filter(
    click: On<PointerClick>,
    buttons: Query<&UnitFilterButton>,
    parents: Query<&ChildOf>,
    mut selection: ResMut<BarSelection>,
) {
    // The click can be on the icon or the label of the entry.
    let Some(UnitFilterButton { unit, kills }) = std::iter::once(click.entity)
        .chain(parents.iter_ancestors(click.entity))
        .find_map(|entity| buttons.get(entity).ok())
    else {
        return;
    };
    if *kills && selection.kind != CellEventKind::Kill {
        selection.kind = CellEventKind::Kill;
        selection.unit = Some(unit.clone());
        return;
    }
    selection.unit = if selection.unit.as_ref() == Some(unit) {
        None
    } else {
        Some(unit.clone())
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
        for (image, unit, count) in units {
            let selected = selection.unit.as_deref() == Some(unit.as_str());
            let mut entry = panel.spawn((
                Node {
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(if selected { Color::WHITE } else { Color::NONE }),
                UnitFilterButton {
                    unit: unit.clone(),
                    kills: false,
                },
            ));
            entry.observe(toggle_unit_filter);
            entry.with_children(|entry| {
                spawn_icon(
                    entry,
                    image,
                    count.to_string(),
                    match selection.kind {
                        CellEventKind::Kill => {
                            format!("{unit}: {count} kills, click to show only its bars")
                        }
                        _ => format!("{unit}: {count}, click to show only its bars"),
                    },
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
            top: px(44),
            right: px(8),
            width: px(420),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            padding: px(8),
            row_gap: px(8),
        }
        // Above the upgrades timeline at the bottom when the window is low.
        GlobalZIndex(16)
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
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(16.),
                }
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
                    @caption: bsn! { Text("Deaths") ThemedText BarKindText }
                }
                AccessibleLabel("The bars count the deaths, the births or the kills")
                Node {
                    flex_grow: 1.0,
                }
                --
                @FeathersMenuPopup
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { Text("Deaths") ThemedText }
                    }
                    on(show_deaths)
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { Text("Births") ThemedText }
                    }
                    on(show_births)
                    --
                    @FeathersMenuItem {
                        @caption: bsn! { Text("Kills") ThemedText }
                    }
                    on(show_kills)
                ]
            ]
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

pub(super) fn small_text(text: impl Into<String>, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextColor(color),
        TextFont {
            font_size: bevy::prelude::FontSize::Px(13.),
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

/// The players, the winners first, then the losers, as in swarmy.
pub(super) fn player_order(replay: &ReplayData) -> Vec<usize> {
    let mut order: Vec<usize> = (0..replay.players.len()).collect();
    order.sort_by_key(|idx| match replay.players[*idx].result.as_str() {
        "Win" => 0,
        "Loss" => 1,
        _ => 2,
    });
    order
}

/// Rebuilds the players: their name in their color, the result, and the unit and the ability with
/// the most kills so far. Their upgrades are in the timeline at the bottom.
fn update_players_panel(
    mut commands: Commands,
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    selection: Res<BarSelection>,
    panel: Single<Entity, With<ReplayPlayersPanel>>,
    mut icons: ResMut<IconImages>,
    mut images: ResMut<Assets<Image>>,
    mut markers: Local<Option<ResultMarkers>>,
) {
    if !time.is_changed() && !selection.is_changed() {
        return;
    }
    let markers = markers.get_or_insert_with(|| ResultMarkers::new(&mut images));
    let seconds = time.seconds();
    let order = player_order(&replay);
    let win_marker = markers.win.clone();
    let loss_marker = markers.loss.clone();
    commands.entity(*panel).despawn_related::<Children>();
    commands.entity(*panel).with_children(|panel| {
        for idx in order {
            let player = &replay.players[idx];
            let [r, g, b, a] = player.color;
            let color = Color::srgba_u8(r, g, b, a);
            // A green triangle up for a winner, a red one down for a loser, as in swarmy.
            let marker = match player.result.as_str() {
                "Win" => Some(win_marker.clone()),
                "Loss" => Some(loss_marker.clone()),
                _ => None,
            };
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
                        if let Some(marker) = marker.clone() {
                            header.spawn((
                                ImageNode::new(marker),
                                Node {
                                    width: px(14),
                                    height: px(14),
                                    ..default()
                                },
                            ));
                        }
                        header.spawn((
                            Node {
                                width: px(14),
                                height: px(14),
                                ..default()
                            },
                            BackgroundColor(color),
                        ));
                        header.spawn((
                            Text::new(player.name.clone()),
                            TextColor(Color::WHITE),
                            TextFont {
                                font_size: bevy::prelude::FontSize::Px(16.),
                                ..default()
                            },
                        ));
                        header.spawn(small_text(
                            player.race.clone(),
                            Color::from(palettes::tailwind::GRAY_300),
                        ));
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
                            // Clicking it shows the bars of its kills, as the units panel does.
                            let chosen = selection.kind == CellEventKind::Kill
                                && selection.unit.as_deref() == Some(unit.as_str());
                            effective
                                .spawn((
                                    Node {
                                        border: UiRect::all(px(1)),
                                        ..default()
                                    },
                                    BorderColor::all(if chosen {
                                        Color::WHITE
                                    } else {
                                        Color::NONE
                                    }),
                                    UnitFilterButton {
                                        unit: unit.clone(),
                                        kills: true,
                                    },
                                ))
                                .observe(toggle_unit_filter)
                                .with_children(|entry| {
                                    spawn_icon(
                                        entry,
                                        image,
                                        format!("{kills} kills"),
                                        format!(
                                            "Most effective unit: {unit}, {kills} kills, click \
                                             to show where it killed"
                                        ),
                                    );
                                });
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
                    });
                });
        }
    });
}

/// The triangles of a winner and of a loser: the default font has no arrows, they are drawn.
struct ResultMarkers {
    win: Handle<Image>,
    loss: Handle<Image>,
}

impl ResultMarkers {
    fn new(images: &mut Assets<Image>) -> Self {
        Self {
            win: images.add(triangle_image(Pointing::Up, palettes::tailwind::GREEN_500)),
            loss: images.add(triangle_image(Pointing::Down, palettes::tailwind::RED_500)),
        }
    }
}

/// Clicking a bar shows its units with their icons in the selection panel.
#[allow(clippy::too_many_arguments)]
fn show_bar_units(
    click: On<PointerClick>,
    mut commands: Commands,
    bars: Query<&CellBar>,
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    selection: Res<BarSelection>,
    selection_icons: Single<Entity, With<SelectionIcons>>,
    mut icons: ResMut<IconImages>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(bar) = bars.get(click.entity) else {
        return;
    };
    let unit = selection.unit.as_deref();
    let units: Vec<_> = replay
        .unit_counts(time.seconds(), selection.kind, Some((bar.cell, bar.player)))
        .into_iter()
        .filter(|(name, _)| unit.is_none_or(|unit| unit == name))
        .map(|(name, count)| {
            let image = replay
                .unit_icons
                .get(&name)
                .and_then(|icon| icons.get(icon, &mut images));
            (image, name, count)
        })
        .collect();
    commands.entity(*selection_icons).with_children(|panel| {
        for (image, name, count) in units {
            spawn_icon(panel, image, count.to_string(), format!("{name}: {count}"));
        }
    });
}

/// The range of the units that killed, of the clicked bar or of the unit type chosen in the units
/// panel, while the bars count the kills.
#[derive(Resource, Debug, Default)]
struct RangeCircles(Vec<RangeCircle>);

fn update_range_circles(
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    selection: Res<BarSelection>,
    selected: Res<SelectedEntity>,
    bars: Query<&CellBar>,
    mut circles: ResMut<RangeCircles>,
) {
    if !time.is_changed() && !selection.is_changed() && !selected.is_changed() {
        return;
    }
    let unit = selection.unit.as_deref();
    let bar = selected.0.and_then(|entity| bars.get(entity).ok());
    circles.0 = match (selection.kind, bar) {
        (CellEventKind::Kill, Some(bar)) => {
            replay.range_circles(time.seconds(), unit, Some((bar.cell, bar.player)))
        }
        (CellEventKind::Kill, None) if unit.is_some() => {
            replay.range_circles(time.seconds(), unit, None)
        }
        _ => vec![],
    };
}

/// The bars of the unit type chosen in the panel breathe: their glow rises and falls through the
/// bloom so that they stand out, back to the usual glow when no unit type is chosen.
fn pulse_chosen_bars(
    time: Res<Time>,
    selection: Res<BarSelection>,
    bars: Option<Res<BarMaterials>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pulsing: Local<bool>,
) {
    let Some(bars) = bars else {
        return;
    };
    let chosen = selection.unit.is_some();
    if !chosen && !*pulsing {
        return;
    }
    *pulsing = chosen;
    let glow = if chosen {
        let phase = time.elapsed_secs() / PULSE_SECONDS * std::f32::consts::TAU;
        BAR_GLOW + (CHOSEN_GLOW - BAR_GLOW) * (0.5 - 0.5 * phase.cos())
    } else {
        BAR_GLOW
    };
    for (handle, color) in &bars.0 {
        if let Some(mut material) = materials.get_mut(handle) {
            material.emissive = *color * glow;
        }
    }
}

/// A map position on the flattened terrain, a little above it, in world coordinates.
fn ground_point(map: (f32, f32)) -> Vec3 {
    Vec3::new(
        map.1 * MAP_SCALE_FACTOR,
        terrain_top(FLAT_TERRAIN_LEVEL) + 0.01,
        map.0 * MAP_SCALE_FACTOR,
    )
}

/// The circles in a light shade of the color of the player, inside a dark one, so that they are
/// seen on the light and the dark grays of the terrain; the unit is a small circle, a Liberator is
/// joined to its zone.
fn draw_range_circles(circles: Res<RangeCircles>, replay: Res<ReplayData>, mut gizmos: Gizmos) {
    let flat = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let halo = Color::from(palettes::tailwind::NEUTRAL_950);
    for circle in &circles.0 {
        let Some(player) = replay.players.get(circle.player) else {
            continue;
        };
        let color = range_color(player.color);
        let center = ground_point(circle.center);
        let radius = circle.radius * MAP_SCALE_FACTOR;
        gizmos
            .circle(Isometry3d::new(center, flat), radius + 0.015, halo)
            .resolution(96);
        gizmos
            .circle(Isometry3d::new(center, flat), radius, color)
            .resolution(96);
        let unit = ground_point(circle.unit);
        gizmos.circle(Isometry3d::new(unit, flat), 0.06, color);
        if circle.unit != circle.center {
            gizmos.line(unit, center, color);
        }
    }
}

/// The color of the player mixed with 35% of white, lighter than the bars on the dark terrain.
pub fn range_color(color: [u8; 4]) -> Color {
    let [r, g, b, _] = color;
    let mix = |channel: u8| (channel as f32 + (255. - channel as f32) * 0.35) / 255.;
    Color::srgb(mix(r), mix(g), mix(b))
}

#[cfg(test)]
mod slot_tests {
    use super::*;

    #[test]
    fn it_gives_each_player_its_own_slot() {
        // Alone, a bar is in the middle of its cell, a tenth of its area.
        let (offset, side) = bar_slot(0, 1);
        assert_eq!(offset, Vec2::ZERO);
        assert!((side * side - 0.1).abs() < 0.001);
        // 3 players have 4 slots, 2 by 2, apart from each other.
        let slots: Vec<_> = (0..3).map(|player| bar_slot(player, 3)).collect();
        assert_eq!(slots[0].0, Vec2::new(-0.25, -0.25));
        assert_eq!(slots[1].0, Vec2::new(0.25, -0.25));
        assert_eq!(slots[2].0, Vec2::new(-0.25, 0.25));
        for (offset, side) in &slots {
            assert!(offset.x.abs() + side / 2. <= 0.5);
        }
        // 8 players, 4 by 2, the bars fit in their slots.
        let (offset, side) = bar_slot(7, 8);
        assert_eq!(offset, Vec2::new(0.375, 0.25));
        assert!(side <= 0.25 * BAR_SLOT_FILL + f32::EPSILON);
    }
}
