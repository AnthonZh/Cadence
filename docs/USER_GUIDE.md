# User Guide

## Project Hub and documents

The Project Hub provides New Drill, Open Drill, Return to Workspace, and a persistent recent-project list. Open and Save As use the native desktop file chooser. The File menu can return to the hub without closing the active document. The window title shows the current filename and an asterisk when the document has unsaved changes.

Cadence documents use the `.mdrill` extension. The format begins with the versioned `MARCHING_DRILL_DOCUMENT` header followed by a human-readable JSON payload, giving the application an identifiable, migratable container without making recovery difficult. Existing `.drill.json` and raw JSON projects still open; they are imported as unsaved documents so saving creates a new `.mdrill` instead of overwriting the legacy file.

The application window icon uses the field-grid mark in [cadence-icon.svg](../assets/cadence-icon.svg).

## Controls

| Action | Control |
| --- | --- |
| Add a performer | Choose the `+` tool or press P, then click the field |
| Select performers | Click, Shift-click, or drag a selection box |
| Resolve overlapping selections | Clicking a performer on a guide selects both and opens Performer/Guide/Both choices |
| Assign an instrument to a group | Select the performers, then choose an instrument in the inspector |
| Draw formation guides | Choose `/`, `O`, or `~` for a line, circle, or spline, then drag on the field |
| Add other formations | Open **Tools → Add Shape…** or press F4 |
| Snap performers to a guide | Select a guide and performers, then use Project or Compass Match in the inspector |
| Move selected performers | Drag a selected mark or use the arrow keys; Shift+arrow moves four steps |
| Fine positioning | Edit dot X/Y and guide geometry in yards; spacing and nudge size remain in steps |
| Position from field references | For one selected performer, choose a yard line or hash/sideline and enter a signed step offset |
| Add a symmetry construction axis | **Tools → Add symmetry axis** or S; edit its endpoints and threshold in the inspector |
| Rename a performer or section | Edit the floating inspector |
| Remove selected performers | Delete |
| Pan / zoom | Right-drag to pan; wheel zooms toward the cursor; F fits the field |
| Add or switch sets | Set menu; Page Up/Page Down switches sets |
| Change transition length | Edit **Counts to next** in the inspector; existing subsets set the safe minimum |
| Select an exact beat | Enter the global beat in the timeline or drag its playhead; optional snapping supports fractional beats |
| Add an intermediate subset | Move to a beat between sets, then use **Set → Add subset here** or Ctrl+Shift+Enter |
| Play or pause | Toolbar or Space |
| Scrub | Drag the playhead in the bottom count ruler |
| Attach and synchronize music | **Perform → Music Setup…**; choose a track, BPM, and count-zero audio offset |
| Export personal drill books | **File → Export Performer Packets…** and choose the destination folder |
| Save, open, export, undo, redo | File/Edit menus, toolbar, or standard shortcuts |

## Keyboard shortcuts

Open **Edit → Keyboard Shortcuts…** to change any binding. Click a binding and press the new shortcut; press Escape to cancel recording or Backspace to leave the action unassigned. Conflicting bindings are rejected with the name of the action already using them. Changes are saved immediately in the per-user configuration directory, and **Restore Defaults** resets the complete map.

The default map includes Ctrl+N/O/S/E for file operations, Ctrl+Z/Y for history, Ctrl+A and Escape for selection, V/P/H for basic tools, I/C/B for line, circle, and spline guides, J/Shift+J for projection and compass matching, K for grid snapping, F/G/L for field display, 1/2 for the floating palettes, Ctrl+Enter to add a set, Ctrl+Shift+Enter to add a subset, Shift+Delete to delete the active subset, Page Up/Page Down for set navigation, Space/Home for playback, and arrow or Shift+arrow nudging.

## Sketch selection tools

The workspace command bar groups File, Edit, Place, Match, View, Set and playback tools. Monochrome icons show tooltips and selection states; unavailable edit/match commands are disabled. The options strip contains document/set context, selection count, grid snap and nudge settings, and panel visibility controls. Transform, path, follow, timing, gate and constraint icons open the corresponding tab in the resizable right-side Tool Properties panel. Ctrl/Cmd+T opens that panel as well. The light-gray theme and field canvas remain unchanged in character.

Edit → Copy / Paste (Ctrl/Cmd+C and Ctrl/Cmd+V) copies selected dots and the selected guide. Copies retain instruments, receive unique performer identities, and paste with a two-step offset. Copied performers join the roster across all sets.

Edit → Transform / Rotation Drill (Ctrl/Cmd+T) provides translation in yards, scale, and rotation in degrees. Dots and geometry share the guide's pivot or the dot selection's bounding-box center. Custom pivot accepts exact yard coordinates. Operations support undo.

