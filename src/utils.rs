use crate::error::*;
use bevy::log::tracing;
use bevy::prelude::*;
/// Basic utils
use std::fs::File;
use std::io::prelude::*;
use tracing::instrument;

// We can create our own gizmo config group!
#[derive(Default, bevy::reflect::Reflect, GizmoConfigGroup)]
pub struct MyRoundGizmos;

/// Attempts to read the s2ma file.
#[instrument]
pub fn read_mpq_file(path: &str) -> Result<Vec<u8>, BevySC2MapError> {
    tracing::info!("Opening file.");
    let mut f = File::open(path)?;
    tracing::info!("Reading into buffer.");
    let mut buffer: Vec<u8> = vec![];
    // read the whole file
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn update_gizmo_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line.style = match config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line.joints = match config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<MyRoundGizmos>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line.width += 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line.width -= 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line.style = match my_config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line.joints = match my_config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behavior in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        if virtual_time.is_paused() {
            virtual_time.unpause();
        } else {
            virtual_time.pause();
        }
    }
}
