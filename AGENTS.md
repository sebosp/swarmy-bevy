# Working on swarmy-bevy

Instructions for the coding agents (Claude Code reads them through `CLAUDE.md`) and the people
working on swarmy-bevy, the 3D view of the maps and replays that swarmy starts. The decisions of
swarmy's interface are in swarmy's `AGENTS.md`; the ones below are those of this window.

## Skills to load

Always, as in swarmy (its `AGENTS.md` has the details): **frontend-design** for every panel,
**dataviz** for every color, bar, scale or legend, **deep-research** on learning platforms for
features about learning. Check readability (text size, contrast on the terrain), color vision
deficiencies, and that every key and color is in Help.

After a change of behavior, update the help text, the screenshots and the docs in the same
change, briefly.

## Decisions

- Every key and every color is explained in Help (the ? button at the top right), written for
  what the window shows (`help_ui::help_text`); a legend panel at the left names the colors of
  the bars, their keys and totals. Escape closes the panels.
- The size of the text and of the panels is a setting (Ctrl+= / Ctrl+- / Ctrl+0), kept in
  `swarmy-bevy/settings.json` of the config directory, as swarmy's zoom. The Settings button and
  panel were removed on 2026-11-21: the keys are known to everybody, Help lists them.
- The camera is Bevy's free camera (W/A/S/D, mouse drag, Space or E up, Q down; Space no longer
  pauses the time, 2026-11-13); a map-tool camera (orbit, pan) was
  tried on 2026-11-02 and rolled back by the author. The arrow keys belong to the replay (time)
  and the gizmos.
- When bars are drawn on the map (a replay) the terrain and the ramps are gray, darker the lower,
  so that the colors of the bars are not those of the terrain. The map lessons and the siege spots
  were removed on 2026-11-12 (not useful; swarmy no longer produced their files).
