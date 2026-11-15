//! The buttons at the top right of the window: Help, what the keys do, what the colors mean and
//! how to move on the map, and Settings, the size of the text and of the panels.

use crate::replay::ReplayData;
use crate::view_settings::ViewSettings;
use bevy::feathers::controls::*;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemedText};
use bevy::feathers::tokens;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

/// The panel of Help, hidden until its button is pressed.
#[derive(Component, Default, Clone)]
pub struct HelpPanel;

/// The text of Help, written for what the window shows.
#[derive(Component, Default, Clone)]
pub struct HelpText;

/// The panel of Settings.
#[derive(Component, Default, Clone)]
pub struct SettingsPanel;

/// The size of the text shown in Settings.
#[derive(Component, Default, Clone)]
pub struct ScaleText;

pub struct HelpUiPlugin;

impl Plugin for HelpUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (toolbar.spawn(), write_help).chain())
            .add_systems(Update, (show_scale, close_on_escape));
    }
}

/// What the window shows, for the text of Help.
pub struct HelpContext {
    pub replay: bool,
}

/// The text of Help: moving on the map, the keys and the colors of what is shown.
pub fn help_text(context: &HelpContext) -> String {
    let mut lines = vec![
        "Moving on the map".to_string(),
        "  W A S D: move, Shift to go faster, Space (or E): up, Q: down".to_string(),
        "  Right drag: look around, the wheel changes the speed".to_string(),
        "  M: lock or unlock the mouse".to_string(),
        String::new(),
        "View menu".to_string(),
        "  Enable Map Graphics: the models and textures stored with the map, where the map"
            .to_string(),
        "  places them. Models of the game itself are not stored there and are not drawn; a"
            .to_string(),
        "  texture of the game is drawn as a free one of the same kind (rock, grass...), and"
            .to_string(),
        "  the flat images (decals) without theirs are left out.".to_string(),
        "  Terrain Textures: the ground painted with the textures of the map: the weights the"
            .to_string(),
        "  map gives to each of its 8 textures at each point, each texture drawn as a free one"
            .to_string(),
        "  of the same kind (grass, dirt, rock...) tinted with its color in the minimap. On for a"
            .to_string(),
        "  map, off for a replay. The geysers and the ore of the minerals are then of rock."
            .to_string(),
        "  Textures: Poly Haven (polyhaven.com), CC0.".to_string(),
        "  The mineral fields are ores with glowing crystals, blue (gold the rich ones), that"
            .to_string(),
        "  twinkle; the geysers have a green vent puffing their gas; the start locations are"
            .to_string(),
        "  landing pads with lights running around them.".to_string(),
        String::new(),
        "Keys".to_string(),
    ];
    if context.replay {
        lines.push(
            "  Left and Right arrows: back and forward in time, Home and End: start and end"
                .to_string(),
        );
    }
    lines.extend([
        "  2: show or hide the circles of the units".to_string(),
        "  B: show or hide the boxes around the objects".to_string(),
        "  Ctrl+= and Ctrl+-: bigger or smaller text, Ctrl+0: the default size".to_string(),
        "  Escape: close this panel".to_string(),
        String::new(),
        "Colors".to_string(),
    ]);
    if context.replay {
        lines.push(
            "  The terrain is in grays, darker the lower, so that the bars stand out".to_string(),
        );
    } else {
        lines.push(
            "  The terrain by level (Terrain Textures off): black the lowest, then light blue, light yellow, green and dark red the highest; the ramps in amber".to_string(),
        );
    }
    if context.replay {
        lines.extend([
            "  Replay: a bar counts the units died (or born, from the dropdown) per 4x4 cells up to the time shown, in the color of their player; two players in the same cell share it".to_string(),
            "  Click a unit in the panel at the top right to show only its bars, click it again for all of them".to_string(),
            "  Players: the winner first, a green triangle up, the loser a red triangle down".to_string(),
        ]);
    }
    lines.extend([
        String::new(),
        "Click a bar or an object to see what it is at the bottom left, with the units of a bar; Close or Escape hides it.".to_string(),
    ]);
    lines.join("\n")
}

fn write_help(mut text: Single<&mut Text, With<HelpText>>, replay: Option<Res<ReplayData>>) {
    text.0 = help_text(&HelpContext {
        replay: replay.is_some(),
    });
}

