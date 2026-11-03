//! The settings of swarmy-bevy kept between runs, in `swarmy-bevy/settings.json` of the config
//! directory (`~/.config` on Linux): the size of the text and of the panels, changed in Settings
//! or with Ctrl+= and Ctrl+- (Ctrl+0 resets it), as in swarmy.

use bevy::prelude::*;
use bevy::window::RequestRedraw;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The smallest and the largest size, and the step of a key press.
pub const MIN_SCALE: f32 = 0.75;
pub const MAX_SCALE: f32 = 2.5;
pub const SCALE_STEP: f32 = 0.125;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewSettings {
    /// The size of the text and of the panels, 1 is the default.
    #[serde(default = "default_scale")]
    pub ui_scale: f32,
}

fn default_scale() -> f32 {
    1.
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            ui_scale: default_scale(),
        }
    }
}

impl ViewSettings {
    /// The settings file, None without a home directory.
    fn path() -> Option<PathBuf> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(config.join("swarmy-bevy").join("settings.json"))
    }

    /// The saved settings, the defaults when there are none or they cannot be read.
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|json| serde_json::from_str::<Self>(&json).ok())
            .map(|settings| settings.clamped())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self)
            && let Err(err) = std::fs::write(&path, json)
        {
            tracing::warn!("Unable to save the settings to {}: {err}", path.display());
        }
    }

    fn clamped(self) -> Self {
        Self {
            ui_scale: self.ui_scale.clamp(MIN_SCALE, MAX_SCALE),
        }
    }

    /// The scale after `steps` key presses, 0 resets it.
    pub fn stepped(self, steps: i32) -> Self {
        if steps == 0 {
            return Self::default();
        }
        Self {
            ui_scale: self.ui_scale + steps as f32 * SCALE_STEP,
        }
        .clamped()
    }
}

pub struct ViewSettingsPlugin;

impl Plugin for ViewSettingsPlugin {
    fn build(&self, app: &mut App) {
        let settings = ViewSettings::load();
        app.insert_resource(settings)
            .insert_resource(UiScale(settings.ui_scale))
            .add_systems(Update, (zoom_keys, apply_settings).chain());
    }
}

/// Ctrl+= (or Ctrl++) and Ctrl+- change the size of the text, Ctrl+0 resets it.
fn zoom_keys(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<ViewSettings>) {
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let steps = if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        1
    } else if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
        -1
    } else if keys.any_just_pressed([KeyCode::Digit0, KeyCode::Numpad0]) {
        0
    } else {
        return;
    };
    let stepped = settings.stepped(steps);
    if stepped != *settings {
        *settings = stepped;
    }
}

/// Applies the settings as they change and saves them.
fn apply_settings(
    settings: Res<ViewSettings>,
    mut ui_scale: ResMut<UiScale>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    if !settings.is_changed() || settings.is_added() {
        return;
    }
    ui_scale.0 = settings.ui_scale;
    settings.save();
    redraw.write(RequestRedraw);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_steps_the_size_within_its_bounds() {
        let settings = ViewSettings::default();
        assert_eq!(settings.stepped(2).ui_scale, 1.25);
        assert_eq!(settings.stepped(-100).ui_scale, MIN_SCALE);
        assert_eq!(settings.stepped(100).ui_scale, MAX_SCALE);
        assert_eq!(settings.stepped(3).stepped(0), ViewSettings::default());
        let json = serde_json::to_string(&settings.stepped(1)).unwrap();
        let read: ViewSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(read.ui_scale, 1.125);
    }
}
