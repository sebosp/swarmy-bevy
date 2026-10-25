//! Takes a screenshot once the map is loaded and exits, `--screenshot <file>`, to check the
//! rendering without interacting with the window.

use crate::cli::ScreenshotParams;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

/// The frames rendered before the screenshot, the map and its objects are loaded by then.
pub const SCREENSHOT_FRAME: u32 = 240;

/// The frames after the screenshot before exiting, the screenshot is saved asynchronously.
pub const EXIT_FRAME: u32 = SCREENSHOT_FRAME + 60;

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, take_screenshot);
    }
}

fn take_screenshot(
    mut commands: Commands,
    params: Res<ScreenshotParams>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(path) = &params.path else {
        return;
    };
    *frame += 1;
    if *frame == SCREENSHOT_FRAME {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
    } else if *frame == EXIT_FRAME {
        exit.write(AppExit::Success);
    }
}
