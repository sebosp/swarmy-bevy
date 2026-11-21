//! Bevy Feathers for Swarmy
//!
use bevy::ecs::entity_disabling::Disabled;
use bevy::feathers::tokens;
use bevy::feathers::{controls::*, theme::ThemeBackgroundColor, theme::ThemedText};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::ValueChange;

use crate::cache_objects::doodas::ObjectDoodadComponent;
use crate::cache_objects::{ObjectPointComponent, ObjectUnitComponent};
use crate::collapsible::{
    ARROW_SIZE, CollapseArrow, Collapsible, CollapsibleBody, CollapsibleHeader, toggle_collapsible,
};
use crate::map_graphics::{MapGraphicComponent, MapGraphicsState};
use crate::pathing::{PathingCheckbox, toggle_pathing};
use crate::t3_terrain::RampComponent;
use crate::terrain_textures::{TerrainTexturesCheckbox, TerrainTexturesState};
use crate::{SelectedEntity, SelectedObjectName};

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
/// of the object, or of the bar with the icons of its units. It folds with its header and takes at
/// most a fifth of the width of the window, the upgrades timeline takes the rest.
#[derive(Component, Default, Clone)]
pub struct SelectionPane;

/// The icons of the units of a clicked bar, in the selection panel.
#[derive(Component, Default, Clone)]
pub struct SelectionIcons;

pub fn selection_pane() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: px(8),
            left: px(8),
            // A width, not a max_width: the text wrapped in a max_width overflowed the panel.
            width: percent(19),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            padding: px(8),
            row_gap: px(6),
        }
        SelectionPane
        Collapsible { collapsed: false }
        GlobalZIndex(15)
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
            }
            CollapsibleHeader
            AccessibleLabel("Fold or unfold the selection")
            on(toggle_collapsible)
            Children [
                Node {
                    width: px(ARROW_SIZE),
                    height: px(ARROW_SIZE),
                }
                CollapseArrow
                --
                Text("Selection") ThemedText
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(13.),
                }
            ]
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
            }
            CollapsibleBody
            Children [
                Text("") ThemedText SelectedObjectName
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(14.),
                }
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
        ]
    }
}

/// Empties the selection, the panel hides.
pub fn clear_selection(
    commands: &mut Commands,
    text: &mut Text,
    icons: Entity,
    selected: &mut SelectedEntity,
) {
    text.0.clear();
    selected.0 = None;
    commands.entity(icons).despawn_related::<Children>();
}

/// Escape closes the selection.
pub fn close_selection_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut text: Single<&mut Text, With<SelectedObjectName>>,
    icons: Single<Entity, With<SelectionIcons>>,
    mut selected: ResMut<SelectedEntity>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        clear_selection(&mut commands, &mut text, *icons, &mut selected);
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
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Enable Map Graphics") ThemedText }
                            }
                            on(toggle_map_graphics)
                            --
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Terrain Textures") ThemedText }
                            }
                            TerrainTexturesCheckbox
                            on(toggle_terrain_textures)
                            --
                            @FeathersCheckbox {
                                @caption: bsn! { Text("Pathing") ThemedText }
                            }
                            PathingCheckbox
                            on(toggle_pathing)
            ]
        ]
    }
}

/// Shows or hides the models and textures of the map caches, they are read the first time.
fn toggle_map_graphics(
    value_change: On<ValueChange<bool>>,
    commands: Commands,
    query: Query<Entity, (With<MapGraphicComponent>, Allow<Disabled>)>,
    mut state: ResMut<MapGraphicsState>,
) {
    state.enabled = value_change.value;
    toggle_entities(value_change, commands, query);
}

/// Paints the terrain with its textures or in its colors, they are read the first time.
fn toggle_terrain_textures(
    value_change: On<ValueChange<bool>>,
    mut commands: Commands,
    mut state: ResMut<TerrainTexturesState>,
) {
    state.enabled = value_change.value;
    if value_change.value {
        commands.entity(value_change.source).insert(Checked);
    } else {
        commands.entity(value_change.source).remove::<Checked>();
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
    mut selected: ResMut<SelectedEntity>,
) {
    // The icons of a previous bar go, a bar adds its own after this.
    commands.entity(*icons).despawn_related::<Children>();
    selected.0 = Some(click.entity);
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
