//! The Auto-rotate button at the top right: the camera turns slowly around the point of the ground
//! it looks at, a turn in [`TURN_SECONDS`], until the button is pressed again or the camera is
//! moved (W A S D, Q, E, Space, a right drag, the wheel). The free camera is paused while it turns
//! and takes the new direction back when it stops.

use bevy::camera_controller::free_camera::FreeCameraState;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

/// The seconds of a full turn.
pub const TURN_SECONDS: f32 = 90.;

/// The keys and buttons that move the camera, they stop the rotation.
const CAMERA_KEYS: [KeyCode; 8] = [
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyQ,
    KeyCode::KeyE,
    KeyCode::Space,
    KeyCode::KeyM,
];

/// Whether the camera turns, and around which point.
#[derive(Resource, Debug, Default)]
pub struct AutoRotate {
    pub pivot: Option<Vec3>,
}

/// The caption of the button, what pressing it does.
#[derive(Component, Default, Clone)]
pub struct AutoRotateText;

pub struct AutoRotatePlugin;

impl Plugin for AutoRotatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoRotate>()
            .add_systems(Update, (stop_on_input, orbit, show_caption).chain());
    }
}

/// The point of the ground (y = 0) in the middle of the view, or a point ahead when the camera
/// looks above the horizon.
pub fn pivot_of(camera: &Transform) -> Vec3 {
    let forward = camera.forward().as_vec3();
    if forward.y < -0.05 {
        let distance = -camera.translation.y / forward.y;
        camera.translation + forward * distance
    } else {
        let ahead = camera.translation + forward * 10.;
        Vec3::new(ahead.x, 0., ahead.z)
    }
}

/// Starts or stops the rotation.
pub fn toggle_auto_rotate(
    _activate: On<Activate>,
    mut rotate: ResMut<AutoRotate>,
    camera: Query<&Transform, With<FreeCameraState>>,
) {
    rotate.pivot = match (rotate.pivot, camera.single()) {
        (None, Ok(camera)) => Some(pivot_of(camera)),
        _ => None,
    };
}

fn stop_on_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    mut rotate: ResMut<AutoRotate>,
) {
    if rotate.pivot.is_none() {
        return;
    }
    if keys.any_pressed(CAMERA_KEYS)
        || mouse.just_pressed(MouseButton::Right)
        || scroll.delta != Vec2::ZERO
    {
        rotate.pivot = None;
    }
}

/// Turns the camera around the pivot, the free camera paused; gives it the new direction back
/// when the rotation stops.
fn orbit(
    time: Res<Time>,
    rotate: Res<AutoRotate>,
    mut cameras: Query<(&mut Transform, &mut FreeCameraState)>,
) {
    for (mut transform, mut state) in &mut cameras {
        match rotate.pivot {
            Some(pivot) => {
                state.enabled = false;
                let turn =
                    Quat::from_rotation_y(std::f32::consts::TAU / TURN_SECONDS * time.delta_secs());
                transform.rotate_around(pivot, turn);
            }
            None if !state.enabled => {
                let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
                state.yaw = yaw;
                state.pitch = pitch;
                state.velocity = Vec3::ZERO;
                state.enabled = true;
            }
            None => {}
        }
    }
}

fn show_caption(rotate: Res<AutoRotate>, mut text: Query<&mut Text, With<AutoRotateText>>) {
    if !rotate.is_changed() {
        return;
    }
    let caption = if rotate.pivot.is_some() {
        "Stop rotating"
    } else {
        "Auto-rotate"
    };
    for mut text in &mut text {
        text.0 = caption.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_turns_around_the_point_looked_at() {
        let camera = Transform::from_xyz(10., 10., 10.).looking_at(Vec3::new(5., 0., 5.), Vec3::Y);
        let pivot = pivot_of(&camera);
        assert!(pivot.distance(Vec3::new(5., 0., 5.)) < 0.001);
        // Above the horizon, a point ahead on the ground.
        let up = Transform::from_xyz(0., 2., 0.).looking_at(Vec3::new(0., 3., -10.), Vec3::Y);
        assert_eq!(pivot_of(&up).y, 0.);
    }
}
