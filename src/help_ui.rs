//! The Auto-rotate and Help buttons at the top right of the window. Help: what the keys do, what
//! the colors mean and how to move on the map. The size of the text has no panel, Ctrl+= / Ctrl+- / Ctrl+0 change it.

use crate::auto_rotate::{AutoRotateText, toggle_auto_rotate};
use crate::replay::ReplayData;
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

pub struct HelpUiPlugin;

impl Plugin for HelpUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (toolbar.spawn(), write_help).chain())
            .add_systems(Update, close_on_escape);
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
        "  Auto-rotate (top right): the camera turns slowly around the point of the ground in the"
            .to_string(),
        "  middle of the view, a turn in 90 seconds; moving the camera (keys, right drag, wheel)"
            .to_string(),
        "  or Stop rotating stops it.".to_string(),
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
        "  twinkle; the geysers are a ring of rock around a vent puffing green gas (violet for the"
            .to_string(),
        "  rich geysers); the start locations are landing pads with lights running around them."
            .to_string(),
        "  Destructible rocks are piles of boulders, debris piles of rusty metal chunks, the".to_string(),
        "  unbuildable plates gray hexagonal plates, on a dark patch of rubble the size of their".to_string(),
        "  footprint (along the diagonal for the diagonal ones). Click one for its name.".to_string(),
        "  No fly zones (air units cannot enter): a red \"no\" sign on the ground, a ring crossed by"
            .to_string(),
        "  a bar, under a faint red column of air with fading hoops.".to_string(),
        "  Pathing: the ground a unit can walk on, read from the cliffs of the map, in faint"
            .to_string(),
        "  aqua. In a replay, the cells of the buildings, minerals, geysers and rocks standing"
            .to_string(),
        "  at the time shown are orange, and the ground they close off from the rest of the map"
            .to_string(),
        "  is violet: the units there cannot walk out (a base walled in). Gaps of less than 4"
            .to_string(),
        "  cells are left out.".to_string(),
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
            "  Replay: a bar counts, per 4x4 cells up to the time shown, in the color of its player, the".to_string(),
            "  units died (Deaths), born (Births) or the enemy units killed (Kills, at the place of the".to_string(),
            "  killer), from the dropdown. Each player has its own slot in a cell, its bars never hide".to_string(),
            "  another's.".to_string(),
            "  Click a unit in the panel at the top right to show only its bars, click it again for all".to_string(),
            "  of them.".to_string(),
            "  Kills: clicking a bar, or a Siege Tank, Liberator, Lurker, Photon Cannon, Spine Crawler,".to_string(),
            "  Planetary Fortress or Missile Turret in the units panel, draws the range of these units".to_string(),
            "  around the places they killed from: a circle in a light shade of the player's color (a".to_string(),
            "  dark edge around it), the unit a small circle; a Liberator's zone is joined to it.".to_string(),
            "  Players: the winner first, a green triangle up, the loser a red triangle down; click the".to_string(),
            "  most effective unit of a player to show where it killed (Kills).".to_string(),
            "  The bars of the unit chosen in a panel breathe, their glow rising and falling.".to_string(),
            "  Upgrades (bottom): a lane per player, each upgrade's icon where it finished, the minutes".to_string(),
            "  under them, a light line at the time shown; the upgrades after it are faded. Hover an".to_string(),
            "  icon for its name and time.".to_string(),
        ]);
    }
    lines.extend([
        String::new(),
        "Click a bar or an object to see what it is at the bottom left, with the units of a bar;"
            .to_string(),
        "Escape hides it. Click the header of a panel with an arrow to fold or unfold it."
            .to_string(),
    ]);
    lines.join("\n")
}

fn write_help(mut text: Single<&mut Text, With<HelpText>>, replay: Option<Res<ReplayData>>) {
    text.0 = help_text(&HelpContext {
        replay: replay.is_some(),
    });
}

fn toggle_help(_activate: On<Activate>, mut help: Query<&mut Node, With<HelpPanel>>) {
    for mut node in &mut help {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn close_on_escape(keys: Res<ButtonInput<KeyCode>>, mut help: Query<&mut Node, With<HelpPanel>>) {
    if keys.just_pressed(KeyCode::Escape) {
        for mut node in &mut help {
            node.display = Display::None;
        }
    }
}

/// The Help button at the top right and its panel below it.
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
                @caption: bsn! { Text("Auto-rotate") ThemedText AutoRotateText }
            }
            AccessibleLabel("Turn the camera slowly around the map, or stop")
            on(toggle_auto_rotate)
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
        assert!(replay.contains("Kills"));
        assert!(replay.contains("Upgrades (bottom)"));
        assert!(replay.contains("Ctrl+="));
        let map = help_text(&HelpContext { replay: false });
        assert!(map.contains("ramps in amber"));
        assert!(!map.contains("Left and Right arrows"));
    }
}