- The window must stay light on a laptop: entities share their meshes and materials (one per
  height, per color, per step of a color scale), no shadows. The window draws continuously:
  drawing on input only (`WinitSettings::desktop_app`) left the objects of the map (rocks,
  Xel'Naga towers) unloaded until an input, it was removed.
- The replay panel at the top right has the controls and the players: the winner first with a
  green triangle up, the loser a red triangle down (drawn images, the default font has no
  arrows), no "Win"/"Loss" words. Explanations go in Help, not in the panels.
- What is clicked shows in a floating panel at the bottom left (`selection_pane`), with the icons
  of the units of a bar; it never covers the buttons at the top right. Since 2026-11-21 it has no
  Close button (Escape empties it), folds with the arrow of its header (`collapsible`, drawn
  triangles) and is 19% of the width of the window (a `width`, a `max_width` let the wrapped text
  overflow it).
- 2026-11-21: a cell has a slot per player, their number rounded up to a power of 2 (3 players: 4
  slots, 2 by 2; `bar_slot`): a player's bar is always in its slot and never hides another. The
  base of a bar is square, a tenth of the area of the cell (0.316 of its side), at most 80% of its
  slot. (Before, a bar alone took the whole cell and the bars of a cell shared it.)
- 2026-11-21: the units panel has no "All" button: clicking the chosen unit again shows all.
- 2026-11-21: the upgrades left the players panel for a timeline at the bottom, right of the
  selection (80% of the width, folds; `replay::timeline`): a lane per player (a lane per upgrade
  was too tall for 20 to 30 upgrades a player), the icons where they finished, at most 2 rows a
  lane (closer ones overlap, so the replay panel above keeps its room on a laptop), no time text on
  the icons, a tooltip with the name, player and time, mm:ss ticks every 1, 2, 5... minutes (64 px
  apart at least), the axis ending 2 minutes after the last upgrade (they are in the first half of
  a long game), a light line at the time shown and the later upgrades faded (30%). Grid and axis
  are neutral grays, checked for contrast with the dataviz validator on the panel.
- 2026-11-21: the bars dropdown is Deaths, Births and Kills (`--replay-bars`, `--replay-unit`).
  Kills are counted at the position of the killer unit, as the most effective unit of the players
  panel counts them (enemy units that cost resources). In Kills, a clicked bar or the unit type
  chosen in the units panel draws the range of the static and sieged units around the places they
  killed from (`RANGED_UNITS`): the longest weapon range of the balance data of the build
  (SiegeTankSieged 13, LiberatorAG 10, LurkerMPBurrowed 8, PhotonCannon 7, SpineCrawler 7,
  PlanetaryFortress 6, MissileTurret 7; the same values are the fallback) plus the unit's radius,
  plus the upgrades the balance data does not give: Seismic Spines (`LurkerRange`) +2 and Hi-Sec
  Auto Tracking +1. A Liberator's circle is its zone, radius 5, around the target point of its
  last command, joined to it. The circles are gizmos in the player's color mixed with 35% white
  inside a near-black one, seen on the light and dark grays of the terrain.
- The View menu's Enable Map Graphics draws the models the map stores in its caches, placed by its
  `Objects` file on top of the terrain (`map_graphics`, `s2protocol::cache_handles::graphics`). Off
  by default and read when first enabled, unlit, with shared meshes and materials. Models and
  textures of the game itself are not in the caches: no model is drawn, a texture of the game is
  replaced by the Poly Haven texture of its kind (`ground_kind` of its name) and the quads of 2
  triangles without their image (decals, the `100spec` planes of Fighting Spirit) are left out.
  The models are unlit with a light baked in their vertex colors (`baked_light`).
- The View menu's Terrain Textures paints the ground with the textures of the map
  (`terrain_textures`): s2protocol reads the 8 textures of `t3Terrain.xml`, their weights
  (`t3TextureMasks`) and their colors fitted to the minimap
  (`s2protocol::cache_handles::terrain_textures`). The textures of the game are not in the caches:
  each one is drawn with a Poly Haven texture (CC0) of the same kind of ground, by the words of its
  name or its color (`terrain_textures::materials`), downloaded once through the Poly Haven API into
  `~/.cache/swarmy-bevy/polyhaven`, tinted with the color of the minimap (unless the minimap is
  drawn by hand: then the texture's own color). On by default for a map, off for a replay (its
  bars on the gray terrain); `--terrain-textures true|false`. One material for all the cells
  (`terrain_splat.wesl`, Bevy 0.20 shaders are WESL), the weights at 4 points per cell.
  With it the geysers (3x3) and the ore of the mineral fields are a rock (`RockTexture`).
- The mineral fields (2x1 cells) are a lump of ore with 7 hexagonal crystals (`minerals`), unlit
  with a light baked per facet, blue (gold the rich ones), glowing through the bloom and twinkling
  (`shine_crystals`); the geysers have a vent puffing gas (`geyser_gas`, see below); the start
  locations are landing pads, 2.75 cells of radius, a yellow ring, an H and 12 lights running
  around (`helipad`).
- The terrain is one mesh per chunk of 32x32 cells (`t3_height_map::chunk_mesh`) with only the
  faces that can be seen, the colors of the cells in the vertex colors: a cuboid per cell was
  40,000 entities visited every frame, and their hidden sides fought with the tops (flickering
  borders with the textures). The objects stand on `terrain_top`. The window draws at most 60
  frames a second with the focus, 10 without (`WinitSettings` in `main.rs`), a laptop's fans spun
  up at the display's rate. The splat shader textures the cliffs from the side.
- The ramps are slopes (one mesh per ramp and kind of cell), between the levels a few cells down and
  up the ramp, not the min/max around it (other plateaus); the cubes under them are lowered.
- The View menu's Pathing (`--pathing`, `pathing`) draws the ground a unit can walk on, from
  `s2protocol::cache_handles::pathing`, in one mesh of vertex colors rebuilt when the time
  changes: aqua (faint) the open ground, orange the cells of the buildings, minerals, geysers and
  rocks standing at the time shown, violet the areas they close off (gaps of less than 4 cells
  left out). Colors checked with the dataviz validator on the dark terrain. A replay on a local
  map (AI Arena) has no cache handles: its `.SC2Map`, renamed `<id>.s2ma` in the map caches
  directory, is given with `--map-cache-ids` (2026-11-21).
- 2026-11-21: the most effective unit of a player (players panel) is clickable as the units of
  the units panel: it shows the bars of its kills (switching to Kills). The bars of a unit chosen
  in a panel breathe: the emissive of the shared material of each player rises and falls (0.6 to
  2.4, 1.6 s) through the bloom, a few materials changed a frame, no mesh rebuilt, at the frame
  rate of the window.
- 2026-11-21: the most effective ability leaves out the mode switches (`MODE_SWITCHES` of
  `replay::is_generic_ability`: burrow, siege/unsiege, modes, LiberatorAGTarget, morphs, lift/land):
  the last command of a burrowed Lurker or a sieged Tank is the switch, not what killed.
- 2026-11-21: the destructible rocks, debris and plates (`rocks`, by their name: `Destructible*`,
  `Unbuildable*`, `*Debris*`) are a pile of rough boulders, of rusty metal chunks or a cover of
  hexagonal plates on a dark patch of rubble, the size of their footprint (6x6, 4x4, 2x2, Huge 8,
  along the diagonal for the diagonal ones, BLUR/ULBR). Procedural, 3 variants of each mesh with
  the light baked, unlit, shared; they no longer need a material in the glTF.
- 2026-11-21: a no fly zone (`no_fly_zone`) is a red "no" sign on the ground (a ring crossed by a
  bar, glowing a little) under a faint red column of air (alpha 0.07, 14 cells) with 3 fading
  hoops: red with the shape of the sign, not color alone; meshes shared per radius.
- 2026-11-21: a geyser is a ring of rock around its glowing vent, no block (the green cube is
  gone); the rich geysers' vent and puffs are violet, the others green.
- 2026-11-21: the Auto-rotate button (left of ?) turns the camera around the point of the ground
  in the middle of the view, a turn in 90 s; the free camera is paused meanwhile and takes the new
  direction back. W A S D, Q, E, Space, M, a right drag or the wheel stop it, as Stop rotating.
- `--screenshot <file>` renders and exits, to check a change without a display (Xvfb and
  lavapipe in a container work).

## Code

- `cargo fmt`, `cargo clippy` without warnings, `cargo test`; compile on the desktops, not on
  the laptop.
