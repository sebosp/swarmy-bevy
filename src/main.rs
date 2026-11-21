use bevy::camera_controller::free_camera::FreeCameraPlugin;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::{FeathersPlugins, theme::UiTheme};
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_skein::SkeinPlugin;
use clap::Parser;
use std::time::Duration;
use swarmy_bevy::MapScene;
use swarmy_bevy::cli::*;
use swarmy_bevy::embedded_assets::{EmbeddedAssetsPlugin, SWARMY_OBJECTS_GLTF};
use swarmy_bevy::replay::plugin::{BarSelection, ReplayPlugin, ReplayStartSeconds};
use swarmy_bevy::replay::{CellEventKind, ReplayData};

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
        // The games played with a local map (AI Arena) have no cache handles: its file, named
        // <id>.s2ma in the map caches directory, is given with the ids.
        (Some(replay), Some(ids)) if replay.cache_handle_ids.is_empty() => ids,
        (Some(replay), _) => replay.cache_handle_ids.join(","),
        (None, Some(ids)) => ids,
        (None, None) => unreachable!("has_map requires the map cache ids without a replay"),
    };
    // A replay keeps the terrain in grays, its bars stand out.
    let terrain_textures = args.terrain_textures.unwrap_or(replay.is_none());
    let mut app = App::new();
    if let Some(replay) = replay {
        app.insert_resource(replay)
            .insert_resource(ReplayStartSeconds(args.replay_seconds))
            .insert_resource(BarSelection {
                kind: match args.replay_bars {
                    Some(ReplayBars::Births) => CellEventKind::Born,
                    Some(ReplayBars::Kills) => CellEventKind::Kill,
                    Some(ReplayBars::Deaths) | None => CellEventKind::Died,
                },
                unit: args.replay_unit.clone(),
            })
            .add_plugins(ReplayPlugin);
    }
    // A window out of sight waits for the display forever, a screenshot must not.
    let present_mode = if args.screenshot.is_some() {
        bevy::window::PresentMode::AutoNoVsync
    } else {
        bevy::window::PresentMode::default()
    };
    // At most 60 frames a second while the window has the focus (input wakes it up sooner), 10
    // without it: the window had drawn as fast as the display (120 Hz and more on laptops), the
    // fans of a laptop spun up. A screenshot draws as fast as it can.
    let frame_rate = if args.screenshot.is_some() {
        WinitSettings::continuous()
    } else {
        WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::from_secs_f64(1. / 60.)),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(1. / 10.)),
        }
    };
    app.insert_resource(CliParams { path, ids })
        .insert_resource(ScreenshotParams {
            path: args.screenshot,
        })
        .init_gizmo_group::<swarmy_bevy::utils::MyRoundGizmos>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode,
                    ..default()
                }),
                ..default()
            }),
            EmbeddedAssetsPlugin,
            FeathersPlugins,
            SkeinPlugin::default(),
            FreeCameraPlugin,
            swarmy_bevy::view_settings::ViewSettingsPlugin,
            swarmy_bevy::help_ui::HelpUiPlugin,
            swarmy_bevy::auto_rotate::AutoRotatePlugin,
            swarmy_bevy::map_plugin::MapPlugin {
                map_graphics: args.map_graphics,
            },
            swarmy_bevy::terrain_textures::TerrainTexturesPlugin {
                enabled: terrain_textures,
            },
            swarmy_bevy::pathing::PathingPlugin {
                enabled: args.pathing,
            },
            MeshPickingPlugin,
            swarmy_bevy::screenshot::ScreenshotPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(frame_rate)
        .add_systems(PreStartup, load_gltf)
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
