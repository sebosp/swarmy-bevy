//! The upgrades of the players on a timeline at the bottom of the window, a section that folds:
//! a lane per player (its color and name at the left), the icon of each upgrade where it finished,
//! the minutes of the game under the lanes and a line at the time shown. The upgrades after the
//! time shown are faded. Hovering an icon shows its name, time and player.

use super::ReplayData;
use super::game_time;
use super::plugin::{IconImages, ReplayTime, player_color, player_order, small_text};
use crate::collapsible::{
    ARROW_SIZE, CollapseArrow, Collapsible, CollapsibleBody, CollapsibleHeader, toggle_collapsible,
};
use bevy::color::palettes;
use bevy::feathers::theme::ThemeBackgroundColor;
use bevy::feathers::tokens;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};

/// The size of the upgrade icons in the lanes.
const ICON: f32 = 20.;
/// The rows of icons of a lane at most, the icons too close for them overlap: the timeline stays
/// low enough for the replay panel above it on a laptop.
const MAX_ROWS: usize = 2;
/// The axis ends this long after the last upgrade (or at the end of the game): the upgrades are
/// in the first half of a long game.
const AFTER_LAST_UPGRADE: u32 = 120;
/// The space between two icons, and between two rows of icons of a lane.
const ICON_GAP: f32 = 2.;
/// The width of the names of the players at the left of the lanes.
const LABEL_WIDTH: f32 = 150.;
/// The timeline takes the width of the window right of the selection panel.
const LEFT_PERCENT: f32 = 20.;
/// The fewest pixels between two minute ticks.
const MIN_TICK_PX: f32 = 64.;

/// The lines of the timeline, recessive: the minute grid, and the time shown, light.
const GRID_COLOR: Color = Color::srgb(0.235, 0.235, 0.255);
const CURSOR_COLOR: Color = Color::srgb(0.941, 0.937, 0.925);
/// The ticks and labels of the axis.
const AXIS_TEXT: Srgba = palettes::tailwind::GRAY_400;
/// The upgrades after the time shown, faded.
const FUTURE_ALPHA: f32 = 0.3;

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_timeline)
            .add_systems(Update, (rebuild_timeline, follow_cursor));
    }
}

/// The lanes and the axis, rebuilt when the time or the size of the window changes.
#[derive(Component)]
struct TimelineLanes;

/// The name, time and player of the hovered upgrade.
#[derive(Component)]
struct TimelineTooltip;

#[derive(Component)]
struct TooltipText;

/// What the tooltip of an upgrade icon says.
#[derive(Component, Clone)]
struct UpgradeTip(String);

fn spawn_timeline(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(8),
                left: percent(LEFT_PERCENT),
                right: px(8),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(8)),
                row_gap: px(6),
                ..default()
            },
            Collapsible::default(),
            GlobalZIndex(14),
            ThemeBackgroundColor(tokens::WINDOW_BG),
            Name::new("Upgrades timeline"),
        ))
        .with_children(|section| {
            section
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    },
                    CollapsibleHeader,
                ))
                .observe(toggle_collapsible)
                .with_children(|header| {
                    header.spawn((
                        Node {
                            width: px(ARROW_SIZE),
                            height: px(ARROW_SIZE),
                            ..default()
                        },
                        CollapseArrow,
                    ));
                    header.spawn(small_text(
                        "Upgrades",
                        Color::from(palettes::tailwind::GRAY_200),
                    ));
                });
            section.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                CollapsibleBody,
                TimelineLanes,
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(8), px(6)),
                row_gap: px(2),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::from(palettes::tailwind::GRAY_500)),
            BackgroundColor(Color::srgb(0.11, 0.11, 0.13)),
            GlobalZIndex(40),
            Pickable::IGNORE,
            TimelineTooltip,
        ))
        .with_child((
            small_text("", Color::from(palettes::tailwind::GRAY_100)),
            Pickable::IGNORE,
            TooltipText,
        ));
}

/// The spacing of the minute ticks: the shortest that leaves [`MIN_TICK_PX`] between them.
fn tick_seconds(duration: u32, track_px: f32) -> u32 {
    const STEPS: [u32; 8] = [60, 120, 300, 600, 900, 1200, 1800, 3600];
    let px_per_second = track_px / duration.max(1) as f32;
    STEPS
        .into_iter()
        .find(|step| *step as f32 * px_per_second >= MIN_TICK_PX)
        .unwrap_or(3600)
}

