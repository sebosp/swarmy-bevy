//! A replay shown on the map as the units born and died per cell over time, `--replay <file>`.
//!
//! The replay contains the cache handles of the map, so only the cache path is needed. The unit
//! born and died tracker events are aggregated per cell of [`AGGREGATE_CELLS`] map cells, the
//! [`plugin`] draws them as bars that grow as the time moves forward in [`TIME_STEP_SECONDS`]
//! steps. The players, their upgrades and their most effective unit and ability are shown too.

pub mod plugin;

use crate::error::BevySC2MapError;
use bevy::math::UVec2;
use bevy::prelude::Resource;
use s2protocol::basic_replay_data::SC2ReplayBasicData;
use s2protocol::game_events::{VersionedBalanceUnits, read_balance_data_from_included_assets};
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::{SC2EventIterator, SC2EventType, SC2Unit, UnitChangeHint};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// The map cells aggregated into one cell of the bars, a 4x4 square.
pub const AGGREGATE_CELLS: u32 = 4;

/// The game loops per second at Faster speed, the speed of the ladder and the tournaments.
/// `s2protocol::convert_tracker_loop_to_seconds` uses 68.58 ms per loop, about 14.6 loops per
/// second, the game times would be 1.5 times longer than in the game and in swarmy.
pub const GAME_LOOPS_PER_SECOND: f64 = 22.4;

/// The game seconds of a tracker loop at Faster speed.
pub fn loop_seconds(tracker_loop: i64) -> u32 {
    (tracker_loop.max(0) as f64 / GAME_LOOPS_PER_SECOND) as u32
}

/// The time moves forward and backwards in steps of these many game seconds.
pub const TIME_STEP_SECONDS: u32 = 30;

/// The length of the sha256 at the end of the cache handles, the cache files are named after it.
const SHA256_HEX_LEN: usize = 64;