**Rotate** works on any selected performers without a formation guide. Enter an angle (90° by default), use the selection center or enter a custom pivot, then choose **Generate rotation**. Intermediate positions follow circular paths into the next subset or set. The Gate tab also supports combined translation and rotation.

Enable **Travel along a route** to move dots along a drawn route or an optional selected guide. Route travel is a signed fraction of total path length. Closed paths wrap; open paths reject travel past an endpoint. Each dot retains its offset from the path. Path movement does not require assigning performers to a formation shape.

## Counts and subsets

The Tool Properties panel (Ctrl/Cmd+T) also provides:

- **Transition timing**: per-performer holds, movement counts, staggered starts in performer ID order, and half/regular/double pace. Duration is divided by pace; any remaining time is an ending hold. Timing applies to the whole outgoing set transition, including its existing path through subsets. Reset restores ordinary timing. Hold entire transition replaces the selected performers' outgoing path with a stationary hold. Changing set counts scales the timing proportionally.
- **Gate / Pinwheel / Float**: rotate a formation about the selected/custom pivot through actual curved paths, translate it as a rigid float, or combine rotation and translation using the Move X/Y fields. Intermediate subsets preserve the motion; the operation ends at the next keyframe and creates a following set if necessary.
- **Sketch constraints**: persistently bind ordered dots to guide spots, fix dot positions or a guide pivot, mirror ordered performer pairs about a construction axis, or make circles concentric. A line binding maintains collinearity; a circle binding maintains its circular form. Guide spot-count and spacing settings control intervals. Constraints belong to the active set/subset, reapply while editing it, save in the document, and support undo/removal. Competing drivers are rejected. Symmetry pairs use performer ID order, with the first of each pair driving the second.

Generated maneuvers do not run collision optimization. Constraints maintain editable keyframe geometry; they do not impose a continuous constraint solver over every playback frame.

**Follow the Leader** works on any selected file. Open the Follow tab, arrange the leader-first order with Up/Down, enter travel in steps, then generate the move. **Continue forward** extends the heading from the first follower through the leader, with no guide required. **Drawn route** uses independent waypoints; **Selected guide** optionally reuses existing geometry. Followers traverse the initial file and then the leader's route, preserving their intervals along the trail.

For a custom path, choose **Draw route**, click waypoints or drag on the field, then press Enter or choose **Finish**. Escape cancels route entry without clearing the performer selection. The Waypoints section supports coordinate edits in yards. Routes appear in dark red on the field. Route drafts remain in the editing session; generated positions save with the document.

Both movement tools end at the next subset or set and create a following set if needed. Scrub the timeline or press Space to inspect the result. Open-path endpoint overruns are rejected. Undo restores the previous transition. Collision optimization does not run on these moves.

The timeline accepts exact global beats, including fractional beats such as `10.5`. Beat snapping is optional and its interval is editable from `0.125` to `8` beats. A subset is an intermediate formation keyframe inside a set-to-set transition. Select performers before adding one to keyframe only those performers; with no selection, the subset captures the full ensemble. Clicking a subset marker selects it. Dragging, nudging, guide projection, and coordinate editing then update that subset while the surrounding full sets remain unchanged. The inspector can rename the active subset or move it to another exact count. Playback interpolates through all subset positions in beat order.

## Music synchronization

Music Setup stores an external MP3, WAV, OGG, FLAC, M4A, or AAC path with the `.mdrill` document. BPM defines playback speed in drill counts: one count equals one beat. The non-negative audio offset is the track timestamp aligned with count zero, which allows trimming silence or count-off material. Playback uses Windows Media on Windows, AVFoundation on macOS, and `ffplay` on Linux. Pausing, scrubbing, opening another document, or changing timing stops it cleanly. Drill animation still works when no music is attached. Codec availability follows the host media stack; MP3, WAV, M4A, and AAC are the most portable choices.

## Performer drill packets

**Export Performer Packets…** creates a `performer-drill-packets` folder containing one long PDF drill book per performer. Each full set and subset receives its own letter-size page with the performer and section, global and transition counts, BPM-derived music timestamp, personalized field chart, highlighted incoming path, conventional side/yard-line/hash coordinate, raw yard coordinate, move distance, step size, and rehearsal-note lines. This follows the personalized drill-book workflow while keeping each performer’s entire show in a single packet.

The coordinate terminology and included fields follow the Drill Design Solutions recommendation to print performer IDs, set numbers, transition counts, and conventional side/yard-line/hash coordinates at quarter-step resolution: <https://www.drilldesignsolutions.com/_files/ugd/8db4d4_e369204a28af44c0bebd3f8ce063527b.pdf>.

## Formation shapes