/// Shows a panel and hides the other one, or hides it when it is shown.
fn toggle_panel<Shown: Component, Other: Component>(
    shown: &mut Query<&mut Node, (With<Shown>, Without<Other>)>,
    other: &mut Query<&mut Node, (With<Other>, Without<Shown>)>,
) {
    for mut node in shown.iter_mut() {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in other.iter_mut() {
        node.display = Display::None;
    }
}

fn toggle_help(
    _activate: On<Activate>,
    mut help: Query<&mut Node, (With<HelpPanel>, Without<SettingsPanel>)>,
    mut settings: Query<&mut Node, (With<SettingsPanel>, Without<HelpPanel>)>,
) {
    toggle_panel(&mut help, &mut settings);
}

fn toggle_settings(
    _activate: On<Activate>,
    mut settings: Query<&mut Node, (With<SettingsPanel>, Without<HelpPanel>)>,
    mut help: Query<&mut Node, (With<HelpPanel>, Without<SettingsPanel>)>,
) {
    toggle_panel(&mut settings, &mut help);
}

fn close_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut panels: Query<&mut Node, Or<(With<HelpPanel>, With<SettingsPanel>)>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for mut node in &mut panels {
            node.display = Display::None;
        }
    }
}

fn smaller_text(_activate: On<Activate>, mut settings: ResMut<ViewSettings>) {
    *settings = settings.stepped(-1);
}

fn bigger_text(_activate: On<Activate>, mut settings: ResMut<ViewSettings>) {
    *settings = settings.stepped(1);
}

fn default_text(_activate: On<Activate>, mut settings: ResMut<ViewSettings>) {
    *settings = settings.stepped(0);
}

fn show_scale(settings: Res<ViewSettings>, mut text: Single<&mut Text, With<ScaleText>>) {
    let label = format!("{:.0}%", settings.ui_scale * 100.);
    if text.0 != label {
        text.0 = label;
    }
}

/// The Help and Settings buttons at the top right and their panels below them.
fn toolbar() -> impl SceneList {
    bsn_list! {
        Node {
            position_type: PositionType::Absolute,
            top: px(4),
            right: px(8),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(6),
        }
        GlobalZIndex(20)
        Children [
            @FeathersButton {
                @caption: bsn! { Text("Settings") ThemedText }
            }
            AccessibleLabel("Settings: the size of the text")
            on(toggle_settings)
            --
            @FeathersButton {
                @caption: bsn! { Text("?") ThemedText }
            }
            AccessibleLabel("Help: the keys, the colors and how to move on the map")
            on(toggle_help)
        ]
        --
        Node {
            position_type: PositionType::Absolute,
            top: px(40),
            right: px(8),
            max_width: px(640),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            padding: px(12),
            row_gap: px(8),
        }
        GlobalZIndex(20)
        HelpPanel
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            Text("") ThemedText HelpText
            TextFont {
                font_size: bevy::prelude::FontSize::Px(14.),
            }
        ]
        --
        Node {
            position_type: PositionType::Absolute,
            top: px(40),
            right: px(8),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            padding: px(12),
            row_gap: px(10),
        }
        GlobalZIndex(20)
        SettingsPanel
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            Text("Size of the text and panels") ThemedText
            TextFont {
                font_size: bevy::prelude::FontSize::Px(14.),
            }
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("Smaller") ThemedText }
                }
                on(smaller_text)
                --
                Text("100%") ThemedText ScaleText
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(14.),
                }
                --
                @FeathersButton {
                    @caption: bsn! { Text("Bigger") ThemedText }
                }
                on(bigger_text)
                --
                @FeathersButton {
                    @caption: bsn! { Text("Default") ThemedText }
                }
                on(default_text)
            ]
            --
            Text("Also Ctrl+= and Ctrl+-, Ctrl+0 for the default.") ThemedText
            TextFont {
                font_size: bevy::prelude::FontSize::Px(12.),
            }
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_explains_the_keys_and_colors_of_what_is_shown() {
        let replay = help_text(&HelpContext { replay: true });
        assert!(replay.contains("in grays"));
        assert!(replay.contains("Left and Right arrows"));
        let map = help_text(&HelpContext { replay: false });
        assert!(map.contains("ramps in amber"));
        assert!(!map.contains("Left and Right arrows"));
    }
}