/// The abilities that every unit has, they are not interesting as the most effective ability.
const GENERIC_ABILITIES: [&str; 8] = [
    "stop",
    "move",
    "attack",
    "Rally",
    "ProgressRally",
    "Train.",
    "Build.",
    "Research.",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayPlayer {
    /// The name without the clan tag escapes, i.e. `<TAG>Player`.
    pub name: String,
    pub race: String,
    /// Win, Loss, Tie or Undecided.
    pub result: String,
    pub team: u8,
    /// RGBA, from the ARGB color of the replay details.
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellEventKind {
    Born,
    Died,
}

/// An aggregated cell and the index of a player in [`ReplayData::players`].
pub type CellPlayer = ((u32, u32), usize);

/// A unit born or died in an aggregated cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CellEvent {
    pub seconds: u32,
    /// The aggregated cell, the map cell divided by [`AGGREGATE_CELLS`].
    pub cell: UVec2,
    pub kind: CellEventKind,
    /// The index of the owner in [`ReplayData::players`].
    pub player: usize,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeDone {
    pub seconds: u32,
    pub player: usize,
    /// The upgrade name of the tracker events, i.e. `zerglingmovementspeed`.
    pub name: String,
    /// The icon of the balance data, its texture is embedded by s2protocol.
    pub icon: Option<String>,
}

/// An enemy unit killed by a unit of a player.
#[derive(Debug, Clone, PartialEq)]
pub struct Kill {
    pub seconds: u32,
    /// The player that killed the unit.
    pub player: usize,
    /// The killer unit type.
    pub unit: String,
    /// The last ability the killer unit was commanded, unless it is a generic one like attack.
    pub ability: Option<String>,
}

/// The replay aggregated for the map.
#[derive(Resource, Debug, Clone, Default)]
pub struct ReplayData {
    pub file_name: String,
    pub map_title: String,
    /// The sha256 of the cache handles, the names of the files in the cache path.
    pub cache_handle_ids: Vec<String>,
    pub players: Vec<ReplayPlayer>,
    pub duration_seconds: u32,
    /// Sorted by time.
    pub events: Vec<CellEvent>,
    pub upgrades: Vec<UpgradeDone>,
    pub kills: Vec<Kill>,
    /// The icons of the units and of the ability commands, `<ability>.<command>`.
    pub unit_icons: HashMap<String, String>,
    pub ability_icons: HashMap<String, String>,
}

/// Removes the escapes of the clan tag of the player names, i.e. `&lt;TAG&gt;<sp/>Player`.
pub fn clean_player_name(name: &str) -> String {
    name.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("<sp/>", " ")
}

/// The variants of a unit that are the same unit, i.e. a burrowed Roach is a Roach.
pub fn base_unit_name(name: &str) -> &str {
    match name {
        "SiegeTankSieged" => "SiegeTank",
        "VikingAssault" => "VikingFighter",
        "LiberatorAG" => "Liberator",
        "ThorAP" => "Thor",
        "WarpPrismPhasing" => "WarpPrism",
        "ObserverSiegeMode" => "Observer",
        "OverseerSiegeMode" => "Overseer",
        _ => name.strip_suffix("Burrowed").unwrap_or(name),
    }
}

/// Larva, eggs, broodlings, etc cost nothing, they are not counted.
fn has_cost(balance: &VersionedBalanceUnits, name: &str) -> bool {
    balance
        .get(base_unit_name(name))
        .and_then(|unit| unit.cost.as_ref())
        .is_some_and(|cost| cost.minerals.unwrap_or(0) > 0 || cost.vespene.unwrap_or(0) > 0)
}

fn is_generic_ability(ability: &str) -> bool {
    ability.is_empty()
        || GENERIC_ABILITIES
            .iter()
            .any(|generic| ability.starts_with(generic))
}

/// The balance data of the protocol version or the closest older one, newer patches add units.
fn closest_balance(
    all: &HashMap<u32, VersionedBalanceUnits>,
    version: u32,
) -> Option<&VersionedBalanceUnits> {
    all.iter()
        .min_by_key(|(candidate, _)| (**candidate > version, candidate.abs_diff(version)))
        .map(|(_, units)| units)
}

/// The aggregated cell of a map position.
fn aggregate_cell(unit: &SC2Unit) -> UVec2 {
    let x = unit.pos.x().max(0.) as u32;
    let y = unit.pos.y().max(0.) as u32;
    UVec2::new(x / AGGREGATE_CELLS, y / AGGREGATE_CELLS)
}

impl ReplayData {
    /// Reads the replay and aggregates its events.
    pub fn load(path: &str) -> Result<Self, BevySC2MapError> {
        let source = PathBuf::from(path);
        let basic_data = SC2ReplayBasicData::new(&source, 0)?;
        let all_balance = read_balance_data_from_included_assets()?;
        let balance = closest_balance(&all_balance, basic_data.init_data.version)
            .cloned()
            .unwrap_or_default();
        let details = &basic_data.details;
        let raw_names: Vec<String> = details
            .player_list
            .iter()
            .map(|player| player.name.clone())
            .collect();
        let players: Vec<ReplayPlayer> = details
            .player_list
            .iter()
            .map(|player| ReplayPlayer {
                name: clean_player_name(&player.name),
                race: player.race.clone(),
                result: player.result.clone(),
                team: player.team_id,
                color: [
                    player.color.r,
                    player.color.g,
                    player.color.b,
                    player.color.a,
                ],
            })
            .collect();
        // The units of the players have their name, the computer players only their player id.
        let unit_player = |unit: &SC2Unit| -> Option<usize> {
            match (&unit.player_name, unit.user_id) {
                (Some(name), _) => raw_names.iter().position(|raw| raw == name),
                (None, Some(player_id)) => {
                    let idx = (player_id as usize).checked_sub(1)?;
                    (idx < players.len()).then_some(idx)
                }
                _ => None,
            }
        };
        let upgrade_icons = s2protocol::textures::upgrade_icons(&balance);
        let mut data = ReplayData {
            file_name: path.to_string(),
            map_title: details.title.clone(),
            cache_handle_ids: details
                .cache_handles
                .iter()
                .map(|handle| {
                    let start = handle.len().saturating_sub(SHA256_HEX_LEN);
                    handle[start..].to_string()
                })
                .collect(),
            unit_icons: balance
                .iter()
                .filter_map(|(name, unit)| Some((name.clone(), unit.meta.icon.clone()?)))
                .collect(),
            ability_icons: s2protocol::textures::ability_command_icons(&balance),
            ..Default::default()
        };
        for item in SC2EventIterator::new(&basic_data, &all_balance)? {
            let SC2EventType::Tracker {
                tracker_loop,
                event,
            } = &item.event_type
            else {
                continue;
            };
            let seconds = loop_seconds(*tracker_loop);
            data.duration_seconds = data.duration_seconds.max(seconds);
            match (event, &item.change_hint) {
                (
                    ReplayTrackerEvent::UnitBorn(_) | ReplayTrackerEvent::UnitInit(_),
                    UnitChangeHint::Registered { unit, .. },
                ) => {
                    let Some(player) = unit_player(unit) else {
                        continue;
                    };
                    if has_cost(&balance, &unit.name) {
                        data.events.push(CellEvent {
                            seconds,
                            cell: aggregate_cell(unit),
                            kind: CellEventKind::Born,
                            player,
                            unit: base_unit_name(&unit.name).to_string(),
                        });
                    }
                }
                (
                    ReplayTrackerEvent::UnitDied(_),
                    UnitChangeHint::Unregistered { killer, killed },
                ) => {
                    let Some(player) = unit_player(killed) else {
                        continue;
                    };
                    if !has_cost(&balance, &killed.name) {
                        continue;
                    }
                    data.events.push(CellEvent {
                        seconds,
                        cell: aggregate_cell(killed),
                        kind: CellEventKind::Died,
                        player,
                        unit: base_unit_name(&killed.name).to_string(),
                    });
                    // A Drone morphing into a structure dies without a killer, own units killed
                    // are not kills either.
                    let Some(killer) = killer else {
                        continue;
                    };
                    let Some(killer_player) = unit_player(killer) else {
                        continue;
                    };
                    if killer_player == player {
                        continue;
                    }
                    let ability = killer
                        .cmd
                        .abil
                        .as_ref()
                        .map(|abil| abil.ability.clone())
                        .filter(|ability| !is_generic_ability(ability));
                    data.kills.push(Kill {
                        seconds,
                        player: killer_player,
                        unit: base_unit_name(&killer.name).to_string(),
                        ability,
                    });
                }
                (ReplayTrackerEvent::Upgrade(upgrade), _) => {
                    let Some(player) = (upgrade.player_id as usize)
                        .checked_sub(1)
                        .filter(|idx| *idx < players.len())
                    else {
                        continue;
                    };
                    // The sprays, dances, etc are upgrades too.
                    let icon = upgrade_icons
                        .get(&upgrade.upgrade_type_name.to_lowercase())
                        .cloned();
                    if icon.is_none() && upgrade.upgrade_type_name.starts_with("Spray") {
                        continue;
                    }
                    // The upgrades at the start of the game are the rewards and skins.
                    if seconds == 0 {
                        continue;
                    }
                    data.upgrades.push(UpgradeDone {
                        seconds,
                        player,
                        name: upgrade.upgrade_type_name.clone(),
                        icon,
                    });
                }
                _ => {}
            }
        }
        data.players = players;
        Ok(data)
    }

    /// The times the replay can be shown at, from the start to the end of the game.
    pub fn time_steps(&self) -> Vec<u32> {
        let mut steps: Vec<u32> = (0..self.duration_seconds)
            .step_by(TIME_STEP_SECONDS as usize)
            .collect();
        steps.push(self.duration_seconds);
        steps
    }

    /// The units born and died of each player in each aggregated cell up to the time, by cell and
    /// player index.
    pub fn cell_counts(&self, until_seconds: u32) -> BTreeMap<CellPlayer, (u32, u32)> {
        let mut counts: BTreeMap<CellPlayer, (u32, u32)> = BTreeMap::new();
        for event in self
            .events
            .iter()
            .take_while(|event| event.seconds <= until_seconds)
        {
            let count = counts
                .entry(((event.cell.x, event.cell.y), event.player))
                .or_default();
            match event.kind {
                CellEventKind::Born => count.0 += 1,
                CellEventKind::Died => count.1 += 1,
            }
        }
        counts
    }

    /// The most units born or died of a player in a cell during the game, the bars are scaled by
    /// it so that their size can be compared over time.
    pub fn max_cell_count(&self) -> u32 {
        self.cell_counts(self.duration_seconds)
            .values()
            .map(|(born, died)| *born.max(died))
            .max()
            .unwrap_or_default()
    }

    /// The upgrades of the player finished up to the time.
    pub fn upgrades_until(&self, player: usize, until_seconds: u32) -> Vec<&UpgradeDone> {
        self.upgrades
            .iter()
            .filter(|upgrade| upgrade.player == player && upgrade.seconds <= until_seconds)
            .collect()
    }

    /// The unit type of the player that killed the most enemy units up to the time.
    pub fn most_effective_unit(
        &self,
        player: usize,
        until_seconds: u32,
    ) -> Option<(String, usize)> {
        self.most_kills(player, until_seconds, |kill| Some(kill.unit.clone()))
    }

    /// The ability most of the killer units of the player were commanded before their kills.
    pub fn most_effective_ability(
        &self,
        player: usize,
        until_seconds: u32,
    ) -> Option<(String, usize)> {
        self.most_kills(player, until_seconds, |kill| kill.ability.clone())
    }

    fn most_kills(
        &self,
        player: usize,
        until_seconds: u32,
        key: impl Fn(&Kill) -> Option<String>,
    ) -> Option<(String, usize)> {
        let mut kills: BTreeMap<String, usize> = BTreeMap::new();
        for kill in self
            .kills
            .iter()
            .filter(|kill| kill.player == player && kill.seconds <= until_seconds)
        {
            if let Some(key) = key(kill) {
                *kills.entry(key).or_default() += 1;
            }
        }
        // Ties go to the first name, so the result is stable.
        kills
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    }
}

/// Formats the game seconds as `mm:ss`.
pub fn game_time(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A replay of the s2protocol-rs checkout next to this one.
    const REPLAY: &str = "../s2protocol-rs/assets/FieldsDeath202507.SC2Replay";

    #[test]
    fn it_converts_the_loops_at_faster_speed() {
        // 25:21 of game time.
        assert_eq!(loop_seconds(34071), 25 * 60 + 21);
        assert_eq!(loop_seconds(-5), 0);
    }

    #[test]
    fn it_cleans_the_clan_tag() {
        assert_eq!(clean_player_name("&lt;TAG&gt;<sp/>Player"), "<TAG> Player");
    }

    #[test]
    fn it_aggregates_a_replay() {
        if !std::path::Path::new(REPLAY).exists() {
            eprintln!("Skipping, {REPLAY} not found");
            return;
        }
        let data = ReplayData::load(REPLAY).unwrap();
        // A player against two computer players.
        assert_eq!(data.players.len(), 3);
        assert_eq!(data.players[0].name, "<chezs> Sazed");
        assert_eq!(data.players[0].result, "Loss");
        assert_eq!(data.players[0].color, [22, 128, 0, 255]);
        assert!(
            data.upgrades
                .iter()
                .any(|upgrade| upgrade.name == "Stimpack"
                    && upgrade.icon.as_deref() == Some("btn-ability-terran-stimpack-color"))
        );
        assert_eq!(
            data.most_effective_unit(0, data.duration_seconds),
            Some(("LurkerMP".to_string(), 109))
        );
        assert!(!data.cache_handle_ids.is_empty());
        assert!(data.cache_handle_ids.iter().all(|id| id.len() == 64));
        assert!(data.duration_seconds > 0);
        // Born and died events, sorted by time.
        let born = data
            .events
            .iter()
            .filter(|event| event.kind == CellEventKind::Born)
            .count();
        assert!(born > 20);
        assert!(data.events.windows(2).all(|w| w[0].seconds <= w[1].seconds));
        // The counts grow over time.
        let half: u32 = data
            .cell_counts(data.duration_seconds / 2)
            .values()
            .map(|c| c.0)
            .sum();
        let end: u32 = data
            .cell_counts(data.duration_seconds)
            .values()
            .map(|c| c.0)
            .sum();
        assert!(half <= end);
        assert_eq!(end as usize, born);
        assert_eq!(data.time_steps().last(), Some(&data.duration_seconds));
    }
}