The shape palette provides center/radius arcs, exact three-point circular arcs, blocks/grids, outline or filled polygons, spirals, editable freeform polylines, fans/wedges, concentric rings, chevrons, bitmap text, and imported outlines. Every shape participates in guide selection, projection, compass matching, spot-order reversal, undo/redo, and project persistence.

The inspector exposes each shape's meaningful parameters: angles and radius for arcs, rows/columns/staggering for blocks, sides and fill for polygons, radii and turns for spirals, editable vertices for paths, ray and ring counts for compound shapes, and text/cell size for text formations.

Selected shapes can be translated, rotated, uniformly scaled, mirrored, duplicated at an offset, split into paths, joined to the nearest path, or converted to a spline. **Save Prefab** writes `formation-prefab.json`; **Insert Prefab** places that saved formation at the cursor.

To import an outline, create `formation-outline.txt` with one `X,Y` yard-coordinate pair per line, then choose **Import formation-outline.txt** in the shape palette. Imported points remain editable and may be open or closed.

Bulk instrument assignment automatically gives every selected performer a unique conventional label—for example `FL1`, `T2`, `SD1`, or `CG3`. Existing labels are skipped, and the whole assignment can be undone in one step. A single generated label can still be edited directly in the inspector.

## Symmetry construction axes

Symmetry axes are persistent dashed construction lines stored with each set and copied forward when a new set is added. They remain visible and selectable on the field but do not create performer spots or become formation guides. Each arbitrary two-point axis has an editable name, endpoints in yards, compass mode, and threshold in steps.

When Compass Match runs, it adds a **Symmetry: _axis name_** proposal for every applicable construction axis. **Perfect** mode prioritizes mirrored destination pairs across the axis. **Try** mode constrains only source pairs whose reflected positions are within the chosen threshold, allowing the rest of the performers to use ordinary travel optimization. The symmetry solver jointly optimizes mirrored pair assignments and two-step performer clearance, including pair-preserving swaps that remove collisions without breaking symmetry. The proposal reports any unavoidable symmetry or clearance conflicts alongside travel metrics; performers move only after the compass proposal is approved.

Formation guides are stored per set and have an editable spot count. By default, the spots fill the whole line, circle, or spline. Turn off **Fill guide** to center the spots at an exact step interval instead. **Project** places selected performers uniformly on those configured spots. **Compass Match** requires the performer and spot counts to agree, then uses positions from the immediately preceding set or subset to calculate the one-to-one assignment with the least total transition travel. When a subset is active, approval writes the destinations into that subset instead of changing the full set. The proposed paths and targets are shown before anything moves; each performer's destination can be changed in the proposal window, which swaps the conflicting assignment automatically. Approve or cancel the complete match when ready.

Compass proposals include several plans: **Shortest total** minimizes total travel, **Even effort** minimizes the longest individual move before minimizing total travel, **Keep order** preserves rank along the source and destination formations, and **Avoid collisions** searches target swaps that reduce simultaneous passes within two steps while keeping travel within a reasonable budget. **Curve left**, **Curve right**, and the curved collision-avoidance variants create two intermediate subsets when approved, so timeline playback follows the proposed flow. The proposal reports total travel, the longest individual route, and detected clearance conflicts.

Compass scope defaults to **Incoming only**. It matches into the selected keyframe and carries the resulting performer-to-spot assignment through later sets and subsets, preserving downstream formation geometry and flow. Choose **Both sides** when the active keyframe should independently reshape both adjacent transitions.

For a line-to-line move, arrange the source set, create the next set, draw the destination guide with the same number of spots, select the source performers and destination guide, then choose **Compass Match**. The prior set remains unchanged, so timeline playback animates the automatically matched movement.

Step-grid snapping defaults on. Its interval is editable in the inspector and applies when performers are placed, when guides are drawn, and when a performer drag is completed. It can be toggled with K or from the View menu.

The editor displays field coordinates in standard football-field yards: X runs from 0 to 100 yards and Y from 0 to 53⅓ yards. Project files continue storing eight-to-five step coordinates internally, so existing drill files remain compatible and marching intervals stay exact.

The inspector also supports traditional reference coordinates. X can be entered as signed steps from any 5-yard line. Y can be entered as signed steps from the front sideline, front hash, back hash, or back sideline. Positive X points toward the 100-yard end; positive Y points toward the back sideline. The field uses college hashes 20 yards from each sideline.

Closing the application window with unsaved changes opens a Save and Close, Discard and Close, or Cancel prompt. A failed save keeps the application open.

The floating preview and inspector can be dragged or hidden from the Window menu. An orange line shows the selected performer's next transition; a dark red ring marks movement longer than one step per count. Positions use an eight-to-five step grid. Documents use the versioned `.mdrill` format.
