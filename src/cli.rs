use bevy::prelude::*;
use clap::Parser;

#[derive(Default, Resource, bevy::reflect::Reflect)]
pub struct CliParams {
    pub path: String,
    pub ids: String,
}

/// Saves a screenshot of the map to this file and exits, to check the rendering.
#[derive(Default, Resource, bevy::reflect::Reflect)]
pub struct ScreenshotParams {
    pub path: Option<String>,
}

/// Shown above the options when swarmy-bevy runs without a map or a replay.
pub const ABOUT: &str = "swarmy-bevy draws a StarCraft II map in 3D: its terrain levels, ramps, \
resources and start locations, and, for a replay, the units born and died of each player over \
time.

It is meant to be started by swarmy (https://github.com/sebosp/swarmy), the StarCraft II replay \
batch analyser: \"Open in Swarmy-Bevy\" in Map Details or \"Load in swarmy-bevy\" in a replay. \
swarmy downloads the map caches and passes their directory and the replay.";

/// The command line options.
#[derive(Parser, Debug)]
#[command(version, about = ABOUT, long_about = None)]
pub struct Args {
    /// The directory of the map caches, the .s2ma files that StarCraft II downloads for each map,
    /// i.e. the "Map Caches Path" of swarmy's Config.
    #[arg(short = 'p', long = "map-caches", alias = "path", value_name = "DIR")]
    pub map_caches: Option<String>,
    /// The map to draw, a comma-separated list of the cache ids (sha256) of the map in the map
    /// caches directory, as stored in a replay. Not needed with --replay, the replay has them.
    #[arg(
        short = 'i',
        long = "map-cache-ids",
        alias = "ids",
        value_name = "IDS",
        requires = "map_caches"
    )]
    pub map_cache_ids: Option<String>,
    /// A .SC2Replay file: draws its map with the units born and died of each player over time.
    #[arg(short, long, value_name = "FILE", requires = "map_caches")]
    pub replay: Option<String>,
    /// The game second the replay starts at, the start of the game by default.
    #[arg(long, value_name = "SECONDS", requires = "replay")]
    pub replay_seconds: Option<u32>,
    /// A JSON file of swarmy with the spots where the Siege Tanks sieged on the map and their
    /// kills, drawn as bars. swarmy writes it from "Siege Tank spots" in Map Details.
    #[arg(long, value_name = "FILE", requires = "map_cache_ids")]
    pub siege_spots: Option<String>,
    /// Saves a screenshot to this PNG file once the map is drawn and exits.
    #[arg(long, value_name = "FILE")]
    pub screenshot: Option<String>,
}

impl Args {
    /// Whether there is a map to draw, otherwise the help is shown.
    pub fn has_map(&self) -> bool {
        self.map_caches.is_some() && (self.map_cache_ids.is_some() || self.replay.is_some())
    }
}

/// The help of the command line, shown in the window when there is no map to draw.
pub fn help_text() -> String {
    use clap::CommandFactory;
    Args::command().render_long_help().to_string()
}
