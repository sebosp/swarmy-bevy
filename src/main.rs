use bevy::camera_controller::free_camera::FreeCameraPlugin;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::{FeathersPlugins, theme::UiTheme};
use bevy::prelude::*;
use bevy_skein::SkeinPlugin;
use clap::Parser;
use swarmy_bevy::MapScene;
use swarmy_bevy::cli::*;
use swarmy_bevy::embedded_assets::{EmbeddedAssetsPlugin, SWARMY_OBJECTS_GLTF};
use swarmy_bevy::replay::ReplayData;
use swarmy_bevy::replay::plugin::{ReplayPlugin, ReplayStartSeconds};
use swarmy_bevy::siege_spots::{SiegeSpots, SiegeSpotsPlugin};

fn main() {
    let args = Args::parse();
    if !args.has_map() {
        show_help();
        return;
    }
    let path = args
        .map_caches
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    // The replay has the cache ids of its map.
    let replay = args.replay.as_deref().map(|replay| {
        ReplayData::load(replay).unwrap_or_else(|err| {
            eprintln!("Unable to read the replay {replay}: {err:?}");
            std::process::exit(1);
        })
    });
    let ids = match (&replay, args.map_cache_ids) {
        (Some(replay), _) => replay.cache_handle_ids.join(","),
        (None, Some(ids)) => ids,
        (None, None) => unreachable!("has_map requires the map cache ids without a replay"),
    };
    let siege_spots = args.siege_spots.as_deref().map(|path| {
        SiegeSpots::load(path).unwrap_or_else(|err| {
            eprintln!("Unable to read the siege spots {err}");
            std::process::exit(1);
        })
    });
    let mut app = App::new();
    if let Some(siege_spots) = siege_spots {
        app.insert_resource(siege_spots)
            .add_plugins(SiegeSpotsPlugin);
    }
    if let Some(replay) = replay {
        app.insert_resource(replay)
            .insert_resource(ReplayStartSeconds(args.replay_seconds))
            .add_plugins(ReplayPlugin);
    }
    app.insert_resource(CliParams { path, ids })
        .insert_resource(ScreenshotParams {
            path: args.screenshot,
        })
        .init_gizmo_group::<swarmy_bevy::utils::MyRoundGizmos>()
        .add_plugins((
            DefaultPlugins,
            EmbeddedAssetsPlugin,
            FeathersPlugins,
            SkeinPlugin::default(),
            FreeCameraPlugin,
            swarmy_bevy::map_plugin::MapPlugin,
            MeshPickingPlugin,
            swarmy_bevy::screenshot::ScreenshotPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(PreStartup, load_gltf)
        .add_systems(Startup, setup_light_and_gizmo_control_text)
        .run();
}

/// Without a map or a replay the window shows the help, swarmy starts swarmy-bevy with them.
fn show_help() {
    eprintln!("{}", help_text());
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "swarmy-bevy".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
            commands.spawn((
                Text::new(help_text()),
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(14.),
                    ..default()
                },
                Node {
                    margin: UiRect::all(px(24)),
                    ..default()
                },
            ));
        })
        .run();
}

fn setup_light_and_gizmo_control_text(mut commands: Commands) {
    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // basic instructions
    // However other plugins have more controls, such as F1/F2 for brightness, etc.
    commands.spawn((
        Text::new(
            "Controls:\n\
            W/A/S/D to move (Shift for speed)\n\
            Mouse drag (scroll for speed)\n\
            B for AABB\n\
            M for lock/unlock mouse navigation",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
        TextFont {
            font_size: bevy::prelude::FontSize::Px(11.),
            ..default()
        },
    ));
}

fn load_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the blender scene.
    /*commands.spawn((
        WorldAssetRoot(
            // SwarmyObjects handle
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("swarmy-objects.gltf")),
        ),
        Transform::from_xyz(10., -2.5, 10.).with_scale(Vec3 {
            x: 10.,
            y: 2.,
            z: 10.,
        }),
    ));*/
    let gltf = asset_server.load(SWARMY_OBJECTS_GLTF);
    commands.insert_resource(MapScene(gltf));
}