/// The row of each upgrade in its lane, by time: an icon goes in the first row where it does not
/// cover the previous one, in the row where it covers the least when there are [`MAX_ROWS`].
/// Returns the rows and their number.
fn pack_rows(seconds: &[u32], duration: u32, track_px: f32) -> (Vec<usize>, usize) {
    let px_per_second = track_px / duration.max(1) as f32;
    let mut row_ends: Vec<f32> = vec![];
    let rows = seconds
        .iter()
        .map(|seconds| {
            let x = *seconds as f32 * px_per_second;
            let free = row_ends.iter().position(|end| x >= *end);
            let row = match free {
                Some(row) => row,
                None if row_ends.len() < MAX_ROWS => row_ends.len(),
                None => (0..row_ends.len())
                    .min_by(|a, b| row_ends[*a].total_cmp(&row_ends[*b]))
                    .unwrap_or(0),
            };
            if row == row_ends.len() {
                row_ends.push(0.);
            }
            row_ends[row] = x + ICON + ICON_GAP;
            row
        })
        .collect();
    (rows, row_ends.len().max(1))
}

/// The name of an upgrade with spaces, `TerranInfantryWeaponsLevel1` is `Terran Infantry Weapons
/// Level 1`.
pub fn upgrade_label(name: &str) -> String {
    let mut label = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if let Some(previous) = previous {
            let word_starts = (c.is_uppercase() && previous.is_lowercase())
                || (c.is_ascii_digit() && !previous.is_ascii_digit());
            if word_starts {
                label.push(' ');
            }
        }
        label.push(c);
        previous = Some(c);
    }
    label
}

fn at_percent(seconds: u32, duration: u32) -> Val {
    percent(seconds.min(duration) as f32 / duration.max(1) as f32 * 100.)
}

#[allow(clippy::too_many_arguments)]
fn rebuild_timeline(
    mut commands: Commands,
    replay: Res<ReplayData>,
    time: Res<ReplayTime>,
    ui_scale: Res<UiScale>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut resized: MessageReader<WindowResized>,
    lanes: Single<Entity, With<TimelineLanes>>,
    mut icons: ResMut<IconImages>,
    mut images: ResMut<Assets<Image>>,
) {
    let resized = resized.read().count() > 0;
    if !time.is_changed() && !resized && !ui_scale.is_changed() {
        return;
    }
    let last_upgrade = replay.upgrades.iter().map(|upgrade| upgrade.seconds).max();
    let duration = last_upgrade
        .map(|last| last + AFTER_LAST_UPGRADE)
        .unwrap_or(u32::MAX)
        .min(replay.duration_seconds)
        .max(1);
    let now = time.seconds();
    // The width of the tracks, the window less the selection panel, the names and the paddings.
    let track_px =
        (window.width() / ui_scale.0 * (1. - LEFT_PERCENT / 100.) - LABEL_WIDTH - ICON - 32.)
            .max(100.);
    let tick = tick_seconds(duration, track_px);
    let ticks: Vec<u32> = (tick..duration).step_by(tick as usize).collect();
    let track_margin = UiRect::horizontal(px(ICON / 2.));
    let mut lanes_data = vec![];
    for player in player_order(&replay) {
        let upgrades: Vec<_> = replay
            .upgrades
            .iter()
            .filter(|upgrade| upgrade.player == player)
            .collect();
        let seconds: Vec<u32> = upgrades.iter().map(|upgrade| upgrade.seconds).collect();
        let (rows, row_count) = pack_rows(&seconds, duration, track_px);
        let icons: Vec<_> = upgrades
            .iter()
            .zip(rows)
            .map(|(upgrade, row)| {
                let image = upgrade
                    .icon
                    .as_deref()
                    .and_then(|icon| icons.get(icon, &mut images));
                (image, *upgrade, row)
            })
            .collect();
        lanes_data.push((player, icons, row_count));
    }
    commands.entity(*lanes).despawn_related::<Children>();
    commands.entity(*lanes).with_children(|lanes| {
        for (player, icons, row_count) in lanes_data {
            let info = &replay.players[player];
            let lane_height = row_count as f32 * (ICON + ICON_GAP);
            lanes
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|lane| {
                    lane.spawn(Node {
                        width: px(LABEL_WIDTH),
                        flex_shrink: 0.,
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_children(|label| {
                        label.spawn((
                            Node {
                                width: px(10),
                                height: px(10),
                                flex_shrink: 0.,
                                ..default()
                            },
                            BackgroundColor(player_color(info.color)),
                        ));
                        label.spawn(small_text(info.name.clone(), Color::WHITE));
                    });
                    lane.spawn(Node {
                        flex_grow: 1.,
                        height: px(lane_height),
                        margin: track_margin,
                        ..default()
                    })
                    .with_children(|track| {
                        for tick in &ticks {
                            track.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: at_percent(*tick, duration),
                                    width: px(1),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(GRID_COLOR),
                                Pickable::IGNORE,
                            ));
                        }
                        for (image, upgrade, row) in icons {
                            let future = upgrade.seconds > now;
                            let tint =
                                Color::WHITE.with_alpha(if future { FUTURE_ALPHA } else { 1. });
                            let node = Node {
                                position_type: PositionType::Absolute,
                                left: at_percent(upgrade.seconds, duration),
                                top: px(row as f32 * (ICON + ICON_GAP)),
                                width: px(ICON),
                                height: px(ICON),
                                margin: UiRect::left(px(-ICON / 2.)),
                                ..default()
                            };
                            let tip = UpgradeTip(format!(
                                "{}\n{} at {}",
                                upgrade_label(&upgrade.name),
                                info.name,
                                game_time(upgrade.seconds)
                            ));
                            let mut icon = match image {
                                Some(image) => {
                                    track.spawn((node, ImageNode::new(image).with_color(tint), tip))
                                }
                                None => track.spawn((
                                    node,
                                    BackgroundColor(
                                        Color::from(palettes::tailwind::GRAY_600)
                                            .with_alpha(if future { FUTURE_ALPHA } else { 1. }),
                                    ),
                                    tip,
                                )),
                            };
                            icon.observe(show_tooltip).observe(hide_tooltip);
                        }
                        if now > duration {
                            return;
                        }
                        track.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: at_percent(now, duration),
                                width: px(2),
                                height: percent(100),
                                margin: UiRect::left(px(-1)),
                                ..default()
                            },
                            BackgroundColor(CURSOR_COLOR),
                            Pickable::IGNORE,
                        ));
                    });
                });
        }
        // The minutes under the lanes.
        lanes
            .spawn(Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..default()
            })
            .with_children(|axis| {
                axis.spawn(Node {
                    width: px(LABEL_WIDTH),
                    flex_shrink: 0.,
                    ..default()
                });
                axis.spawn(Node {
                    flex_grow: 1.,
                    height: px(16),
                    margin: track_margin,
                    ..default()
                })
                .with_children(|track| {
                    for tick in &ticks {
                        track
                            .spawn(Node {
                                position_type: PositionType::Absolute,
                                left: at_percent(*tick, duration),
                                // Centered on the tick, wide enough for 00:00.
                                width: px(40),
                                margin: UiRect::left(px(-20)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            })
                            .with_child(small_text(game_time(*tick), Color::from(AXIS_TEXT)));
                    }
                });
            });
    });
}

