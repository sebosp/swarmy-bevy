//! Bevy Feathers for Swarmy
//!
use bevy::ecs::entity_disabling::Disabled;
use bevy::feathers::tokens;
use bevy::feathers::{controls::*, theme::ThemeBackgroundColor, theme::ThemedText};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{Activate, ValueChange};

use crate::SelectedObjectName;
use crate::cache_objects::doodas::ObjectDoodadComponent;
use crate::cache_objects::{ObjectPointComponent, ObjectUnitComponent};
use crate::t3_terrain::RampComponent;

/// Allows a component to be observed for click events and show display information on a text feather.
#[derive(Component)]
pub struct DisplayInfoOnClick;

pub fn init_feathers() -> impl Scene {
    bsn! {
        Node {
            top: px(0),
            left: px(0),
            width: percent(100),
            height: px(8),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            @feather_column_1()
        ]
    }
}

fn feather_column_1() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(100),
            min_width: px(200),
        }
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Start,
                column_gap: px(8),
            }
            Children [
                @main_feather_menu()
            ]
        ]
    }
}

/// The panel of what was clicked, at the bottom left, hidden while nothing is selected: the name
/// of the object, or of the bar with the icons of its units.
#[derive(Component, Default, Clone)]
pub struct SelectionPane;

/// The icons of the units of a clicked bar, in the selection panel.
#[derive(Component, Default, Clone)]
pub struct SelectionIcons;

pub fn selection_pane() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            max_width: px(560),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            padding: px(10),
            row_gap: px(8),
        }
        SelectionPane
        GlobalZIndex(15)
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                column_gap: px(12),
            }
            Children [
                Text("") ThemedText SelectedObjectName
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(15.),
                }
                Node {
                    flex_grow: 1.0,
                }
                --
                @FeathersButton {
                    @caption: bsn! { Text("Close") ThemedText }
                }
                AccessibleLabel("Close the selection")
                on(close_selection)
            ]
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
            }
            SelectionIcons
        ]
    }
}

/// Empties the selection, the panel hides.
pub fn clear_selection(commands: &mut Commands, text: &mut Text, icons: Entity) {
    text.0.clear();
    commands.entity(icons).despawn_related::<Children>();
}

fn close_selection(
    _activate: On<Activate>,
    mut commands: Commands,
    mut text: Single<&mut Text, With<SelectedObjectName>>,
    icons: Single<Entity, With<SelectionIcons>>,
) {
    clear_selection(&mut commands, &mut text, *icons);
}

/// Escape closes the selection.
pub fn close_selection_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut text: Single<&mut Text, With<SelectedObjectName>>,
    icons: Single<Entity, With<SelectionIcons>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        clear_selection(&mut commands, &mut text, *icons);
    }
}

/// The selection panel is shown while something is selected.
pub fn show_selection_pane(
    text: Single<&Text, (With<SelectedObjectName>, Changed<Text>)>,
    mut pane: Single<&mut Node, With<SelectionPane>>,
) {
    pane.display = if text.0.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
}

fn main_feather_menu() -> impl Scene {
    bsn! {
        @FeathersMenu
        Children [
            @FeathersMenuButton {
                @caption: bsn! { Text("View") ThemedText }
            }
            AccessibleLabel("View Menu")
            Node {
                flex_grow: 1.0,
            }
            --
            @FeathersMenuPopup
            Node {
                height: px(10),
            }
            Children [
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Enable ObjectDoodas") ThemedText }
                            }
                            Checked
                            on(|
                                value_change: On<ValueChange<bool>>,
                                commands: Commands,
                                query: Query<Entity, (With<ObjectDoodadComponent>, Allow<Disabled>)>, | {
                                toggle_entities(value_change, commands, query)
                            })
                            --
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Enable ObjectUnits") ThemedText }
                            }
                            Checked
                            on(|
                                value_change: On<ValueChange<bool>>,
                                commands: Commands,
                                query: Query<Entity, (With<ObjectUnitComponent>, Allow<Disabled>)>, | {
                                toggle_entities(value_change, commands, query)
                            })
                            --
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Enable ObjectPoints") ThemedText }
                            }
                            Checked
                            on(|
                                value_change: On<ValueChange<bool>>,
                                commands: Commands,
                                query: Query<Entity, (With<ObjectPointComponent>, Allow<Disabled>)>, | {
                                toggle_entities(value_change, commands, query)
                            })
                            --
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Enable Ramps") ThemedText }
                            }
                            Checked
                            on(|
                                value_change: On<ValueChange<bool>>,
                                commands: Commands,
                                query: Query<Entity, (With<RampComponent>, Allow<Disabled>)>, | {
                                toggle_entities(value_change, commands, query)
                            })
                --
                @FeathersMenuDivider
                --
                @FeathersMenuItem {
                    @caption: bsn! { Text("Second section") ThemedText }
                }
            ]
        ]
    }
}

/// Shows or hides the entities of a View menu checkbox, the hidden entities are disabled.
pub fn toggle_entities<F: bevy::ecs::query::QueryFilter>(
    value_change: On<ValueChange<bool>>,
    mut commands: Commands,
    query: Query<Entity, F>,
) {
    if value_change.value {
        commands.entity(value_change.source).insert(Checked);
        for entity in query.iter() {
            commands.entity(entity).remove::<Disabled>();
        }
    } else {
        commands.entity(value_change.source).remove::<Checked>();
        for entity in query.iter() {
            commands.entity(entity).insert(Disabled);
        }
    }
}

pub fn update_info_on_click(
    click: On<PointerClick>,
    mut commands: Commands,
    display_info_query: Query<(&Name, Entity), With<DisplayInfoOnClick>>,
    mut selected_object: Single<&mut Text, With<SelectedObjectName>>,
    icons: Single<Entity, With<SelectionIcons>>,
) {
    // The icons of a previous bar go, a bar adds its own after this.
    commands.entity(*icons).despawn_related::<Children>();
    for (name, entity) in &display_info_query {
        if entity == click.entity {
            info!(
                "Valid Query on on name {}, but clicked on {}",
                name, click.entity
            );
            selected_object.0 = format!("{}", name);
        }
    }
}
