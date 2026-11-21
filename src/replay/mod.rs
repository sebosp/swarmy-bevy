//! A replay shown on the map as the units born and died per cell over time, `--replay <file>`.
//!
//! The replay contains the cache handles of the map, so only the cache path is needed. The unit
//! born and died tracker events are aggregated per cell of [`AGGREGATE_CELLS`] map cells, the
//! [`plugin`] draws them as bars that grow as the time moves forward in [`TIME_STEP_SECONDS`]
//! steps. The kills are aggregated at the position of the killer unit, with the range of the
//! static and sieged units ([`RANGED_UNITS`]). The players, their upgrades and their most
//! effective unit and ability are shown too.

pub mod plugin;
pub mod timeline;

use crate::error::BevySC2MapError;
use bevy::math::UVec2;
use bevy::prelude::Resource;
use s2protocol::basic_replay_data::SC2ReplayBasicData;
use s2protocol::cache_handles::pathing::ground_footprint;
use s2protocol::game_events::{VersionedBalanceUnits, read_balance_data_from_included_assets};
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::{SC2EventIterator, SC2EventType, SC2Unit, SC2UnitCmdData, UnitChangeHint};
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
    /// A unit born, not the hallucinations.
    Born,
    /// A unit lost, killed by another player or its own, not a morph or a unit that expired.
    Died,
    /// An enemy unit killed, at the position of the killer unit, counted for its owner.
    Kill,
}

/// The units whose range is drawn around the positions they killed from, in the form they have
/// while they fire (sieged, burrowed), with the upgrade that extends it and by how much: the
/// balance data has the ranges, not what the upgrades add (game values of the current patch).
pub const RANGED_UNITS: [(&str, Option<(&str, f32)>); 7] = [
    ("SiegeTankSieged", None),
    ("LiberatorAG", None),
    // Seismic Spines.
    ("LurkerMPBurrowed", Some(("LurkerRange", 2.))),
    ("PhotonCannon", None),
    ("SpineCrawler", None),
    // Hi-Sec Auto Tracking.
    ("PlanetaryFortress", Some(("HiSecAutoTracking", 1.))),
    ("MissileTurret", Some(("HiSecAutoTracking", 1.))),
];

/// The ranges used when the balance data of the build has no weapon range for a unit.
const FALLBACK_RANGES: [(&str, f32); 7] = [
    ("SiegeTankSieged", 13.),
    ("LiberatorAG", 10.),
    ("LurkerMPBurrowed", 8.),
    ("PhotonCannon", 7.),
    ("SpineCrawler", 7.),
    ("PlanetaryFortress", 6.),
    ("MissileTurret", 7.),
];

/// The radius of the zone of a Liberator in Defender Mode around the point it was ordered to,
/// its weapon's range (10) reaches the far side of the zone.
pub const LIBERATOR_ZONE_RADIUS: f32 = 5.;

/// The area a unit of [`RANGED_UNITS`] fired at when it killed, in map coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeCircle {
    /// The index of the owner in [`ReplayData::players`].
    pub player: usize,
    /// The position of the unit.
    pub unit: (f32, f32),
    /// The center of the circle: the unit, or the zone of a Liberator.
    pub center: (f32, f32),
    pub radius: f32,
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
    /// The killer unit as it was when it killed, i.e. `SiegeTankSieged`.
    pub variant: String,
    pub tag_index: u32,
    /// The position of the killer unit, in map coordinates.
    pub pos: (f32, f32),
    /// The radius of the killer unit, its range starts at its edge.
    pub radius: f32,
    /// The point of the last command of the killer unit, the zone of a Liberator.
    pub target: Option<(f32, f32)>,
}