fn show_tooltip(
    over: On<PointerOver>,
    tips: Query<&UpgradeTip>,
    mut tooltip: Single<&mut Node, With<TimelineTooltip>>,
    mut text: Single<&mut Text, With<TooltipText>>,
) {
    let Ok(tip) = tips.get(over.entity) else {
        return;
    };
    text.0 = tip.0.clone();
    tooltip.display = Display::Flex;
}

fn hide_tooltip(_out: On<PointerOut>, mut tooltip: Single<&mut Node, With<TimelineTooltip>>) {
    tooltip.display = Display::None;
}

/// The tooltip follows the mouse, above it: the timeline is at the bottom of the window.
fn follow_cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut tooltip: Single<(&mut Node, &ComputedNode), With<TimelineTooltip>>,
) {
    let (node, computed) = &mut *tooltip;
    if node.display == Display::None {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let scale = ui_scale.0.max(0.1);
    let size = computed.size() * computed.inverse_scale_factor();
    let width = window.width() / scale;
    let x = (cursor.x / scale + 12.).min(width - size.x - 4.).max(4.);
    let y = (cursor.y / scale - size.y - 12.).max(4.);
    node.left = px(x);
    node.top = px(y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_labels_the_upgrades() {
        assert_eq!(
            upgrade_label("TerranInfantryWeaponsLevel1"),
            "Terran Infantry Weapons Level 1"
        );
        assert_eq!(upgrade_label("Stimpack"), "Stimpack");
    }

    #[test]
    fn it_packs_the_close_upgrades_in_rows() {
        // 1 pixel a second: two upgrades 10 s apart overlap, 30 s apart do not.
        let (rows, count) = pack_rows(&[100, 110, 140, 200], 1000, 1000.);
        assert_eq!(rows, vec![0, 1, 0, 0]);
        assert_eq!(count, 2);
        // A third close one overlaps the row that ends first.
        let (rows, count) = pack_rows(&[100, 105, 110], 1000, 1000.);
        assert_eq!(rows, vec![0, 1, 0]);
        assert_eq!(count, MAX_ROWS);
        assert_eq!(pack_rows(&[], 1000, 1000.).1, 1);
    }

    #[test]
    fn it_spaces_the_minute_ticks() {
        // 20 minutes on 1000 px: 50 px a minute is too close, 2 minutes is 100 px.
        assert_eq!(tick_seconds(1200, 1000.), 120);
        assert_eq!(tick_seconds(600, 2000.), 60);
    }
}
