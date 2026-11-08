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
- The camera is Bevy's free camera (W/A/S/D, mouse drag); a map-tool camera (orbit, pan) was
  tried on 2026-11-02 and rolled back by the author. The arrow keys belong to the replay (time)
  and the gizmos.
- When bars are drawn on the map (a replay, the map lessons, the siege spots) the terrain and the
  ramps are gray, darker the lower, so that the colors of the bars are not those of the terrain.
  The layers of the map lessons are red, blue, magenta and amber.
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
  textures of the game itself are not in the caches: no model is drawn, or it is gray.
- `--screenshot <file>` renders and exits, to check a change without a display (Xvfb and
  lavapipe in a container work).

## Code

- `cargo fmt`, `cargo clippy` without warnings, `cargo test`; compile on the desktops, not on
  the laptop.