/// The replay aggregated for the map.
#[derive(Resource, Debug, Clone, Default)]
pub struct ReplayData {
    /// The units that block the ground over time, for [`crate::pathing`].
    pub blockers: Vec<crate::pathing::BlockerEvent>,
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
    /// The weapon range of the units of [`RANGED_UNITS`], from the balance data of the build.
    pub weapon_ranges: HashMap<String, f32>,
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

/// The parts of the names of the abilities that switch the mode of a unit without damage: burrow
/// down or up, siege and unsiege, the modes of the Liberator, Viking, Thor and Warp Prism, the
/// morphs, a building lifting off or landing. The last command of a sieged Tank or a burrowed
/// Lurker is the switch, it is not what killed.
const MODE_SWITCHES: [&str; 8] = [
    "burrow",
    "mode",
    "unsiege",
    "liberatoragtarget",
    "morph",
    "phasing",
    "lift",
    "land",
];

/// The abilities that are not interesting as the most effective one: every unit has them, or they
/// switch the mode of the unit and do not kill.
fn is_generic_ability(ability: &str) -> bool {
    let lower = ability.to_lowercase();
    ability.is_empty()
        || GENERIC_ABILITIES
            .iter()
            .any(|generic| ability.starts_with(generic))
        || MODE_SWITCHES.iter().any(|switch| lower.contains(switch))
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

/// The longest weapon range of a unit in the balance data, or the known value of the game.
fn weapon_range(balance: &VersionedBalanceUnits, name: &str) -> Option<f32> {
    balance
        .get(name)
        .and_then(|unit| {
            unit.weapons
                .inner
                .iter()
                .filter_map(|weapon| weapon.misc.as_ref()?.range)
                .reduce(f32::max)
        })
        .or_else(|| {
            FALLBACK_RANGES
                .iter()
                .find(|(unit, _)| *unit == name)
                .map(|(_, range)| *range)
        })
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
            weapon_ranges: RANGED_UNITS
                .iter()
                .filter_map(|(unit, _)| Some((unit.to_string(), weapon_range(&balance, unit)?)))
                .collect(),
            ..Default::default()
        };
        // The units that block the ground now or did, a depot lowered still has its tag here.
        let mut blocking_tags: std::collections::HashSet<u32> = Default::default();
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
            // Born, done or changed (a depot lowered, a building lifting off), and died.
            match &item.change_hint {
                UnitChangeHint::Registered { unit, .. }
                    if ground_footprint(&unit.name).is_some()
                        || blocking_tags.contains(&unit.tag_index) =>
                {
                    blocking_tags.insert(unit.tag_index);
                    data.blockers.push(crate::pathing::BlockerEvent {
                        seconds,
                        tag_index: unit.tag_index,
                        unit: Some(unit.name.clone()),
                        x: unit.pos.x() as i32,
                        y: unit.pos.y() as i32,
                    });
                }
                UnitChangeHint::Unregistered { killed, .. }
                    if blocking_tags.remove(&killed.tag_index) =>
                {
                    data.blockers.push(crate::pathing::BlockerEvent {
                        seconds,
                        tag_index: killed.tag_index,
                        unit: None,
                        x: 0,
                        y: 0,
                    });
                }
                _ => {}
            }
            match (event, &item.change_hint) {
                (
                    ReplayTrackerEvent::UnitBorn(_) | ReplayTrackerEvent::UnitInit(_),
                    UnitChangeHint::Registered { unit, .. },
                ) => {
                    let Some(player) = unit_player(unit) else {
                        continue;
                    };
                    // The hallucinations of a sentry are not units of the player.
                    let hallucination = unit
                        .creator_ability_name
                        .as_deref()
                        .is_some_and(|ability| ability.starts_with("Hallucination"));
                    if has_cost(&balance, &unit.name) && !hallucination {
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
                    ReplayTrackerEvent::UnitDied(died),
                    UnitChangeHint::Unregistered { killer, killed },
                ) => {
                    let Some(player) = unit_player(killed) else {
                        continue;
                    };
                    // Only the units lost are deaths: a Drone that becomes a structure, a Larva
                    // that hatches, an expired MULE, AutoTurret or AdeptPhaseShift, a
                    // hallucination or a cancelled building are not (`UnitDiedCause`).
                    if !died.cause.is_loss() || !has_cost(&balance, &killed.name) {
                        continue;
                    }
                    data.events.push(CellEvent {
                        seconds,
                        cell: aggregate_cell(killed),
                        kind: CellEventKind::Died,
                        player,
                        unit: base_unit_name(&killed.name).to_string(),
                    });
                    // A kill is a unit killed by another player, own units killed are losses
                    // only. A unit killed inside a transport has no killer to show.
                    if !died.cause.is_kill() {
                        continue;
                    }
                    let Some(killer) = killer else {
                        continue;
                    };
                    let Some(killer_player) = unit_player(killer) else {
                        continue;
                    };
                    let ability = killer
                        .cmd
                        .abil
                        .as_ref()
                        .map(|abil| abil.ability.clone())
                        .filter(|ability| !is_generic_ability(ability));
                    let unit = base_unit_name(&killer.name).to_string();
                    data.events.push(CellEvent {
                        seconds,
                        cell: aggregate_cell(killer),
                        kind: CellEventKind::Kill,
                        player: killer_player,
                        unit: unit.clone(),
                    });
                    let target = match &killer.cmd.data {
                        SC2UnitCmdData::TargetPoint(point) => Some((point.x(), point.y())),
                        _ => None,
                    };
                    data.kills.push(Kill {
                        seconds,
                        player: killer_player,
                        unit,
                        ability,
                        variant: killer.name.clone(),
                        tag_index: killer.tag_index,
                        pos: (killer.pos.x(), killer.pos.y()),
                        radius: killer.radius,
                        target,
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

    /// The events of a kind up to the time, of one unit type or all of them.
    fn events_until<'a>(
        &'a self,
        until_seconds: u32,
        kind: CellEventKind,
        unit: Option<&'a str>,
    ) -> impl Iterator<Item = &'a CellEvent> {
        self.events
            .iter()
            .take_while(move |event| event.seconds <= until_seconds)
            .filter(move |event| event.kind == kind)
            .filter(move |event| unit.is_none_or(|unit| event.unit == unit))
    }

    /// The units born or died of each player in each aggregated cell up to the time, by cell and
    /// player index, of one unit type or all of them.
    pub fn cell_counts(
        &self,
        until_seconds: u32,
        kind: CellEventKind,
        unit: Option<&str>,
    ) -> BTreeMap<CellPlayer, u32> {
        let mut counts: BTreeMap<CellPlayer, u32> = BTreeMap::new();
        for event in self.events_until(until_seconds, kind, unit) {
            *counts
                .entry(((event.cell.x, event.cell.y), event.player))
                .or_default() += 1;
        }
        counts
    }

    /// The cells and players with units born or died during the game, they have a bar.
    pub fn cell_players(&self) -> Vec<CellPlayer> {
        let mut cells: Vec<CellPlayer> = self
            .events
            .iter()
            .map(|event| ((event.cell.x, event.cell.y), event.player))
            .collect();
        cells.sort();
        cells.dedup();
        cells
    }

    /// The most units born or died of a player in a cell during the game, the bars are scaled by
    /// it so that their size can be compared over time.
    pub fn max_cell_count(&self, kind: CellEventKind, unit: Option<&str>) -> u32 {
        self.cell_counts(self.duration_seconds, kind, unit)
            .into_values()
            .max()
            .unwrap_or_default()
    }

    /// The unit types born or died up to the time, the most frequent first, of a cell and player
    /// or of the whole map.
    pub fn unit_counts(
        &self,
        until_seconds: u32,
        kind: CellEventKind,
        cell_player: Option<CellPlayer>,
    ) -> Vec<(String, u32)> {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for event in self
            .events_until(until_seconds, kind, None)
            .filter(|event| {
                cell_player.is_none_or(|(cell, player)| {
                    (event.cell.x, event.cell.y) == cell && event.player == player
                })
            })
        {
            *counts.entry(event.unit.as_str()).or_default() += 1;
        }
        let mut counts: Vec<(String, u32)> = counts
            .into_iter()
            .map(|(unit, count)| (unit.to_string(), count))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        counts
    }

    /// The upgrades of the player finished up to the time.
    pub fn upgrades_until(&self, player: usize, until_seconds: u32) -> Vec<&UpgradeDone> {
        self.upgrades
            .iter()
            .filter(|upgrade| upgrade.player == player && upgrade.seconds <= until_seconds)
            .collect()
    }

    /// Whether the player finished the upgrade up to the time.
    fn has_upgrade(&self, player: usize, name: &str, until_seconds: u32) -> bool {
        self.upgrades.iter().any(|upgrade| {
            upgrade.player == player && upgrade.seconds <= until_seconds && upgrade.name == name
        })
    }

    /// The range of the units of [`RANGED_UNITS`] that killed up to the time, of one unit type
    /// or all of them, of one cell and player or all of them: one circle per unit and position.
    pub fn range_circles(
        &self,
        until_seconds: u32,
        unit: Option<&str>,
        cell_player: Option<CellPlayer>,
    ) -> Vec<RangeCircle> {
        let mut seen: std::collections::HashSet<(u32, i32, i32)> = Default::default();
        let mut circles = vec![];
        for kill in self
            .kills
            .iter()
            .take_while(|kill| kill.seconds <= until_seconds)
        {
            if unit.is_some_and(|unit| unit != kill.unit) {
                continue;
            }
            let cell = (
                kill.pos.0.max(0.) as u32 / AGGREGATE_CELLS,
                kill.pos.1.max(0.) as u32 / AGGREGATE_CELLS,
            );
            if cell_player.is_some_and(|(at, player)| at != cell || player != kill.player) {
                continue;
            }
            let Some((_, upgrade)) = RANGED_UNITS
                .iter()
                .find(|(variant, _)| *variant == kill.variant)
            else {
                continue;
            };
            let Some(range) = self.weapon_ranges.get(&kill.variant) else {
                continue;
            };
            // A unit firing from the same spot again is one circle.
            if !seen.insert((
                kill.tag_index,
                kill.pos.0.round() as i32,
                kill.pos.1.round() as i32,
            )) {
                continue;
            }
            let bonus = upgrade
                .filter(|(name, _)| self.has_upgrade(kill.player, name, kill.seconds))
                .map(|(_, bonus)| bonus)
                .unwrap_or_default();
            let (center, radius) = match (kill.variant.as_str(), kill.target) {
                ("LiberatorAG", Some(target)) => (target, LIBERATOR_ZONE_RADIUS),
                _ => (kill.pos, range + bonus + kill.radius),
            };
            circles.push(RangeCircle {
                player: kill.player,
                unit: kill.pos,
                center,
                radius,
            });
        }
        circles
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
    fn it_leaves_out_the_mode_switches() {
        for ability in [
            "BurrowLurkerMPDown",
            "BurrowLurkerMPUp",
            "SiegeMode",
            "Unsiege",
            "LiberatorAGTarget",
            "LiberatorAAMode",
            "WidowMineBurrow",
            "WidowMineUnburrow",
            "move",
            "",
        ] {
            assert!(is_generic_ability(ability), "{ability}");
        }
        for ability in ["Stimpack", "PsiStorm", "FungalGrowth", "YamatoGun"] {
            assert!(!is_generic_ability(ability), "{ability}");
        }
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
            .cell_counts(data.duration_seconds / 2, CellEventKind::Born, None)
            .values()
            .sum();
        let end: u32 = data
            .cell_counts(data.duration_seconds, CellEventKind::Born, None)
            .values()
            .sum();
        assert!(half <= end);
        assert_eq!(end as usize, born);
        // The counts of the unit types add up to the counts of the cells.
        let units: u32 = data
            .unit_counts(data.duration_seconds, CellEventKind::Born, None)
            .iter()
            .map(|(_, count)| count)
            .sum();
        assert_eq!(units, end);
        assert_eq!(data.time_steps().last(), Some(&data.duration_seconds));
        // A kill bar per kill, at the killer, the counts of the units panel are its kills.
        let kills: u32 = data
            .cell_counts(data.duration_seconds, CellEventKind::Kill, None)
            .values()
            .sum();
        assert_eq!(kills as usize, data.kills.len());
        assert_eq!(
            data.unit_counts(data.duration_seconds, CellEventKind::Kill, None)
                .iter()
                .find(|(unit, _)| unit == "LurkerMP")
                .map(|(_, count)| *count),
            Some(109)
        );
        // The burrowed Lurkers have a range, 8 in the balance data, more with Seismic Spines.
        assert_eq!(data.weapon_ranges.get("LurkerMPBurrowed"), Some(&8.));
        let circles = data.range_circles(data.duration_seconds, Some("LurkerMP"), None);
        assert!(!circles.is_empty());
        assert!(circles.iter().all(|circle| circle.radius >= 8.));
    }
}
