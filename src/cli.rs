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

/// Parse cli args.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The path where the caches have been downloaded to.
    /// A user should have already clicked on download caches.
    /// This is set in the Config section of Swarmy.
    #[arg(short, long)]
    pub path: String,
    /// A comma-separated list of caches to inspect for the map data.
    /// When a replay is loaded it contains multiple ids for different purposes.
    /// I assume there's only one t3HeightMap and only one MapInfo sector.
    #[arg(short, long, required_unless_present = "replay")]
    pub ids: Option<String>,
    /// Shows the units born and died during the game of this SC2Replay file on its map, the
    /// replay has the cache ids of the map.
    #[arg(short, long)]
    pub replay: Option<String>,
    /// The game second the replay is shown at when it starts, the start of the game by default.
    #[arg(long, requires = "replay")]
    pub replay_seconds: Option<u32>,
    /// Saves a screenshot to this PNG file once the map is loaded and exits.
    #[arg(long)]
    pub screenshot: Option<String>,
}
