# Marching drill editor: 2D first

Historical prototype plan. Current features, platform requirements, and release procedures appear in [README.md](README.md) and [docs/RELEASING.md](docs/RELEASING.md).

Build a desktop Rust app for writing marching band drill on a football field. The first usable version lets a designer place performers, define sets by count, watch movement between sets, and save the show. PyWare is a reference for the workflow, not a file format or UI specification. Its [feature page](https://www.pyware.com/3d/) describes performer placement, formations, count tracks, animation, and coordinate printouts.

## First usable version

1. Draw a scaled football field with yard lines, hashes, sidelines, and a visible step grid. Support pan and zoom. Store positions in field coordinates, independent of screen pixels.
2. Add performers with stable IDs, section names, and labels. Place dots by clicking, drag one or a selection, and undo or redo edits.
3. Create ordered sets. Give each transition a positive count length. Keep a position for each performer at each set; copy the prior set when creating the next one.
4. Scrub a count timeline and play transitions. Start with straight line movement between sets, including holds when start and end positions match. Show paths and flag large step sizes.
5. Save and open a native project file. Export a printable chart for one set with labels, field markings, and a set number. A simple PDF or image export is enough for this milestone.

## Implementation sequence

- **Foundation:** Create a Cargo project with a desktop window and a field canvas. Define field dimensions and coordinate conversion in one module. Verify that zooming and resizing never change stored positions.
- **Editor:** Add performer creation, selection, dragging, labels, and undo/redo. Give performers stable IDs so a label change does not break their positions across sets.
- **Timeline:** Add sets, count lengths, copy-forward positions, scrubbing, and playback. Define the exact count convention in code and in the UI before adding transitions.
- **Persistence:** Add a versioned project schema and round-trip save/open checks. Prompt before discarding unsaved edits.
- **Output:** Export set charts and show coordinates relative to yard lines and hashes. Check the printed chart against the editor at the same set.

## Data model

`Show` contains field settings, a cast list, and ordered sets. Each `Performer` has an ID, label, and section. Each `Set` has a title, transition count, and a map from performer ID to 2D field position. Rendering interpolates between adjacent sets for a requested count; it does not change saved positions. Store field positions in steps so yard line and hash coordinates can be calculated from one field definition.

## Decisions to settle before coding

- Desktop target: start with Linux, then decide whether Windows and macOS builds belong in the first release.
- Field template: use a standard outdoor football field first; make yard line and hash dimensions configurable later.
- UI toolkit: use `egui` with `eframe` and its `wgpu` renderer for the desktop window and controls. Draw the field in an egui canvas; keep PNG chart export separate.
- Native file format: use a versioned, readable format initially. PyWare import/export can wait until its format and permitted interoperability path are understood.

## First milestone is done when

A designer can create a small cast, arrange two sets, assign transition counts, scrub and play the movement, save and reopen the show without losing positions, and export a legible chart for either set.

After that, add line and arc formation tools, curved paths, collision checks, section controls, coordinate sheets, and music sync in response to real drill-writing use.
