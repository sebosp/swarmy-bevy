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
- The size of the text and of the panels is a setting (Settings, Ctrl+= / Ctrl+- / Ctrl+0),
  kept in `swarmy-bevy/settings.json` of the config directory, as swarmy's zoom.
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
  of the units of a bar; it never covers the buttons at the top right.
- A cell has one bar per player with units there at the current time: alone, a bar takes the
  whole cell.
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
  (`shine_crystals`); the geysers have a green vent puffing gas (`geyser_gas`); the start
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
- `--screenshot <file>` renders and exits, to check a change without a display (Xvfb and
  lavapipe in a container work).

## Code

- `cargo fmt`, `cargo clippy` without warnings, `cargo test`; compile on the desktops, not on
  the laptop.
