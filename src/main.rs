#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod chart;
mod components;
mod drill_sheet;
mod field;
mod maneuvers;
mod model;
mod motion_input;
mod platform;
mod shortcuts;
mod theme;
mod workspace;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Layout, PointerButton, Pos2, Rect, Sense, Stroke, Vec2,
};
use model::{
    FILE_EXTENSION, FormationGuide, GuideKind, Point, Show, Subset, SymmetryAxis, SymmetryMode,
};
use shortcuts::{Action, Shortcut, ShortcutConfig};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Child,
    time::{Duration, Instant},
};

const FIELD_W: f32 = field::LENGTH_STEPS;
const FIELD_H: f32 = field::WIDTH_STEPS;
const SHORTCUT_FILE: &str = "shortcuts.json";
const RECENT_FILE: &str = "recent-projects.json";
const COMPASS_CLEARANCE_STEPS: f32 = 2.0;
const COMPASS_TRAVEL_BUDGET: f32 = 1.35;
const COMPASS_COLLISION_PASSES: usize = 6;
const SYMMETRY_AXIS_EPSILON: f32 = 0.25;
const INSTRUMENTS: &[&str] = &[
    "Piccolo",
    "Flute",
    "Clarinet",
    "Bass Clarinet",
    "Alto Saxophone",
    "Tenor Saxophone",
    "Baritone Saxophone",
    "Trumpet",
    "Mellophone",
    "Trombone",
    "Baritone",
    "Euphonium",
    "Tuba",
    "Sousaphone",
    "Snare Drum",
    "Tenor Drums",
    "Bass Drum",
    "Cymbals",
    "Front Ensemble",
    "Color Guard",
    "Drum Major",
];

#[derive(Clone, Copy)]
struct FieldView {
    origin: Pos2,
    scale: f32,
}
impl FieldView {
    fn screen(self, point: Point) -> Pos2 {
        self.origin + Vec2::new(point.x * self.scale, point.y * self.scale)
    }
    fn field(self, point: Pos2) -> Point {
        Point {
            x: (point.x - self.origin.x) / self.scale,
            y: (point.y - self.origin.y) / self.scale,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct ReferenceEditor {
    performer_id: u32,
    source_point: Point,
    x_reference_yards: f32,
    x_offset_steps: f32,
    y_reference: field::VerticalReference,
    y_offset_steps: f32,
}

#[derive(Clone, PartialEq, Eq)]
struct CompassEntry {
    performer_id: u32,
    target_index: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum CompassRoute {
    Straight,
    Curve(f32),
}

#[derive(Clone, Copy, PartialEq)]
enum CompassScope {
    IncomingOnly,
    BothSides,
}

#[derive(Clone)]
struct CompassCandidate {
    name: String,
    entries: Vec<CompassEntry>,
    route: CompassRoute,
    total_distance: f32,
    maximum_distance: f32,
    collision_risks: usize,
    symmetry_conflicts: Option<usize>,
    symmetry_error: f32,
    symmetry_axis: Option<SymmetryAxis>,
}

#[derive(Clone, Copy)]
struct CompassInterval {
    set_index: usize,
    source_offset: f32,
    destination_offset: f32,
}

struct CompassSymmetryConstraint {
    pairs: Vec<(usize, usize)>,
    centers: Vec<usize>,
    tolerance: f32,
}

#[derive(Clone, Copy)]
struct CompassSymmetryScore {
    symmetry_conflicts: usize,
    collision_conflicts: usize,
    symmetry_error: f32,
    collision_penalty: f32,
    travel: f32,
}

#[derive(Clone)]
struct CompassPreview {
    candidates: Vec<CompassCandidate>,
    selected_candidate: usize,
    spots: Vec<Point>,
    source_positions: Vec<(u32, Point)>,
    original_destination_positions: Vec<(u32, Point)>,
    destination_set: usize,
    destination_subset: Option<u32>,
    interval: Option<CompassInterval>,
    scope: CompassScope,
}

#[derive(Clone, Copy)]
struct PickChooser {
    position: Pos2,
    performer_id: u32,
    guide_id: u32,
}

impl ReferenceEditor {
    fn from_point(performer_id: u32, point: Point) -> Self {
        let x_reference_yards = field::nearest_yard_line(point.x);
        let y_reference = field::nearest_vertical_reference(point.y);
        Self {
            performer_id,
            source_point: point,
            x_reference_yards,
            x_offset_steps: point.x - field::yards_to_steps(x_reference_yards),
            y_reference,
            y_offset_steps: point.y - field::yards_to_steps(y_reference.yards()),
        }
    }

    fn point(self) -> Point {
        Point {
            x: field::yards_to_steps(self.x_reference_yards) + self.x_offset_steps,
            y: field::yards_to_steps(self.y_reference.yards()) + self.y_offset_steps,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    MotionRoute,
    Select,
    Place,
    Pan,
    LineGuide,
    CircleGuide,
    SplineGuide,
}

#[derive(Clone, Copy)]
enum ShapePreset {
    Arc,
    ThreePointArc,
    Block,
    Polygon,
    Spiral,
    Polyline,
    Fan,
    Concentric,
    Chevron,
    Text,
    ImportedPath,
}

struct DrillApp {
    component_clipboard: Option<components::ComponentClipboard>,
    show_component_tools: bool,
    component_page: usize,
    component_scale: f32,
    component_angle: f32,
    component_pivot_override: Option<Point>,
    component_translation: Point,
    rotation_fraction: f32,
    rotation_degrees: f32,
    rotation_use_path: bool,
    motion_route: Vec<Point>,
    follow_route_mode: usize,
    follow_order: Vec<u32>,
    follow_travel_steps: f32,
    follow_reverse: bool,
    move_delay: f32,
    move_duration: f32,
    move_stagger: f32,
    move_pace: f32,
    show: Show,
    current: usize,
    playhead: f32,
    playing: bool,
    last_frame: Instant,
    selected: BTreeSet<u32>,
    tool: Tool,
    dragging_dots: bool,
    box_start: Option<Pos2>,
    box_current: Option<Pos2>,
    cursor_field: Option<Point>,
    zoom: f32,
    pan: Vec2,
    undo: Vec<Show>,
    redo: Vec<Show>,
    file_path: Option<PathBuf>,
    pending_open_path: Option<PathBuf>,
    recent_files: Vec<String>,
    show_project_hub: bool,
    status: String,
    dirty: bool,
    confirm_new: bool,
    confirm_open: bool,
    show_grid: bool,
    show_labels: bool,
    show_preview: bool,
    show_inspector: bool,
    shortcuts: ShortcutConfig,
    show_shortcut_editor: bool,
    recording_shortcut: Option<Action>,
    shortcut_status: String,
    selected_guide: Option<u32>,
    selected_symmetry_axis: Option<u32>,
    guide_start: Option<Point>,
    guide_current: Option<Point>,
    nudge_steps: f32,
    reference_editor: Option<ReferenceEditor>,
    snap_to_grid: bool,
    grid_snap_steps: f32,
    compass_preview: Option<CompassPreview>,
    pick_chooser: Option<PickChooser>,
    confirm_close: bool,
    allow_close: bool,
    show_shape_palette: bool,
    show_music_setup: bool,
    guide_offset_x_yards: f32,
    guide_offset_y_yards: f32,
    guide_rotation_degrees: f32,
    guide_scale: f32,
    active_subset: Option<u32>,
    snap_playhead: bool,
    beat_snap: f32,
    audio_process: Option<Child>,
}

impl Default for DrillApp {
    fn default() -> Self {
        Self {
            component_clipboard: None,
            show_component_tools: false,
            component_page: 0,
            component_scale: 1.0,
            component_angle: 0.0,
            component_pivot_override: None,
            component_translation: Point::default(),
            rotation_fraction: 0.25,
            rotation_degrees: 90.0,
            rotation_use_path: false,
            motion_route: Vec::new(),
            follow_route_mode: 0,
            follow_order: Vec::new(),
            follow_travel_steps: 16.0,
            follow_reverse: false,
            move_delay: 0.0,
            move_duration: 8.0,
            move_stagger: 0.0,
            move_pace: 1.0,
            show: Show::default(),
            current: 0,
            playhead: 0.0,
            playing: false,
            last_frame: Instant::now(),
            selected: BTreeSet::new(),
            tool: Tool::Select,
            dragging_dots: false,
            box_start: None,
            box_current: None,
            cursor_field: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            undo: vec![],
            redo: vec![],
            file_path: None,
            pending_open_path: None,
            recent_files: Self::load_recent_files(),
            show_project_hub: true,
            status: "Ready".into(),
            dirty: false,
            confirm_new: false,
            confirm_open: false,
            show_grid: true,
            show_labels: true,
            show_preview: true,
            show_inspector: true,
            shortcuts: {
                let config = platform::config_file(SHORTCUT_FILE);
                if config.exists() {
                    ShortcutConfig::load(&config)
                } else {
                    ShortcutConfig::load(Path::new(SHORTCUT_FILE))
                }
            },
            show_shortcut_editor: false,
            recording_shortcut: None,
            shortcut_status: String::new(),
            selected_guide: None,
            selected_symmetry_axis: None,
            guide_start: None,
            guide_current: None,
            nudge_steps: 1.0,
            reference_editor: None,
            snap_to_grid: true,
            grid_snap_steps: 1.0,
            compass_preview: None,
            pick_chooser: None,
            confirm_close: false,
            allow_close: false,
            show_shape_palette: false,
            show_music_setup: false,
            guide_offset_x_yards: 0.0,
            guide_offset_y_yards: 0.0,
            guide_rotation_degrees: 0.0,
            guide_scale: 1.0,
            active_subset: None,
            snap_playhead: true,
            beat_snap: 1.0,
            audio_process: None,
        }
    }
}

impl DrillApp {
    fn snapshot(&mut self) {
        self.undo.push(self.show.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }
    fn set_current(&mut self, index: usize) {
        self.current = index.min(self.show.sets.len() - 1);
        self.playhead = self.show.set_start(self.current);
        self.playing = false;
        self.selected_guide = None;
        self.selected_symmetry_axis = None;
        self.compass_preview = None;
        self.pick_chooser = None;
        self.active_subset = None;
    }
    fn follow_playhead(&mut self) {
        self.current = (0..self.show.sets.len())
            .rev()
            .find(|&i| self.playhead >= self.show.set_start(i))
            .unwrap_or(0);
        self.sync_active_subset();
    }
    fn sync_active_subset(&mut self) {
        if self.current + 1 >= self.show.sets.len() {
            self.active_subset = None;
            return;
        }
        let local = self.playhead - self.show.set_start(self.current);
        self.active_subset = self.show.sets[self.current]
            .subsets
            .iter()
            .find(|subset| (subset.offset_count - local).abs() < 0.001)
            .map(|subset| subset.id);
    }
    fn active_subset(&self) -> Option<&Subset> {
        let id = self.active_subset?;
        self.show.sets[self.current]
            .subsets
            .iter()
            .find(|subset| subset.id == id)
    }
    fn editable_position(&self, id: u32) -> Option<Point> {
        if let Some(subset) = self.active_subset() {
            subset
                .positions
                .get(&id)
                .copied()
                .or_else(|| self.show.position_at(id, self.playhead))
        } else {
            self.show.sets[self.current].positions.get(&id).copied()
        }
    }
    fn set_editable_position(&mut self, id: u32, point: Point) {
        if let Some(subset_id) = self.active_subset
            && let Some(subset) = self.show.sets[self.current]
                .subsets
                .iter_mut()
                .find(|subset| subset.id == subset_id)
        {
            subset.positions.insert(id, point);
        } else if let Some(saved) = self.show.sets[self.current].positions.get_mut(&id) {
            *saved = point;
        }
    }
    fn add_subset_at_playhead(&mut self) {
        if self.current + 1 >= self.show.sets.len() {
            self.status = "A subset must be between this set and a following set".into();
            return;
        }
        let local = self.playhead - self.show.set_start(self.current);
        let transition_counts = self.show.sets[self.current].counts_to_next as f32;
        if local <= 0.001 || local >= transition_counts - 0.001 {
            self.status = "Choose a beat strictly between the two sets".into();
            return;
        }
        if let Some(existing) = self.show.sets[self.current]
            .subsets
            .iter()
            .find(|subset| (subset.offset_count - local).abs() < 0.001)
        {
            self.active_subset = Some(existing.id);
            self.status = format!("Selected {}", existing.name);
            return;
        }
        let ids: Vec<u32> = if self.selected.is_empty() {
            self.show
                .performers
                .iter()
                .map(|performer| performer.id)
                .collect()
        } else {
            self.selected.iter().copied().collect()
        };
        let positions: BTreeMap<u32, Point> = ids
            .into_iter()
            .filter_map(|id| {
                self.show
                    .position_at(id, self.playhead)
                    .map(|point| (id, point))
            })
            .collect();
        self.snapshot();
        let id = self.show.next_subset_id;
        self.show.next_subset_id += 1;
        let subset = Subset {
            id,
            name: format!("Subset {id}"),
            offset_count: local,
            positions,
        };
        self.show.sets[self.current].subsets.push(subset);
        self.show.sets[self.current]
            .subsets
            .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
        self.active_subset = Some(id);
        self.status = format!("Created Subset {id} at count {local:.2}");
    }
    fn delete_active_subset(&mut self) {
        let Some(id) = self.active_subset else {
            return;
        };
        self.snapshot();
        self.show.sets[self.current]
            .subsets
            .retain(|subset| subset.id != id);
        self.active_subset = None;
        self.status = "Deleted subset".into();
    }
    fn undo(&mut self) {
        if let Some(old) = self.undo.pop() {
            self.redo.push(self.show.clone());
            self.show = old;
            self.current = self.current.min(self.show.sets.len() - 1);
            self.active_subset = None;
            self.dirty = true;
        }
    }
    fn redo(&mut self) {
        if let Some(new) = self.redo.pop() {
            self.undo.push(self.show.clone());
            self.show = new;
            self.current = self.current.min(self.show.sets.len() - 1);
            self.active_subset = None;
            self.dirty = true;
        }
    }
    fn save(&mut self) {
        if let Some(path) = self.file_path.clone() {
            self.save_to(path);
        } else {
            self.save_as();
        }
    }
    fn save_as(&mut self) {
        if let Some(path) = Self::choose_project_file(true) {
            self.save_to(path);
        }
    }
    fn save_to(&mut self, path: PathBuf) {
        match self.show.save(&path) {
            Ok(()) => {
                self.dirty = false;
                self.file_path = Some(path.clone());
                self.remember_file(&path);
                self.status = format!("Saved {}", path.display())
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }
    fn open(&mut self) {
        if let Some(path) = Self::choose_project_file(false) {
            self.request_open(path);
        }
    }
    fn request_open(&mut self, path: PathBuf) {
        if self.dirty {
            self.pending_open_path = Some(path);
            self.confirm_open = true;
        } else {
            self.open_path(path);
        }
    }
    fn open_now(&mut self) {
        self.confirm_open = false;
        if let Some(path) = self.pending_open_path.take() {
            self.open_path(path);
        }
    }
    fn open_path(&mut self, path: PathBuf) {
        match Show::load(&path) {
            Ok(show) => {
                self.stop_audio();
                self.motion_route.clear();
                self.follow_order.clear();
                self.tool = Tool::Select;
                self.playing = false;
                let native_document = path.extension().and_then(|extension| extension.to_str())
                    == Some(FILE_EXTENSION);
                self.show = show;
                self.file_path = native_document.then(|| path.clone());
                self.current = 0;
                self.playhead = 0.0;
                self.selected.clear();
                self.selected_symmetry_axis = None;
                self.active_subset = None;
                self.undo.clear();
                self.redo.clear();
                self.dirty = !native_document;
                self.show_project_hub = false;
                self.remember_file(&path);
                self.status = if native_document {
                    format!("Opened {}", path.display())
                } else {
                    "Imported legacy project; save to create an .mdrill document".into()
                }
            }
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }
    fn load_recent_files() -> Vec<String> {
        let config = platform::config_file(RECENT_FILE);
        fs::read(if config.exists() {
            config.as_path()
        } else {
            Path::new(RECENT_FILE)
        })
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
    }
    fn remember_file(&mut self, path: &Path) {
        let value = path.to_string_lossy().into_owned();
        self.recent_files.retain(|recent| recent != &value);
        self.recent_files.insert(0, value);
        self.recent_files.truncate(10);
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.recent_files) {
            let _ = fs::write(platform::config_file(RECENT_FILE), bytes);
        }
    }
    fn choose_project_file(save: bool) -> Option<PathBuf> {
        let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let start = if save {
            current.join(format!("Untitled.{FILE_EXTENSION}"))
        } else {
            current
        };
        let kind = if save {
            platform::DialogKind::SaveProject
        } else {
            platform::DialogKind::OpenProject
        };
        let mut path = platform::choose(kind, &start)?;
        if save && path.extension().and_then(|extension| extension.to_str()) != Some(FILE_EXTENSION)
        {
            path.set_extension(FILE_EXTENSION);
        }
        Some(path)
    }
    fn export(&mut self) {
        let path = PathBuf::from(format!("set-{}.png", self.current + 1));
        match chart::export(&self.show, self.current, &path) {
            Ok(()) => self.status = format!("Exported {}", path.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }
    fn export_performer_packets(&mut self) {
        let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let Some(directory) = platform::choose(platform::DialogKind::ExportFolder, &current) else {
            return;
        };
        let packet_directory = directory.join("performer-drill-packets");
        match drill_sheet::export_performer_packets(&self.show, &packet_directory) {
            Ok(count) => {
                self.status = format!(
                    "Exported {count} performer packet(s) to {}",
                    packet_directory.display()
                )
            }
            Err(error) => self.status = format!("Packet export failed: {error}"),
        }
    }
    fn add_set(&mut self) {
        self.snapshot();
        let i = self.show.add_set(self.current);
        self.set_current(i);
        self.status = format!("Created Set {}", i + 1);
    }
    fn add_shape(&mut self, preset: ShapePreset) {
        let center = self.cursor_field.unwrap_or(Point {
            x: FIELD_W * 0.5,
            y: FIELD_H * 0.5,
        });
        let kind = match preset {
            ShapePreset::Arc => GuideKind::Arc {
                center,
                radius: 12.0,
                start_degrees: 180.0,
                sweep_degrees: 180.0,
            },
            ShapePreset::ThreePointArc => GuideKind::ThreePointArc {
                start: Point {
                    x: center.x - 12.0,
                    y: center.y + 4.0,
                },
                through: Point {
                    x: center.x,
                    y: center.y - 8.0,
                },
                end: Point {
                    x: center.x + 12.0,
                    y: center.y + 4.0,
                },
            },
            ShapePreset::Block => GuideKind::Block {
                center,
                width: 20.0,
                height: 12.0,
                rows: 4,
                columns: 6,
                stagger: false,
                rotation_degrees: 0.0,
            },
            ShapePreset::Polygon => GuideKind::Polygon {
                center,
                radius: 12.0,
                sides: 5,
                rotation_degrees: -90.0,
                filled: false,
            },
            ShapePreset::Spiral => GuideKind::Spiral {
                center,
                start_radius: 2.0,
                end_radius: 16.0,
                turns: 2.0,
                rotation_degrees: -90.0,
            },
            ShapePreset::Polyline => GuideKind::Polyline {
                points: vec![
                    Point {
                        x: center.x - 15.0,
                        y: center.y + 6.0,
                    },
                    Point {
                        x: center.x - 5.0,
                        y: center.y - 6.0,
                    },
                    Point {
                        x: center.x + 5.0,
                        y: center.y + 6.0,
                    },
                    Point {
                        x: center.x + 15.0,
                        y: center.y - 6.0,
                    },
                ],
                closed: false,
            },
            ShapePreset::Fan => GuideKind::Fan {
                pivot: center,
                radius: 16.0,
                start_degrees: -150.0,
                sweep_degrees: 120.0,
                rays: 5,
                spots_per_ray: 4,
            },
            ShapePreset::Concentric => GuideKind::Concentric {
                center,
                start_radius: 5.0,
                ring_spacing: 5.0,
                rings: 3,
                spots_per_ring: 12,
            },
            ShapePreset::Chevron => GuideKind::Chevron {
                left: Point {
                    x: center.x - 15.0,
                    y: center.y + 8.0,
                },
                pivot: center,
                right: Point {
                    x: center.x + 15.0,
                    y: center.y + 8.0,
                },
            },
            ShapePreset::Text => GuideKind::Text {
                origin: Point {
                    x: center.x - 12.0,
                    y: center.y - 5.0,
                },
                text: "BAND".into(),
                cell_size: 1.5,
            },
            ShapePreset::ImportedPath => GuideKind::ImportedPath {
                points: vec![
                    Point {
                        x: center.x,
                        y: center.y - 10.0,
                    },
                    Point {
                        x: center.x + 10.0,
                        y: center.y,
                    },
                    Point {
                        x: center.x,
                        y: center.y + 10.0,
                    },
                    Point {
                        x: center.x - 10.0,
                        y: center.y,
                    },
                ],
                closed: true,
            },
        };
        self.snapshot();
        let id = self.show.next_guide_id();
        self.show.sets[self.current].guides.push(FormationGuide {
            id,
            spacing_steps: 2.0,
            spot_count: self.selected.len().max(8),
            fill_guide: true,
            reverse_spots: false,
            kind,
        });
        self.selected_guide = Some(id);
        self.show_inspector = true;
        self.status = "Added formation shape".into();
    }
    fn add_symmetry_axis(&mut self) {
        let x = self.cursor_field.map_or(FIELD_W * 0.5, |point| point.x);
        let id = self
            .show
            .sets
            .iter()
            .flat_map(|set| &set.symmetry_axes)
            .map(|axis| axis.id)
            .max()
            .unwrap_or(0)
            + 1;
        self.snapshot();
        self.show.sets[self.current]
            .symmetry_axes
            .push(SymmetryAxis {
                id,
                name: format!("Axis {id}"),
                start: Point { x, y: 0.0 },
                end: Point { x, y: FIELD_H },
                threshold_steps: 2.0,
                mode: SymmetryMode::Try,
            });
        self.selected_symmetry_axis = Some(id);
        self.selected_guide = None;
        self.show_inspector = true;
        self.status = format!("Added symmetry Axis {id}");
    }
    fn selected_symmetry_axis(&self) -> Option<SymmetryAxis> {
        let id = self.selected_symmetry_axis?;
        self.show.sets[self.current]
            .symmetry_axes
            .iter()
            .find(|axis| axis.id == id)
            .cloned()
    }
    fn symmetry_basis(axis: &SymmetryAxis) -> Option<(Point, Point)> {
        let dx = axis.end.x - axis.start.x;
        let dy = axis.end.y - axis.start.y;
        let length = (dx * dx + dy * dy).sqrt();
        (length > 0.001).then_some((
            Point {
                x: dx / length,
                y: dy / length,
            },
            Point {
                x: -dy / length,
                y: dx / length,
            },
        ))
    }
    fn symmetry_coordinates(axis: &SymmetryAxis, point: Point) -> Option<(f32, f32)> {
        let (along, normal) = Self::symmetry_basis(axis)?;
        let relative = Point {
            x: point.x - axis.start.x,
            y: point.y - axis.start.y,
        };
        Some((
            relative.x * along.x + relative.y * along.y,
            relative.x * normal.x + relative.y * normal.y,
        ))
    }
    fn symmetry_point(axis: &SymmetryAxis, along_value: f32, normal_value: f32) -> Option<Point> {
        let (along, normal) = Self::symmetry_basis(axis)?;
        Some(Point {
            x: (axis.start.x + along.x * along_value + normal.x * normal_value).clamp(0.0, FIELD_W),
            y: (axis.start.y + along.y * along_value + normal.y * normal_value).clamp(0.0, FIELD_H),
        })
    }
    fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.snapshot();
        self.show
            .performers
            .retain(|p| !self.selected.contains(&p.id));
        for set in &mut self.show.sets {
            set.positions.retain(|id, _| !self.selected.contains(id));
            for subset in &mut set.subsets {
                subset.positions.retain(|id, _| !self.selected.contains(id));
            }
        }
        self.selected.clear();
    }
    fn fit_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }
    fn view(&self, canvas: Rect) -> FieldView {
        let scale = ((canvas.width() - theme::CANVAS_INSET * 2.0) / FIELD_W)
            .min((canvas.height() - theme::CANVAS_INSET * 2.0) / FIELD_H)
            .max(1.0)
            * self.zoom;
        FieldView {
            origin: canvas.center() - Vec2::new(FIELD_W * scale, FIELD_H * scale) / 2.0 + self.pan,
            scale,
        }
    }
    fn hit_dot(&self, view: FieldView, pos: Pos2) -> Option<u32> {
        self.show.performers.iter().find_map(|dot| {
            self.show
                .position_at(dot.id, self.playhead)
                .filter(|p| view.screen(*p).distance(pos) < theme::HIT_RADIUS)
                .map(|_| dot.id)
        })
    }
    fn dot_color(&self, section: &str) -> Color32 {
        let s = section.to_ascii_lowercase();
        if s.contains("guard") || s.contains("color") {
            theme::DOT_GUARD
        } else if ["drum", "perc", "cymbal", "front ensemble"]
            .iter()
            .any(|instrument| s.contains(instrument))
        {
            theme::DOT_PERCUSSION
        } else if [
            "band",
            "wind",
            "brass",
            "flute",
            "piccolo",
            "clarinet",
            "saxophone",
            "trumpet",
            "mellophone",
            "horn",
            "trombone",
            "baritone",
            "euphonium",
            "tuba",
            "sousaphone",
        ]
        .iter()
        .any(|instrument| s.contains(instrument))
        {
            theme::DOT_BAND
        } else {
            theme::DOT_OTHER
        }
    }
    fn nudge(&mut self, delta: Point) {
        if self.selected.is_empty() {
            return;
        }
        self.snapshot();
        let updates: Vec<(u32, Point)> = self
            .selected
            .iter()
            .filter_map(|id| {
                self.editable_position(*id).map(|point| {
                    (
                        *id,
                        Point {
                            x: (point.x + delta.x).clamp(0.0, FIELD_W),
                            y: (point.y + delta.y).clamp(0.0, FIELD_H),
                        },
                    )
                })
            })
            .collect();
        for (id, point) in updates {
            self.set_editable_position(id, point);
        }
    }
    fn snap_point(&self, point: Point) -> Point {
        let snapped = if self.snap_to_grid {
            let interval = self.grid_snap_steps.max(0.125);
            Point {
                x: (point.x / interval).round() * interval,
                y: (point.y / interval).round() * interval,
            }
        } else {
            point
        };
        Point {
            x: snapped.x.clamp(0.0, FIELD_W),
            y: snapped.y.clamp(0.0, FIELD_H),
        }
    }
    fn project_to_guide(guide: &FormationGuide, point: Point) -> Point {
        match guide.kind {
            GuideKind::Line { start, end } => Self::project_to_segment(start, end, point),
            GuideKind::Circle { center, radius } => {
                let dx = point.x - center.x;
                let dy = point.y - center.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance <= f32::EPSILON {
                    Point {
                        x: center.x,
                        y: center.y - radius,
                    }
                } else {
                    Point {
                        x: center.x + dx / distance * radius,
                        y: center.y + dy / distance * radius,
                    }
                }
            }
            _ => Self::guide_paths(guide)
                .iter()
                .flat_map(|path| path.windows(2))
                .map(|segment| Self::project_to_segment(segment[0], segment[1], point))
                .chain(Self::guide_spots(guide))
                .min_by(|a, b| a.distance(point).total_cmp(&b.distance(point)))
                .unwrap_or(point),
        }
    }
    fn project_to_segment(start: Point, end: Point, point: Point) -> Point {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f32::EPSILON {
            return start;
        }
        let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared)
            .clamp(0.0, 1.0);
        Point {
            x: start.x + dx * t,
            y: start.y + dy * t,
        }
    }
    fn cubic_point(start: Point, control_1: Point, control_2: Point, end: Point, t: f32) -> Point {
        let u = 1.0 - t;
        Point {
            x: u.powi(3) * start.x
                + 3.0 * u.powi(2) * t * control_1.x
                + 3.0 * u * t.powi(2) * control_2.x
                + t.powi(3) * end.x,
            y: u.powi(3) * start.y
                + 3.0 * u.powi(2) * t * control_1.y
                + 3.0 * u * t.powi(2) * control_2.y
                + t.powi(3) * end.y,
        }
    }
    fn rotated_point(center: Point, point: Point, degrees: f32) -> Point {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        let x = point.x - center.x;
        let y = point.y - center.y;
        Point {
            x: center.x + x * cos - y * sin,
            y: center.y + x * sin + y * cos,
        }
    }
    fn circle_path(center: Point, radius: f32, start: f32, sweep: f32) -> Vec<Point> {
        let segments = ((sweep.abs() / 360.0 * 96.0).ceil() as usize).clamp(8, 128);
        (0..=segments)
            .map(|index| {
                let angle = (start + sweep * index as f32 / segments as f32).to_radians();
                Point {
                    x: center.x + radius * angle.cos(),
                    y: center.y + radius * angle.sin(),
                }
            })
            .collect()
    }
    fn three_point_arc_geometry(
        start: Point,
        through: Point,
        end: Point,
    ) -> Option<(Point, f32, f32, f32)> {
        let denominator = 2.0
            * (start.x * (through.y - end.y)
                + through.x * (end.y - start.y)
                + end.x * (start.y - through.y));
        if denominator.abs() <= 0.0001 {
            return None;
        }
        let start_squared = start.x * start.x + start.y * start.y;
        let through_squared = through.x * through.x + through.y * through.y;
        let end_squared = end.x * end.x + end.y * end.y;
        let center = Point {
            x: (start_squared * (through.y - end.y)
                + through_squared * (end.y - start.y)
                + end_squared * (start.y - through.y))
                / denominator,
            y: (start_squared * (end.x - through.x)
                + through_squared * (start.x - end.x)
                + end_squared * (through.x - start.x))
                / denominator,
        };
        let angle = |point: Point| (point.y - center.y).atan2(point.x - center.x).to_degrees();
        let start_degrees = angle(start);
        let through_degrees = angle(through);
        let end_degrees = angle(end);
        let counter_clockwise_sweep = (end_degrees - start_degrees).rem_euclid(360.0);
        let counter_clockwise_to_through = (through_degrees - start_degrees).rem_euclid(360.0);
        let sweep_degrees = if counter_clockwise_to_through <= counter_clockwise_sweep + 0.001 {
            counter_clockwise_sweep
        } else {
            counter_clockwise_sweep - 360.0
        };
        Some((center, center.distance(start), start_degrees, sweep_degrees))
    }
    fn guide_paths(guide: &FormationGuide) -> Vec<Vec<Point>> {
        match &guide.kind {
            GuideKind::Line { start, end } => vec![vec![*start, *end]],
            GuideKind::Circle { center, radius } => {
                vec![Self::circle_path(*center, *radius, -90.0, 360.0)]
            }
            GuideKind::Spline {
                start,
                control_1,
                control_2,
                end,
            } => vec![
                (0..=64)
                    .map(|index| {
                        Self::cubic_point(*start, *control_1, *control_2, *end, index as f32 / 64.0)
                    })
                    .collect(),
            ],
            GuideKind::Arc {
                center,
                radius,
                start_degrees,
                sweep_degrees,
            } => vec![Self::circle_path(
                *center,
                *radius,
                *start_degrees,
                *sweep_degrees,
            )],
            GuideKind::ThreePointArc {
                start,
                through,
                end,
            } => {
                if let Some((center, radius, start_degrees, sweep_degrees)) =
                    Self::three_point_arc_geometry(*start, *through, *end)
                {
                    vec![Self::circle_path(
                        center,
                        radius,
                        start_degrees,
                        sweep_degrees,
                    )]
                } else {
                    vec![vec![*start, *through, *end]]
                }
            }
            GuideKind::Block {
                center,
                width,
                height,
                rotation_degrees,
                ..
            } => {
                let half_w = *width * 0.5;
                let half_h = *height * 0.5;
                let mut corners = vec![
                    Point {
                        x: center.x - half_w,
                        y: center.y - half_h,
                    },
                    Point {
                        x: center.x + half_w,
                        y: center.y - half_h,
                    },
                    Point {
                        x: center.x + half_w,
                        y: center.y + half_h,
                    },
                    Point {
                        x: center.x - half_w,
                        y: center.y + half_h,
                    },
                    Point {
                        x: center.x - half_w,
                        y: center.y - half_h,
                    },
                ];
                for point in &mut corners {
                    *point = Self::rotated_point(*center, *point, *rotation_degrees);
                }
                vec![corners]
            }
            GuideKind::Polygon {
                center,
                radius,
                sides,
                rotation_degrees,
                ..
            } => {
                let sides = (*sides).clamp(3, 32);
                let mut points: Vec<Point> = (0..sides)
                    .map(|index| {
                        let angle =
                            (rotation_degrees + index as f32 * 360.0 / sides as f32).to_radians();
                        Point {
                            x: center.x + radius * angle.cos(),
                            y: center.y + radius * angle.sin(),
                        }
                    })
                    .collect();
                points.push(points[0]);
                vec![points]
            }
            GuideKind::Spiral {
                center,
                start_radius,
                end_radius,
                turns,
                rotation_degrees,
            } => vec![
                (0..=128)
                    .map(|index| {
                        let t = index as f32 / 128.0;
                        let radius = start_radius + (end_radius - start_radius) * t;
                        let angle = (rotation_degrees + turns * 360.0 * t).to_radians();
                        Point {
                            x: center.x + radius * angle.cos(),
                            y: center.y + radius * angle.sin(),
                        }
                    })
                    .collect(),
            ],
            GuideKind::Polyline { points, closed } | GuideKind::ImportedPath { points, closed } => {
                let mut path = points.clone();
                if *closed && path.len() > 2 && path.first() != path.last() {
                    path.push(path[0]);
                }
                vec![path]
            }
            GuideKind::Fan {
                pivot,
                radius,
                start_degrees,
                sweep_degrees,
                rays,
                ..
            } => {
                let rays = (*rays).max(1);
                (0..rays)
                    .map(|index| {
                        let t = if rays == 1 {
                            0.5
                        } else {
                            index as f32 / (rays - 1) as f32
                        };
                        let angle = (start_degrees + sweep_degrees * t).to_radians();
                        vec![
                            *pivot,
                            Point {
                                x: pivot.x + radius * angle.cos(),
                                y: pivot.y + radius * angle.sin(),
                            },
                        ]
                    })
                    .collect()
            }
            GuideKind::Concentric {
                center,
                start_radius,
                ring_spacing,
                rings,
                ..
            } => (0..(*rings).max(1))
                .map(|index| {
                    Self::circle_path(
                        *center,
                        start_radius + ring_spacing * index as f32,
                        -90.0,
                        360.0,
                    )
                })
                .collect(),
            GuideKind::Chevron { left, pivot, right } => vec![vec![*left, *pivot, *right]],
            GuideKind::Text { .. } => vec![],
        }
    }
    fn point_on_polyline(points: &[Point], distance: f32) -> Point {
        let mut remaining = distance.max(0.0);
        for segment in points.windows(2) {
            let length = segment[0].distance(segment[1]);
            if remaining <= length || length <= f32::EPSILON {
                let t = if length <= f32::EPSILON {
                    0.0
                } else {
                    remaining / length
                };
                return segment[0].lerp(segment[1], t.clamp(0.0, 1.0));
            }
            remaining -= length;
        }
        points.last().copied().unwrap_or_default()
    }
    fn point_on_paths(paths: &[Vec<Point>], distance: f32) -> Point {
        let mut remaining = distance.max(0.0);
        for path in paths {
            let length: f32 = path
                .windows(2)
                .map(|segment| segment[0].distance(segment[1]))
                .sum();
            if remaining <= length {
                return Self::point_on_polyline(path, remaining);
            }
            remaining -= length;
        }
        paths
            .last()
            .and_then(|path| path.last())
            .copied()
            .unwrap_or_default()
    }
    fn evenly_spaced_on_paths(
        paths: &[Vec<Point>],
        count: usize,
        closed: bool,
        fill_guide: bool,
        requested_spacing: f32,
    ) -> Vec<Point> {
        let total_length: f32 = paths
            .iter()
            .flat_map(|path| path.windows(2))
            .map(|segment| segment[0].distance(segment[1]))
            .sum();
        if count <= 1 || total_length <= f32::EPSILON {
            return vec![Self::point_on_paths(paths, total_length * 0.5)];
        }
        let denominator = if closed { count } else { count - 1 };
        let capacity = total_length / denominator as f32;
        let spacing = if fill_guide {
            capacity
        } else {
            requested_spacing.max(0.25).min(capacity)
        };
        let first = if fill_guide || closed {
            0.0
        } else {
            (total_length - spacing * (count - 1) as f32) * 0.5
        };
        (0..count)
            .map(|index| Self::point_on_paths(paths, first + spacing * index as f32))
            .collect()
    }
    fn glyph_rows(character: char) -> [u8; 7] {
        match character.to_ascii_uppercase() {
            'A' => [14, 17, 17, 31, 17, 17, 17],
            'B' => [30, 17, 17, 30, 17, 17, 30],
            'C' => [14, 17, 16, 16, 16, 17, 14],
            'D' => [30, 17, 17, 17, 17, 17, 30],
            'E' => [31, 16, 16, 30, 16, 16, 31],
            'F' => [31, 16, 16, 30, 16, 16, 16],
            'G' => [14, 17, 16, 23, 17, 17, 14],
            'H' => [17, 17, 17, 31, 17, 17, 17],
            'I' => [31, 4, 4, 4, 4, 4, 31],
            'J' => [7, 2, 2, 2, 18, 18, 12],
            'K' => [17, 18, 20, 24, 20, 18, 17],
            'L' => [16, 16, 16, 16, 16, 16, 31],
            'M' => [17, 27, 21, 21, 17, 17, 17],
            'N' => [17, 25, 21, 19, 17, 17, 17],
            'O' => [14, 17, 17, 17, 17, 17, 14],
            'P' => [30, 17, 17, 30, 16, 16, 16],
            'Q' => [14, 17, 17, 17, 21, 18, 13],
            'R' => [30, 17, 17, 30, 20, 18, 17],
            'S' => [15, 16, 16, 14, 1, 1, 30],
            'T' => [31, 4, 4, 4, 4, 4, 4],
            'U' => [17, 17, 17, 17, 17, 17, 14],
            'V' => [17, 17, 17, 17, 17, 10, 4],
            'W' => [17, 17, 17, 21, 21, 21, 10],
            'X' => [17, 17, 10, 4, 10, 17, 17],
            'Y' => [17, 17, 10, 4, 4, 4, 4],
            'Z' => [31, 1, 2, 4, 8, 16, 31],
            '0' => [14, 17, 19, 21, 25, 17, 14],
            '1' => [4, 12, 4, 4, 4, 4, 14],
            '2' => [14, 17, 1, 2, 4, 8, 31],
            '3' => [30, 1, 1, 14, 1, 1, 30],
            '4' => [2, 6, 10, 18, 31, 2, 2],
            '5' => [31, 16, 16, 30, 1, 1, 30],
            '6' => [14, 16, 16, 30, 17, 17, 14],
            '7' => [31, 1, 2, 4, 8, 8, 8],
            '8' => [14, 17, 17, 14, 17, 17, 14],
            '9' => [14, 17, 17, 15, 1, 1, 14],
            ' ' => [0; 7],
            _ => [31, 17, 1, 2, 4, 0, 4],
        }
    }
    fn text_formation_spots(origin: Point, text: &str, cell_size: f32) -> Vec<Point> {
        let cell = cell_size.max(0.25);
        let mut points = Vec::new();
        for (character_index, character) in text.chars().take(24).enumerate() {
            for (row, bits) in Self::glyph_rows(character).into_iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        points.push(Point {
                            x: origin.x + (character_index as f32 * 6.0 + column as f32) * cell,
                            y: origin.y + row as f32 * cell,
                        });
                    }
                }
            }
        }
        points
    }
    fn guide_spots(guide: &FormationGuide) -> Vec<Point> {
        let count = guide.spot_count.clamp(1, 256);
        let mut spots = match &guide.kind {
            GuideKind::Block {
                center,
                width,
                height,
                rows,
                columns,
                stagger,
                rotation_degrees,
            } => {
                let rows = (*rows).clamp(1, 64);
                let columns = (*columns).clamp(1, 64);
                let x_step = if columns > 1 {
                    width / (columns - 1) as f32
                } else {
                    0.0
                };
                let y_step = if rows > 1 {
                    height / (rows - 1) as f32
                } else {
                    0.0
                };
                let mut points = Vec::with_capacity(rows * columns);
                for row in 0..rows {
                    for column in 0..columns {
                        let stagger_offset = if *stagger && row % 2 == 1 {
                            x_step * 0.5
                        } else {
                            0.0
                        };
                        let point = Point {
                            x: center.x - width * 0.5 + column as f32 * x_step + stagger_offset,
                            y: center.y - height * 0.5 + row as f32 * y_step,
                        };
                        points.push(Self::rotated_point(*center, point, *rotation_degrees));
                    }
                }
                points
            }
            GuideKind::Polygon {
                center,
                radius,
                sides,
                rotation_degrees,
                filled: true,
            } => {
                let sides = (*sides).clamp(3, 32);
                let mut points = vec![*center];
                let mut layer = 1;
                while points.len() < count {
                    let layer_count = sides * layer;
                    let layer_radius = radius * (layer as f32 / (layer + 1) as f32);
                    let mut vertices: Vec<Point> = (0..sides)
                        .map(|index| {
                            let angle = (rotation_degrees + index as f32 * 360.0 / sides as f32)
                                .to_radians();
                            Point {
                                x: center.x + layer_radius * angle.cos(),
                                y: center.y + layer_radius * angle.sin(),
                            }
                        })
                        .collect();
                    vertices.push(vertices[0]);
                    points.extend(Self::evenly_spaced_on_paths(
                        &[vertices],
                        layer_count,
                        true,
                        true,
                        1.0,
                    ));
                    layer += 1;
                }
                points.truncate(count);
                points
            }
            GuideKind::Fan {
                pivot,
                radius,
                start_degrees,
                sweep_degrees,
                rays,
                spots_per_ray,
            } => {
                let rays = (*rays).clamp(1, 64);
                let per_ray = (*spots_per_ray).clamp(1, 64);
                let mut points = Vec::with_capacity(rays * per_ray);
                for ray in 0..rays {
                    let t = if rays == 1 {
                        0.5
                    } else {
                        ray as f32 / (rays - 1) as f32
                    };
                    let angle = (start_degrees + sweep_degrees * t).to_radians();
                    for spot in 1..=per_ray {
                        let distance = radius * spot as f32 / per_ray as f32;
                        points.push(Point {
                            x: pivot.x + distance * angle.cos(),
                            y: pivot.y + distance * angle.sin(),
                        });
                    }
                }
                points
            }
            GuideKind::Concentric {
                center,
                start_radius,
                ring_spacing,
                rings,
                spots_per_ring,
            } => {
                let mut points = Vec::new();
                for ring in 0..(*rings).clamp(1, 32) {
                    let radius = start_radius + ring_spacing * ring as f32;
                    let ring_count = (*spots_per_ring).clamp(1, 128);
                    for spot in 0..ring_count {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + std::f32::consts::TAU * spot as f32 / ring_count as f32;
                        points.push(Point {
                            x: center.x + radius * angle.cos(),
                            y: center.y + radius * angle.sin(),
                        });
                    }
                }
                points
            }
            GuideKind::Text {
                origin,
                text,
                cell_size,
            } => Self::text_formation_spots(*origin, text, *cell_size),
            _ => {
                let closed = matches!(
                    guide.kind,
                    GuideKind::Circle { .. }
                        | GuideKind::Polygon { .. }
                        | GuideKind::Polyline { closed: true, .. }
                        | GuideKind::ImportedPath { closed: true, .. }
                );
                Self::evenly_spaced_on_paths(
                    &Self::guide_paths(guide),
                    count,
                    closed,
                    guide.fill_guide,
                    guide.spacing_steps,
                )
            }
        };
        if guide.reverse_spots {
            spots.reverse();
        }
        spots
    }
    fn guide_actual_spacing(guide: &FormationGuide) -> f32 {
        let spots = Self::guide_spots(guide);
        if spots.len() <= 1 {
            return 0.0;
        }
        spots
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum::<f32>()
            / (spots.len() - 1) as f32
    }
    fn guide_anchor(guide: &FormationGuide) -> Point {
        let spots = Self::guide_spots(guide);
        if spots.is_empty() {
            return Point::default();
        }
        let (x, y) = spots
            .iter()
            .fold((0.0, 0.0), |(x, y), point| (x + point.x, y + point.y));
        Point {
            x: x / spots.len() as f32,
            y: y / spots.len() as f32,
        }
    }
    fn transform_guide(
        guide: &mut FormationGuide,
        translation: Point,
        scale: f32,
        rotation_degrees: f32,
        mirror_x: bool,
        mirror_y: bool,
    ) {
        let anchor = Self::guide_anchor(guide);
        let sx = scale.max(0.01) * if mirror_x { -1.0 } else { 1.0 };
        let sy = scale.max(0.01) * if mirror_y { -1.0 } else { 1.0 };
        let transform_point = |point: &mut Point| {
            let scaled = Point {
                x: anchor.x + (point.x - anchor.x) * sx,
                y: anchor.y + (point.y - anchor.y) * sy,
            };
            let rotated = Self::rotated_point(anchor, scaled, rotation_degrees);
            point.x = (rotated.x + translation.x).clamp(0.0, FIELD_W);
            point.y = (rotated.y + translation.y).clamp(0.0, FIELD_H);
        };
        let magnitude = scale.max(0.01);
        match &mut guide.kind {
            GuideKind::Line { start, end } => {
                transform_point(start);
                transform_point(end);
            }
            GuideKind::Circle { center, radius } => {
                transform_point(center);
                *radius *= magnitude;
            }
            GuideKind::Spline {
                start,
                control_1,
                control_2,
                end,
            } => {
                for point in [start, control_1, control_2, end] {
                    transform_point(point);
                }
            }
            GuideKind::Arc {
                center,
                radius,
                start_degrees,
                sweep_degrees,
            } => {
                transform_point(center);
                *radius *= magnitude;
                *start_degrees += rotation_degrees;
                if mirror_x {
                    *start_degrees = 180.0 - *start_degrees;
                    *sweep_degrees = -*sweep_degrees;
                }
                if mirror_y {
                    *start_degrees = -*start_degrees;
                    *sweep_degrees = -*sweep_degrees;
                }
            }
            GuideKind::ThreePointArc {
                start,
                through,
                end,
            } => {
                for point in [start, through, end] {
                    transform_point(point);
                }
            }
            GuideKind::Block {
                center,
                width,
                height,
                rotation_degrees: rotation,
                ..
            } => {
                transform_point(center);
                *width *= magnitude;
                *height *= magnitude;
                *rotation += rotation_degrees;
                if mirror_x ^ mirror_y {
                    *rotation = -*rotation;
                }
            }
            GuideKind::Polygon {
                center,
                radius,
                rotation_degrees: rotation,
                ..
            } => {
                transform_point(center);
                *radius *= magnitude;
                *rotation += rotation_degrees;
                if mirror_x ^ mirror_y {
                    *rotation = -*rotation;
                }
            }
            GuideKind::Spiral {
                center,
                start_radius,
                end_radius,
                turns,
                rotation_degrees: rotation,
            } => {
                transform_point(center);
                *start_radius *= magnitude;
                *end_radius *= magnitude;
                *rotation += rotation_degrees;
                if mirror_x ^ mirror_y {
                    *turns = -*turns;
                }
            }
            GuideKind::Polyline { points, .. } | GuideKind::ImportedPath { points, .. } => {
                for point in points {
                    transform_point(point);
                }
            }
            GuideKind::Fan {
                pivot,
                radius,
                start_degrees,
                sweep_degrees,
                ..
            } => {
                transform_point(pivot);
                *radius *= magnitude;
                *start_degrees += rotation_degrees;
                if mirror_x ^ mirror_y {
                    *sweep_degrees = -*sweep_degrees;
                }
            }
            GuideKind::Concentric {
                center,
                start_radius,
                ring_spacing,
                ..
            } => {
                transform_point(center);
                *start_radius *= magnitude;
                *ring_spacing *= magnitude;
            }
            GuideKind::Chevron { left, pivot, right } => {
                for point in [left, pivot, right] {
                    transform_point(point);
                }
            }
            GuideKind::Text {
                origin, cell_size, ..
            } => {
                transform_point(origin);
                *cell_size *= magnitude;
            }
        }
        guide.spacing_steps *= magnitude;
        if mirror_x ^ mirror_y {
            guide.reverse_spots = !guide.reverse_spots;
        }
    }
    fn convert_guide_to_spline(guide: &mut FormationGuide) {
        let path = Self::guide_paths(guide)
            .into_iter()
            .next()
            .unwrap_or_default();
        if path.len() < 2 {
            return;
        }
        let length: f32 = path
            .windows(2)
            .map(|segment| segment[0].distance(segment[1]))
            .sum();
        guide.kind = GuideKind::Spline {
            start: path[0],
            control_1: Self::point_on_polyline(&path, length / 3.0),
            control_2: Self::point_on_polyline(&path, length * 2.0 / 3.0),
            end: *path.last().unwrap(),
        };
    }
    fn duplicate_selected_guide(&mut self) {
        let Some(mut guide) = self.selected_guide() else {
            return;
        };
        self.snapshot();
        guide.id = self.show.next_guide_id();
        Self::transform_guide(
            &mut guide,
            Point {
                x: field::yards_to_steps(self.guide_offset_x_yards),
                y: field::yards_to_steps(self.guide_offset_y_yards),
            },
            1.0,
            0.0,
            false,
            false,
        );
        self.selected_guide = Some(guide.id);
        self.show.sets[self.current].guides.push(guide);
        self.status = "Created offset guide copy".into();
    }
    fn apply_selected_guide_transform(&mut self, mirror_x: bool, mirror_y: bool) {
        let Some(mut guide) = self.selected_guide() else {
            return;
        };
        self.snapshot();
        Self::transform_guide(
            &mut guide,
            Point {
                x: field::yards_to_steps(self.guide_offset_x_yards),
                y: field::yards_to_steps(self.guide_offset_y_yards),
            },
            self.guide_scale,
            self.guide_rotation_degrees,
            mirror_x,
            mirror_y,
        );
        if let Some(saved) = self.show.sets[self.current]
            .guides
            .iter_mut()
            .find(|saved| saved.id == guide.id)
        {
            *saved = guide;
        }
        self.guide_offset_x_yards = 0.0;
        self.guide_offset_y_yards = 0.0;
        self.guide_rotation_degrees = 0.0;
        self.guide_scale = 1.0;
        self.status = "Transformed formation guide".into();
    }
    fn split_selected_guide(&mut self) {
        let Some(mut guide) = self.selected_guide() else {
            return;
        };
        let path = Self::guide_paths(&guide)
            .into_iter()
            .next()
            .unwrap_or_default();
        if path.len() < 4 {
            self.status = "This guide needs at least four sampled points to split".into();
            return;
        }
        let middle = path.len() / 2;
        let first = path[..=middle].to_vec();
        let second = path[middle..].to_vec();
        self.snapshot();
        let original_id = guide.id;
        guide.kind = GuideKind::Polyline {
            points: first,
            closed: false,
        };
        if let Some(saved) = self.show.sets[self.current]
            .guides
            .iter_mut()
            .find(|saved| saved.id == original_id)
        {
            *saved = guide.clone();
        }
        guide.id = self.show.next_guide_id();
        guide.kind = GuideKind::Polyline {
            points: second,
            closed: false,
        };
        self.selected_guide = Some(guide.id);
        self.show.sets[self.current].guides.push(guide);
        self.status = "Split guide into two editable paths".into();
    }
    fn join_selected_to_nearest(&mut self) {
        let Some(selected) = self.selected_guide() else {
            return;
        };
        let mut selected_path = Self::guide_paths(&selected)
            .into_iter()
            .next()
            .unwrap_or_default();
        if selected_path.len() < 2 {
            return;
        }
        let candidate = self.show.sets[self.current]
            .guides
            .iter()
            .filter(|guide| guide.id != selected.id)
            .filter_map(|guide| {
                let path = Self::guide_paths(guide).into_iter().next()?;
                if path.len() < 2 {
                    return None;
                }
                let endpoints = [
                    (
                        selected_path
                            .last()
                            .unwrap()
                            .distance(*path.first().unwrap()),
                        false,
                        false,
                    ),
                    (
                        selected_path
                            .last()
                            .unwrap()
                            .distance(*path.last().unwrap()),
                        false,
                        true,
                    ),
                    (
                        selected_path
                            .first()
                            .unwrap()
                            .distance(*path.first().unwrap()),
                        true,
                        false,
                    ),
                    (
                        selected_path
                            .first()
                            .unwrap()
                            .distance(*path.last().unwrap()),
                        true,
                        true,
                    ),
                ];
                let best = endpoints.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?;
                Some((guide.id, path, best))
            })
            .min_by(|a, b| a.2.0.total_cmp(&b.2.0));
        let Some((other_id, mut other_path, (_, reverse_selected, reverse_other))) = candidate
        else {
            self.status = "No other path is available to join".into();
            return;
        };
        if reverse_selected {
            selected_path.reverse();
        }
        if reverse_other {
            other_path.reverse();
        }
        selected_path.extend(other_path.into_iter().skip(1));
        self.snapshot();
        if let Some(saved) = self.show.sets[self.current]
            .guides
            .iter_mut()
            .find(|guide| guide.id == selected.id)
        {
            saved.kind = GuideKind::Polyline {
                points: selected_path,
                closed: false,
            };
        }
        self.show.sets[self.current]
            .guides
            .retain(|guide| guide.id != other_id);
        self.status = "Joined selected guide to the nearest path".into();
    }
    fn save_selected_prefab(&mut self) {
        let Some(guide) = self.selected_guide() else {
            return;
        };
        self.status = match serde_json::to_vec_pretty(&guide)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                fs::write("formation-prefab.json", bytes).map_err(|error| error.to_string())
            }) {
            Ok(()) => "Saved formation-prefab.json".into(),
            Err(error) => format!("Could not save prefab: {error}"),
        };
    }
    fn insert_saved_prefab(&mut self) {
        let result = fs::read("formation-prefab.json")
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<FormationGuide>(&bytes).map_err(|error| error.to_string())
            });
        match result {
            Ok(mut guide) => {
                let target = self.cursor_field.unwrap_or(Point {
                    x: FIELD_W * 0.5,
                    y: FIELD_H * 0.5,
                });
                let anchor = Self::guide_anchor(&guide);
                self.snapshot();
                guide.id = self.show.next_guide_id();
                Self::transform_guide(
                    &mut guide,
                    Point {
                        x: target.x - anchor.x,
                        y: target.y - anchor.y,
                    },
                    1.0,
                    0.0,
                    false,
                    false,
                );
                self.selected_guide = Some(guide.id);
                self.show.sets[self.current].guides.push(guide);
                self.status = "Inserted saved formation prefab".into();
            }
            Err(error) => self.status = format!("Could not load prefab: {error}"),
        }
    }
    fn import_outline_file(&mut self) {
        let result = fs::read_to_string("formation-outline.txt").map_err(|error| error.to_string());
        match result {
            Ok(text) => {
                let numbers: Vec<f32> = text
                    .split(|character: char| {
                        !(character.is_ascii_digit()
                            || matches!(character, '.' | '-' | '+' | 'e' | 'E'))
                    })
                    .filter(|part| !part.is_empty())
                    .filter_map(|part| part.parse().ok())
                    .collect();
                let points: Vec<Point> = numbers
                    .chunks_exact(2)
                    .map(|pair| Point {
                        x: field::yards_to_steps(pair[0]).clamp(0.0, FIELD_W),
                        y: field::yards_to_steps(pair[1]).clamp(0.0, FIELD_H),
                    })
                    .collect();
                if points.len() < 2 {
                    self.status = "formation-outline.txt needs at least two X,Y yard pairs".into();
                    return;
                }
                self.snapshot();
                let id = self.show.next_guide_id();
                self.show.sets[self.current].guides.push(FormationGuide {
                    id,
                    spacing_steps: 2.0,
                    spot_count: self.selected.len().max(8),
                    fill_guide: true,
                    reverse_spots: false,
                    kind: GuideKind::ImportedPath {
                        points,
                        closed: true,
                    },
                });
                self.selected_guide = Some(id);
                self.status = "Imported formation-outline.txt".into();
            }
            Err(error) => self.status = format!("Could not import outline: {error}"),
        }
    }
    fn guide_distance(guide: &FormationGuide, point: Point) -> f32 {
        Self::project_to_guide(guide, point).distance(point)
    }
    fn hit_guide(&self, point: Point, tolerance: f32) -> Option<u32> {
        self.show.sets[self.current]
            .guides
            .iter()
            .filter_map(|guide| {
                let distance = Self::guide_distance(guide, point);
                (distance <= tolerance).then_some((guide.id, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
    fn hit_symmetry_axis(&self, point: Point, tolerance: f32) -> Option<u32> {
        self.show.sets[self.current]
            .symmetry_axes
            .iter()
            .filter_map(|axis| {
                let distance =
                    Self::project_to_segment(axis.start, axis.end, point).distance(point);
                (distance <= tolerance).then_some((axis.id, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
    fn selected_guide(&self) -> Option<FormationGuide> {
        let id = self.selected_guide?;
        self.show.sets[self.current]
            .guides
            .iter()
            .find(|guide| guide.id == id)
            .cloned()
    }
    fn project_selected_to_guide(&mut self) {
        let Some(guide) = self.selected_guide() else {
            self.status = "Select a formation guide first".into();
            return;
        };
        let ids: Vec<u32> = self.selected.iter().copied().collect();
        if ids.is_empty() {
            self.status = "Select performers to snap".into();
            return;
        }
        let spots = Self::guide_spots(&guide);
        if ids.len() != spots.len() {
            self.status = format!(
                "Project needs {} selected performer(s); {} selected",
                spots.len(),
                ids.len()
            );
            return;
        }
        let sources: Vec<Point> = ids
            .iter()
            .filter_map(|id| self.editable_position(*id))
            .collect();
        let assignment = Self::minimum_cost_assignment(&sources, &spots);
        self.snapshot();
        for (source_index, id) in ids.into_iter().enumerate() {
            self.set_editable_position(id, spots[assignment[source_index]]);
        }
        self.status = format!(
            "Projected {} performer(s) uniformly to guide spots",
            self.selected.len()
        );
    }
    fn distribute_selected_on_guide(&mut self) {
        let Some(guide) = self.selected_guide() else {
            self.status = "Select a formation guide first".into();
            return;
        };
        let ids: Vec<u32> = self.selected.iter().copied().collect();
        if ids.is_empty() {
            self.status = "Select performers to distribute".into();
            return;
        }
        let positions = Self::guide_spots(&guide);
        let destination_count = positions.len();
        if ids.len() != positions.len() {
            self.status = format!(
                "Compass needs {} selected performer(s); {} selected",
                positions.len(),
                ids.len()
            );
            return;
        }
        let sources: Vec<Point> = ids
            .iter()
            .filter_map(|id| self.compass_source_position(*id))
            .collect();
        if sources.len() != ids.len() {
            self.status = "A selected performer has no preceding position".into();
            return;
        }
        let shortest = Self::minimum_cost_assignment(&sources, &positions);
        let even = Self::minimum_maximum_assignment(&sources, &positions);
        let ordered = Self::ordered_assignment(&sources, &positions);
        let collision_avoiding = Self::collision_avoiding_assignment(
            &sources,
            &positions,
            &[shortest.clone(), even.clone(), ordered.clone()],
        );
        let mut candidates = Vec::new();
        for (name, assignment) in [
            ("Shortest total", shortest.clone()),
            ("Even effort", even),
            ("Keep order", ordered),
            ("Avoid collisions", collision_avoiding.clone()),
        ] {
            Self::push_compass_candidate(
                &mut candidates,
                name,
                CompassRoute::Straight,
                &ids,
                assignment,
                &sources,
                &positions,
            );
        }
        let symmetry_seeds = [
            shortest.clone(),
            Self::minimum_maximum_assignment(&sources, &positions),
            Self::ordered_assignment(&sources, &positions),
            collision_avoiding.clone(),
        ];
        for axis in self.show.sets[self.current].symmetry_axes.clone() {
            if let Some((assignment, score)) =
                Self::symmetry_compass_assignment(&axis, &sources, &positions, &symmetry_seeds)
            {
                Self::push_compass_candidate(
                    &mut candidates,
                    format!("Symmetry: {}", axis.name),
                    CompassRoute::Straight,
                    &ids,
                    assignment,
                    &sources,
                    &positions,
                );
                if let Some(candidate) = candidates.last_mut() {
                    candidate.symmetry_conflicts = Some(score.symmetry_conflicts);
                    candidate.symmetry_error = score.symmetry_error;
                    candidate.collision_risks = score.collision_conflicts;
                    candidate.symmetry_axis = Some(axis);
                }
            }
        }
        let interval = self.compass_interval();
        if interval.is_some() {
            for (name, bend, assignment) in [
                ("Curve left", 0.22, shortest.clone()),
                ("Curve right", -0.22, shortest.clone()),
                ("Avoid + curve left", 0.22, collision_avoiding.clone()),
                ("Avoid + curve right", -0.22, collision_avoiding.clone()),
            ] {
                Self::push_compass_candidate(
                    &mut candidates,
                    name,
                    CompassRoute::Curve(bend),
                    &ids,
                    assignment,
                    &sources,
                    &positions,
                );
            }
        }
        self.compass_preview = Some(CompassPreview {
            candidates,
            selected_candidate: 0,
            spots: positions,
            original_destination_positions: ids
                .iter()
                .filter_map(|id| self.editable_position(*id).map(|point| (*id, point)))
                .collect(),
            source_positions: ids.into_iter().zip(sources).collect(),
            destination_set: self.current,
            destination_subset: self.active_subset,
            interval,
            scope: CompassScope::IncomingOnly,
        });
        self.status = format!(
            "Proposed compass match for {} performer(s); approve or adjust it",
            destination_count
        );
    }
    fn push_compass_candidate(
        candidates: &mut Vec<CompassCandidate>,
        name: impl Into<String>,
        route: CompassRoute,
        ids: &[u32],
        assignment: Vec<usize>,
        sources: &[Point],
        targets: &[Point],
    ) {
        let entries: Vec<CompassEntry> = ids
            .iter()
            .enumerate()
            .map(|(source_index, performer_id)| CompassEntry {
                performer_id: *performer_id,
                target_index: assignment[source_index],
            })
            .collect();
        let distances: Vec<f32> = sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                Self::compass_route_distance(*source, targets[assignment[index]], route)
            })
            .collect();
        candidates.push(CompassCandidate {
            name: name.into(),
            entries,
            route,
            total_distance: distances.iter().sum(),
            maximum_distance: distances.into_iter().fold(0.0, f32::max),
            collision_risks: Self::assignment_collision_metrics(
                sources,
                targets,
                &assignment,
                route,
            )
            .0,
            symmetry_conflicts: None,
            symmetry_error: 0.0,
            symmetry_axis: None,
        });
    }
    fn compass_route_distance(source: Point, target: Point, route: CompassRoute) -> f32 {
        match route {
            CompassRoute::Straight => source.distance(target),
            CompassRoute::Curve(bend) => {
                let mut previous = source;
                let mut distance = 0.0;
                for step in 1..=16 {
                    let point =
                        Self::curved_compass_point(source, target, step as f32 / 16.0, bend);
                    distance += previous.distance(point);
                    previous = point;
                }
                distance
            }
        }
    }
    fn refresh_compass_metrics(preview: &mut CompassPreview) {
        let Some(candidate) = preview.candidates.get(preview.selected_candidate) else {
            return;
        };
        let distances: Vec<f32> = candidate
            .entries
            .iter()
            .filter_map(|entry| {
                let source = preview
                    .source_positions
                    .iter()
                    .find_map(|(id, point)| (*id == entry.performer_id).then_some(*point))?;
                let target = *preview.spots.get(entry.target_index)?;
                Some(Self::compass_route_distance(
                    source,
                    target,
                    candidate.route,
                ))
            })
            .collect();
        let assignment: Vec<usize> = candidate
            .entries
            .iter()
            .map(|entry| entry.target_index)
            .collect();
        let sources: Vec<Point> = candidate
            .entries
            .iter()
            .filter_map(|entry| {
                preview
                    .source_positions
                    .iter()
                    .find_map(|(id, point)| (*id == entry.performer_id).then_some(*point))
            })
            .collect();
        let collision_risks = if sources.len() == assignment.len() {
            Self::assignment_collision_metrics(
                &sources,
                &preview.spots,
                &assignment,
                candidate.route,
            )
            .0
        } else {
            0
        };
        let symmetry = candidate.symmetry_axis.as_ref().and_then(|axis| {
            let constraint = Self::compass_symmetry_constraint(axis, &sources)?;
            let score = Self::compass_symmetry_score(
                axis,
                &constraint,
                &preview.spots,
                &assignment,
                &sources,
            );
            Some((score.symmetry_conflicts, score.symmetry_error))
        });
        if let Some(candidate) = preview.candidates.get_mut(preview.selected_candidate) {
            candidate.total_distance = distances.iter().sum();
            candidate.maximum_distance = distances.into_iter().fold(0.0, f32::max);
            candidate.collision_risks = collision_risks;
            if let Some((conflicts, error)) = symmetry {
                candidate.symmetry_conflicts = Some(conflicts);
                candidate.symmetry_error = error;
            }
        }
    }
    fn compass_interval(&self) -> Option<CompassInterval> {
        if let Some(active) = self.active_subset() {
            let source_offset = self.show.sets[self.current]
                .subsets
                .iter()
                .filter(|subset| subset.offset_count < active.offset_count)
                .map(|subset| subset.offset_count)
                .max_by(f32::total_cmp)
                .unwrap_or(0.0);
            return Some(CompassInterval {
                set_index: self.current,
                source_offset,
                destination_offset: active.offset_count,
            });
        }
        if self.current == 0 {
            return None;
        }
        let previous_index = self.current - 1;
        let previous = &self.show.sets[previous_index];
        let source_offset = previous
            .subsets
            .iter()
            .map(|subset| subset.offset_count)
            .max_by(f32::total_cmp)
            .unwrap_or(0.0);
        Some(CompassInterval {
            set_index: previous_index,
            source_offset,
            destination_offset: previous.counts_to_next as f32,
        })
    }
    fn compass_source_position(&self, performer_id: u32) -> Option<Point> {
        if let Some(active) = self.active_subset() {
            return self.show.sets[self.current]
                .subsets
                .iter()
                .filter(|subset| subset.offset_count < active.offset_count)
                .filter_map(|subset| {
                    subset
                        .positions
                        .get(&performer_id)
                        .copied()
                        .map(|point| (subset.offset_count, point))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, point)| point)
                .or_else(|| {
                    self.show.sets[self.current]
                        .positions
                        .get(&performer_id)
                        .copied()
                });
        }
        if self.current == 0 {
            return self.show.sets[0].positions.get(&performer_id).copied();
        }
        let previous = &self.show.sets[self.current - 1];
        previous
            .subsets
            .iter()
            .filter_map(|subset| {
                subset
                    .positions
                    .get(&performer_id)
                    .copied()
                    .map(|point| (subset.offset_count, point))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, point)| point)
            .or_else(|| previous.positions.get(&performer_id).copied())
    }
    fn minimum_cost_assignment(sources: &[Point], targets: &[Point]) -> Vec<usize> {
        let costs: Vec<Vec<f32>> = sources
            .iter()
            .map(|source| {
                targets
                    .iter()
                    .map(|target| source.distance(*target))
                    .collect()
            })
            .collect();
        Self::minimum_cost_assignment_for_matrix(&costs)
    }
    fn minimum_maximum_assignment(sources: &[Point], targets: &[Point]) -> Vec<usize> {
        let distances: Vec<Vec<f32>> = sources
            .iter()
            .map(|source| {
                targets
                    .iter()
                    .map(|target| source.distance(*target))
                    .collect()
            })
            .collect();
        let mut thresholds: Vec<f32> = distances.iter().flatten().copied().collect();
        thresholds.sort_by(f32::total_cmp);
        thresholds.dedup_by(|a, b| (*a - *b).abs() < f32::EPSILON);
        let mut low = 0;
        let mut high = thresholds.len().saturating_sub(1);
        while low < high {
            let middle = (low + high) / 2;
            if Self::has_perfect_matching(&distances, thresholds[middle]) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let threshold = thresholds.get(low).copied().unwrap_or(0.0);
        let penalty = distances.iter().flatten().copied().sum::<f32>() + 1_000_000.0;
        let costs: Vec<Vec<f32>> = distances
            .iter()
            .map(|row| {
                row.iter()
                    .map(|distance| {
                        if *distance <= threshold + f32::EPSILON {
                            *distance
                        } else {
                            penalty + *distance
                        }
                    })
                    .collect()
            })
            .collect();
        Self::minimum_cost_assignment_for_matrix(&costs)
    }
    fn has_perfect_matching(distances: &[Vec<f32>], threshold: f32) -> bool {
        fn assign(
            source: usize,
            distances: &[Vec<f32>],
            threshold: f32,
            seen: &mut [bool],
            target_owner: &mut [Option<usize>],
        ) -> bool {
            for target in 0..distances.len() {
                if seen[target] || distances[source][target] > threshold + f32::EPSILON {
                    continue;
                }
                seen[target] = true;
                if target_owner[target]
                    .is_none_or(|owner| assign(owner, distances, threshold, seen, target_owner))
                {
                    target_owner[target] = Some(source);
                    return true;
                }
            }
            false
        }
        let mut target_owner = vec![None; distances.len()];
        for source in 0..distances.len() {
            let mut seen = vec![false; distances.len()];
            if !assign(source, distances, threshold, &mut seen, &mut target_owner) {
                return false;
            }
        }
        true
    }
    fn ordered_assignment(sources: &[Point], targets: &[Point]) -> Vec<usize> {
        fn order(points: &[Point]) -> Vec<usize> {
            if points.len() < 2 {
                return (0..points.len()).collect();
            }
            let (a, b) = (0..points.len())
                .flat_map(|a| (a + 1..points.len()).map(move |b| (a, b)))
                .max_by(|(a1, b1), (a2, b2)| {
                    points[*a1]
                        .distance(points[*b1])
                        .total_cmp(&points[*a2].distance(points[*b2]))
                })
                .unwrap_or((0, 1));
            let axis = Point {
                x: points[b].x - points[a].x,
                y: points[b].y - points[a].y,
            };
            let mut indices: Vec<usize> = (0..points.len()).collect();
            indices.sort_by(|left, right| {
                let left_projection = points[*left].x * axis.x + points[*left].y * axis.y;
                let right_projection = points[*right].x * axis.x + points[*right].y * axis.y;
                left_projection.total_cmp(&right_projection)
            });
            indices
        }
        let source_order = order(sources);
        let target_order = order(targets);
        let build = |reverse: bool| {
            let mut assignment = vec![0; sources.len()];
            for (rank, source) in source_order.iter().enumerate() {
                let target_rank = if reverse {
                    target_order.len() - 1 - rank
                } else {
                    rank
                };
                assignment[*source] = target_order[target_rank];
            }
            assignment
        };
        let direct = build(false);
        let reverse = build(true);
        let cost = |assignment: &[usize]| {
            sources
                .iter()
                .enumerate()
                .map(|(index, source)| source.distance(targets[assignment[index]]))
                .sum::<f32>()
        };
        if cost(&direct) <= cost(&reverse) {
            direct
        } else {
            reverse
        }
    }
    fn reflect_across_axis(axis: &SymmetryAxis, point: Point) -> Option<Point> {
        let (along, normal) = Self::symmetry_coordinates(axis, point)?;
        Self::symmetry_point(axis, along, -normal)
    }
    fn compass_symmetry_constraint(
        axis: &SymmetryAxis,
        sources: &[Point],
    ) -> Option<CompassSymmetryConstraint> {
        let coordinates: Vec<(usize, f32)> = sources
            .iter()
            .enumerate()
            .filter_map(|(index, point)| {
                Self::symmetry_coordinates(axis, *point).map(|(_, normal)| (index, normal))
            })
            .collect();
        if coordinates.len() != sources.len() {
            return None;
        }
        let mut pairs = Vec::new();
        let mut centers = Vec::new();
        match axis.mode {
            SymmetryMode::Perfect => {
                let mut ordered = coordinates;
                ordered.sort_by(|a, b| a.1.total_cmp(&b.1));
                let pair_count = ordered.len() / 2;
                let left = &ordered[..pair_count];
                let right = &ordered[ordered.len() - pair_count..];
                let reflected: Vec<Point> = left
                    .iter()
                    .filter_map(|(index, _)| Self::reflect_across_axis(axis, sources[*index]))
                    .collect();
                let right_points: Vec<Point> =
                    right.iter().map(|(index, _)| sources[*index]).collect();
                if reflected.len() != pair_count {
                    return None;
                }
                let assignment = Self::minimum_cost_assignment(&reflected, &right_points);
                for (left_index, right_index) in assignment.into_iter().enumerate() {
                    pairs.push((left[left_index].0, right[right_index].0));
                }
                if ordered.len() % 2 == 1 {
                    centers.push(ordered[pair_count].0);
                }
            }
            SymmetryMode::Try => {
                let left: Vec<_> = coordinates
                    .iter()
                    .filter(|(_, normal)| *normal < -SYMMETRY_AXIS_EPSILON)
                    .copied()
                    .collect();
                let right: Vec<_> = coordinates
                    .iter()
                    .filter(|(_, normal)| *normal > SYMMETRY_AXIS_EPSILON)
                    .copied()
                    .collect();
                centers.extend(
                    coordinates
                        .iter()
                        .filter(|(_, normal)| normal.abs() <= SYMMETRY_AXIS_EPSILON)
                        .map(|(index, _)| *index),
                );
                let mut options = Vec::new();
                for (left_index, _) in &left {
                    let reflected = Self::reflect_across_axis(axis, sources[*left_index])?;
                    for (right_index, _) in &right {
                        options.push((
                            reflected.distance(sources[*right_index]),
                            *left_index,
                            *right_index,
                        ));
                    }
                }
                options.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut used_left = BTreeSet::new();
                let mut used_right = BTreeSet::new();
                for (distance, left_index, right_index) in options {
                    if distance <= axis.threshold_steps
                        && used_left.insert(left_index)
                        && used_right.insert(right_index)
                    {
                        pairs.push((left_index, right_index));
                    }
                }
            }
        }
        (!pairs.is_empty() || !centers.is_empty()).then_some(CompassSymmetryConstraint {
            pairs,
            centers,
            tolerance: match axis.mode {
                SymmetryMode::Perfect => SYMMETRY_AXIS_EPSILON,
                SymmetryMode::Try => axis.threshold_steps.max(0.0),
            },
        })
    }
    fn compass_symmetry_score(
        axis: &SymmetryAxis,
        constraint: &CompassSymmetryConstraint,
        targets: &[Point],
        assignment: &[usize],
        sources: &[Point],
    ) -> CompassSymmetryScore {
        let mut symmetry_conflicts = 0;
        let mut error = 0.0;
        for (first, second) in &constraint.pairs {
            let mirrored = Self::reflect_across_axis(axis, targets[assignment[*first]])
                .unwrap_or(targets[assignment[*first]]);
            let pair_error = mirrored.distance(targets[assignment[*second]]);
            symmetry_conflicts += (pair_error > constraint.tolerance) as usize;
            error += pair_error;
        }
        for center in &constraint.centers {
            let center_error = Self::symmetry_coordinates(axis, targets[assignment[*center]])
                .map_or(f32::INFINITY, |(_, normal)| normal.abs());
            symmetry_conflicts += (center_error > constraint.tolerance) as usize;
            error += center_error;
        }
        let (collision_conflicts, collision_penalty) = Self::assignment_collision_metrics(
            sources,
            targets,
            assignment,
            CompassRoute::Straight,
        );
        CompassSymmetryScore {
            symmetry_conflicts,
            collision_conflicts,
            symmetry_error: error,
            collision_penalty,
            travel: Self::assignment_travel(sources, targets, assignment),
        }
    }
    fn symmetry_score_better(
        candidate: CompassSymmetryScore,
        current: CompassSymmetryScore,
    ) -> bool {
        candidate.symmetry_conflicts < current.symmetry_conflicts
            || candidate.symmetry_conflicts == current.symmetry_conflicts
                && (candidate.collision_conflicts < current.collision_conflicts
                    || candidate.collision_conflicts == current.collision_conflicts
                        && (candidate.symmetry_error < current.symmetry_error - 0.0001
                            || (candidate.symmetry_error - current.symmetry_error).abs() <= 0.0001
                                && (candidate.collision_penalty
                                    < current.collision_penalty - 0.0001
                                    || (candidate.collision_penalty - current.collision_penalty)
                                        .abs()
                                        <= 0.0001
                                        && candidate.travel < current.travel)))
    }
    fn symmetry_compass_assignment(
        axis: &SymmetryAxis,
        sources: &[Point],
        targets: &[Point],
        seeds: &[Vec<usize>],
    ) -> Option<(Vec<usize>, CompassSymmetryScore)> {
        #[derive(Clone, Copy)]
        enum Swap {
            Single(usize, usize),
            Pair((usize, usize), (usize, usize)),
        }
        let constraint = Self::compass_symmetry_constraint(axis, sources)?;
        let shortest = seeds.first()?;
        let shortest_travel = Self::assignment_travel(sources, targets, shortest);
        let travel_budget = match axis.mode {
            SymmetryMode::Perfect => f32::INFINITY,
            SymmetryMode::Try => {
                (shortest_travel * COMPASS_TRAVEL_BUDGET).max(shortest_travel + 4.0)
            }
        };
        let mut best = shortest.clone();
        let mut best_score =
            Self::compass_symmetry_score(axis, &constraint, targets, &best, sources);
        for seed in seeds {
            let mut assignment = seed.clone();
            let mut score =
                Self::compass_symmetry_score(axis, &constraint, targets, &assignment, sources);
            for _ in 0..COMPASS_COLLISION_PASSES {
                let mut best_swap = None;
                let mut swap_score = score;
                for first in 0..assignment.len() {
                    for second in first + 1..assignment.len() {
                        assignment.swap(first, second);
                        let candidate_score = Self::compass_symmetry_score(
                            axis,
                            &constraint,
                            targets,
                            &assignment,
                            sources,
                        );
                        assignment.swap(first, second);
                        if candidate_score.travel <= travel_budget
                            && Self::symmetry_score_better(candidate_score, swap_score)
                        {
                            best_swap = Some(Swap::Single(first, second));
                            swap_score = candidate_score;
                        }
                    }
                }
                for (pair_index, first_pair) in constraint.pairs.iter().enumerate() {
                    assignment.swap(first_pair.0, first_pair.1);
                    let candidate_score = Self::compass_symmetry_score(
                        axis,
                        &constraint,
                        targets,
                        &assignment,
                        sources,
                    );
                    assignment.swap(first_pair.0, first_pair.1);
                    if candidate_score.travel <= travel_budget
                        && Self::symmetry_score_better(candidate_score, swap_score)
                    {
                        best_swap = Some(Swap::Single(first_pair.0, first_pair.1));
                        swap_score = candidate_score;
                    }
                    for second_pair in constraint.pairs.iter().skip(pair_index + 1) {
                        assignment.swap(first_pair.0, second_pair.0);
                        assignment.swap(first_pair.1, second_pair.1);
                        let candidate_score = Self::compass_symmetry_score(
                            axis,
                            &constraint,
                            targets,
                            &assignment,
                            sources,
                        );
                        assignment.swap(first_pair.1, second_pair.1);
                        assignment.swap(first_pair.0, second_pair.0);
                        if candidate_score.travel <= travel_budget
                            && Self::symmetry_score_better(candidate_score, swap_score)
                        {
                            best_swap = Some(Swap::Pair(*first_pair, *second_pair));
                            swap_score = candidate_score;
                        }
                    }
                }
                let Some(best_swap) = best_swap else {
                    break;
                };
                match best_swap {
                    Swap::Single(first, second) => assignment.swap(first, second),
                    Swap::Pair(first, second) => {
                        assignment.swap(first.0, second.0);
                        assignment.swap(first.1, second.1);
                    }
                }
                score = swap_score;
            }
            if Self::symmetry_score_better(score, best_score) {
                best = assignment;
                best_score = score;
            }
        }
        Some((best, best_score))
    }
    fn collision_avoiding_assignment(
        sources: &[Point],
        targets: &[Point],
        seeds: &[Vec<usize>],
    ) -> Vec<usize> {
        let shortest_total = seeds
            .first()
            .map(|assignment| Self::assignment_travel(sources, targets, assignment))
            .unwrap_or(0.0);
        let travel_budget = (shortest_total * COMPASS_TRAVEL_BUDGET).max(shortest_total + 4.0);
        let mut best = seeds.first().cloned().unwrap_or_default();
        let mut best_score = Self::collision_assignment_score(sources, targets, &best);
        for seed in seeds {
            let mut assignment = seed.clone();
            let mut score = Self::collision_assignment_score(sources, targets, &assignment);
            for _ in 0..COMPASS_COLLISION_PASSES {
                let mut best_swap = None;
                let mut swap_score = score;
                for first in 0..assignment.len() {
                    for second in first + 1..assignment.len() {
                        let first_target = assignment[first];
                        let second_target = assignment[second];
                        let new_travel = score.2
                            - sources[first].distance(targets[first_target])
                            - sources[second].distance(targets[second_target])
                            + sources[first].distance(targets[second_target])
                            + sources[second].distance(targets[first_target]);
                        if new_travel > travel_budget {
                            continue;
                        }
                        let mut risk_delta = 0_i32;
                        let mut penalty_delta = 0.0;
                        let old_pair = Self::collision_pair_metrics(
                            sources[first],
                            targets[first_target],
                            sources[second],
                            targets[second_target],
                            CompassRoute::Straight,
                        );
                        let new_pair = Self::collision_pair_metrics(
                            sources[first],
                            targets[second_target],
                            sources[second],
                            targets[first_target],
                            CompassRoute::Straight,
                        );
                        risk_delta += new_pair.0 as i32 - old_pair.0 as i32;
                        penalty_delta += new_pair.1 - old_pair.1;
                        for other in 0..assignment.len() {
                            if other == first || other == second {
                                continue;
                            }
                            for (performer, old_target, new_target) in [
                                (first, first_target, second_target),
                                (second, second_target, first_target),
                            ] {
                                let old = Self::collision_pair_metrics(
                                    sources[performer],
                                    targets[old_target],
                                    sources[other],
                                    targets[assignment[other]],
                                    CompassRoute::Straight,
                                );
                                let new = Self::collision_pair_metrics(
                                    sources[performer],
                                    targets[new_target],
                                    sources[other],
                                    targets[assignment[other]],
                                    CompassRoute::Straight,
                                );
                                risk_delta += new.0 as i32 - old.0 as i32;
                                penalty_delta += new.1 - old.1;
                            }
                        }
                        let candidate_score = (
                            (score.0 as i32 + risk_delta).max(0) as usize,
                            (score.1 + penalty_delta).max(0.0),
                            new_travel,
                        );
                        if Self::collision_score_better(candidate_score, swap_score) {
                            best_swap = Some((first, second));
                            swap_score = candidate_score;
                        }
                    }
                }
                let Some((first, second)) = best_swap else {
                    break;
                };
                assignment.swap(first, second);
                score = swap_score;
            }
            if Self::collision_score_better(score, best_score) {
                best = assignment;
                best_score = score;
            }
        }
        best
    }
    fn collision_score_better(candidate: (usize, f32, f32), current: (usize, f32, f32)) -> bool {
        candidate.0 < current.0
            || candidate.0 == current.0
                && (candidate.1 < current.1 - 0.0001
                    || (candidate.1 - current.1).abs() <= 0.0001 && candidate.2 < current.2)
    }
    fn collision_assignment_score(
        sources: &[Point],
        targets: &[Point],
        assignment: &[usize],
    ) -> (usize, f32, f32) {
        let (risks, penalty) = Self::assignment_collision_metrics(
            sources,
            targets,
            assignment,
            CompassRoute::Straight,
        );
        (
            risks,
            penalty,
            Self::assignment_travel(sources, targets, assignment),
        )
    }
    fn assignment_travel(sources: &[Point], targets: &[Point], assignment: &[usize]) -> f32 {
        sources
            .iter()
            .enumerate()
            .map(|(index, source)| source.distance(targets[assignment[index]]))
            .sum()
    }
    fn assignment_collision_metrics(
        sources: &[Point],
        targets: &[Point],
        assignment: &[usize],
        route: CompassRoute,
    ) -> (usize, f32) {
        let mut risks = 0;
        let mut penalty = 0.0;
        for first in 0..sources.len() {
            for second in first + 1..sources.len() {
                let pair = Self::collision_pair_metrics(
                    sources[first],
                    targets[assignment[first]],
                    sources[second],
                    targets[assignment[second]],
                    route,
                );
                risks += pair.0;
                penalty += pair.1;
            }
        }
        (risks, penalty)
    }
    fn collision_pair_metrics(
        source_a: Point,
        target_a: Point,
        source_b: Point,
        target_b: Point,
        route: CompassRoute,
    ) -> (usize, f32) {
        let minimum = match route {
            CompassRoute::Straight => {
                let relative_start = Point {
                    x: source_a.x - source_b.x,
                    y: source_a.y - source_b.y,
                };
                let relative_velocity = Point {
                    x: (target_a.x - source_a.x) - (target_b.x - source_b.x),
                    y: (target_a.y - source_a.y) - (target_b.y - source_b.y),
                };
                let denominator = relative_velocity.x * relative_velocity.x
                    + relative_velocity.y * relative_velocity.y;
                let time = if denominator <= f32::EPSILON {
                    0.0
                } else {
                    (-(relative_start.x * relative_velocity.x
                        + relative_start.y * relative_velocity.y)
                        / denominator)
                        .clamp(0.0, 1.0)
                };
                Point {
                    x: relative_start.x + relative_velocity.x * time,
                    y: relative_start.y + relative_velocity.y * time,
                }
                .distance(Point::default())
            }
            CompassRoute::Curve(bend) => (0..=24)
                .map(|step| {
                    let time = step as f32 / 24.0;
                    Self::curved_compass_point(source_a, target_a, time, bend)
                        .distance(Self::curved_compass_point(source_b, target_b, time, bend))
                })
                .fold(f32::INFINITY, f32::min),
        };
        let shortfall = (COMPASS_CLEARANCE_STEPS - minimum).max(0.0);
        ((shortfall > 0.0) as usize, shortfall * shortfall)
    }
    fn minimum_cost_assignment_for_matrix(costs: &[Vec<f32>]) -> Vec<usize> {
        let count = costs.len();
        let mut u = vec![0.0_f32; count + 1];
        let mut v = vec![0.0_f32; count + 1];
        let mut p = vec![0_usize; count + 1];
        let mut way = vec![0_usize; count + 1];
        for i in 1..=count {
            p[0] = i;
            let mut j0 = 0;
            let mut minv = vec![f32::INFINITY; count + 1];
            let mut used = vec![false; count + 1];
            loop {
                used[j0] = true;
                let i0 = p[j0];
                let mut delta = f32::INFINITY;
                let mut j1 = 0;
                for j in 1..=count {
                    if !used[j] {
                        let cost = costs[i0 - 1][j - 1] - u[i0] - v[j];
                        if cost < minv[j] {
                            minv[j] = cost;
                            way[j] = j0;
                        }
                        if minv[j] < delta {
                            delta = minv[j];
                            j1 = j;
                        }
                    }
                }
                for j in 0..=count {
                    if used[j] {
                        u[p[j]] += delta;
                        v[j] -= delta;
                    } else {
                        minv[j] -= delta;
                    }
                }
                j0 = j1;
                if p[j0] == 0 {
                    break;
                }
            }
            loop {
                let j1 = way[j0];
                p[j0] = p[j1];
                j0 = j1;
                if j0 == 0 {
                    break;
                }
            }
        }
        let mut assignment = vec![0; count];
        for j in 1..=count {
            assignment[p[j] - 1] = j - 1;
        }
        assignment
    }
    fn approve_compass_preview(&mut self) -> bool {
        let Some(preview) = self.compass_preview.take() else {
            return false;
        };
        let Some(candidate) = preview.candidates.get(preview.selected_candidate).cloned() else {
            return false;
        };
        let original_show = self.show.clone();
        self.snapshot();
        if let CompassRoute::Curve(bend) = candidate.route
            && let Some(interval) = preview.interval
        {
            self.insert_compass_curve_subsets(&preview, &candidate, interval, bend);
        }
        for entry in &candidate.entries {
            let Some(target) = preview.spots.get(entry.target_index).copied() else {
                continue;
            };
            if let Some(subset_id) = preview.destination_subset {
                if let Some(subset) = self.show.sets[preview.destination_set]
                    .subsets
                    .iter_mut()
                    .find(|subset| subset.id == subset_id)
                {
                    subset.positions.insert(entry.performer_id, target);
                }
            } else if let Some(saved) = self.show.sets[preview.destination_set]
                .positions
                .get_mut(&entry.performer_id)
            {
                *saved = target;
            }
        }
        if preview.scope == CompassScope::IncomingOnly {
            self.propagate_compass_assignment(&preview, &candidate, &original_show);
        }
        self.status = format!(
            "Approved {} for {} performer(s)",
            candidate.name,
            candidate.entries.len()
        );
        true
    }
    fn propagate_compass_assignment(
        &mut self,
        preview: &CompassPreview,
        candidate: &CompassCandidate,
        original_show: &Show,
    ) {
        let original_slots: Vec<(u32, Point)> = candidate
            .entries
            .iter()
            .filter_map(|entry| {
                preview
                    .original_destination_positions
                    .iter()
                    .find(|(id, _)| *id == entry.performer_id)
                    .copied()
            })
            .collect();
        let new_positions: Vec<Point> = candidate
            .entries
            .iter()
            .filter_map(|entry| preview.spots.get(entry.target_index).copied())
            .collect();
        if original_slots.len() != candidate.entries.len()
            || new_positions.len() != candidate.entries.len()
        {
            return;
        }
        let old_positions: Vec<Point> = original_slots.iter().map(|(_, point)| *point).collect();
        let inherited_slots = Self::minimum_cost_assignment(&new_positions, &old_positions);
        let inheritance: Vec<(u32, u32)> = candidate
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.performer_id, original_slots[inherited_slots[index]].0))
            .collect();
        let destination_offset = preview.destination_subset.and_then(|id| {
            original_show.sets[preview.destination_set]
                .subsets
                .iter()
                .find(|subset| subset.id == id)
                .map(|subset| subset.offset_count)
        });

        if let Some(offset) = destination_offset {
            self.remap_future_subsets(
                preview.destination_set,
                Some(offset),
                &inheritance,
                original_show,
            );
        } else {
            self.remap_future_subsets(preview.destination_set, None, &inheritance, original_show);
        }
        for set_index in preview.destination_set + 1..self.show.sets.len() {
            let positions: Vec<(u32, Point)> = inheritance
                .iter()
                .filter_map(|(performer, inherited)| {
                    original_show.sets[set_index]
                        .positions
                        .get(inherited)
                        .copied()
                        .map(|point| (*performer, point))
                })
                .collect();
            for (performer, point) in positions {
                self.show.sets[set_index].positions.insert(performer, point);
            }
            self.remap_future_subsets(set_index, None, &inheritance, original_show);
        }
    }
    fn remap_future_subsets(
        &mut self,
        set_index: usize,
        after_offset: Option<f32>,
        inheritance: &[(u32, u32)],
        original_show: &Show,
    ) {
        let set_start = original_show.set_start(set_index);
        let subset_ids: Vec<(u32, f32)> = original_show.sets[set_index]
            .subsets
            .iter()
            .filter(|subset| after_offset.is_none_or(|offset| subset.offset_count > offset))
            .map(|subset| (subset.id, subset.offset_count))
            .collect();
        for (subset_id, subset_offset) in subset_ids {
            let positions: Vec<(u32, Point)> = inheritance
                .iter()
                .filter_map(|(performer, inherited)| {
                    original_show
                        .position_at(*inherited, set_start + subset_offset)
                        .map(|point| (*performer, point))
                })
                .collect();
            if let Some(subset) = self.show.sets[set_index]
                .subsets
                .iter_mut()
                .find(|subset| subset.id == subset_id)
            {
                subset.positions.extend(positions);
            }
        }
    }
    fn insert_compass_curve_subsets(
        &mut self,
        preview: &CompassPreview,
        candidate: &CompassCandidate,
        interval: CompassInterval,
        bend: f32,
    ) {
        let span = interval.destination_offset - interval.source_offset;
        if span <= 0.01 {
            return;
        }
        for (fraction, name) in [(1.0 / 3.0, "Curve 1"), (2.0 / 3.0, "Curve 2")] {
            let offset = interval.source_offset + span * fraction;
            let positions = candidate
                .entries
                .iter()
                .filter_map(|entry| {
                    let source = preview
                        .source_positions
                        .iter()
                        .find_map(|(id, point)| (*id == entry.performer_id).then_some(*point))?;
                    let target = *preview.spots.get(entry.target_index)?;
                    Some((
                        entry.performer_id,
                        Self::curved_compass_point(source, target, fraction, bend),
                    ))
                })
                .collect();
            let id = self.show.next_subset_id;
            self.show.next_subset_id += 1;
            self.show.sets[interval.set_index].subsets.push(Subset {
                id,
                name: name.into(),
                offset_count: offset,
                positions,
            });
        }
        self.show.sets[interval.set_index]
            .subsets
            .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
    }
    fn curved_compass_point(source: Point, target: Point, t: f32, bend: f32) -> Point {
        let dx = target.x - source.x;
        let dy = target.y - source.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= f32::EPSILON {
            return source;
        }
        let control = Point {
            x: (source.x + target.x) * 0.5 - dy * bend,
            y: (source.y + target.y) * 0.5 + dx * bend,
        };
        let inverse = 1.0 - t;
        let point = Point {
            x: inverse * inverse * source.x + 2.0 * inverse * t * control.x + t * t * target.x,
            y: inverse * inverse * source.y + 2.0 * inverse * t * control.y + t * t * target.y,
        };
        Point {
            x: point.x.clamp(0.0, FIELD_W),
            y: point.y.clamp(0.0, FIELD_H),
        }
    }
    fn selected_coordinate(&self) -> String {
        let Some(id) = self.selected.iter().next() else {
            return "No selection".into();
        };
        let Some(point) = self.editable_position(*id) else {
            return "No position".into();
        };
        format!(
            "X {:.2} yd / Y {:.2} yd",
            field::steps_to_yards(point.x),
            field::steps_to_yards(point.y)
        )
    }
    fn selected_drill_terms(&self) -> String {
        let Some(id) = self.selected.iter().next() else {
            return "No selection".into();
        };
        let Some(p) = self.editable_position(*id) else {
            return "No position".into();
        };
        let x_reference = field::nearest_yard_line(p.x);
        let x_offset = p.x - field::yards_to_steps(x_reference);
        let side_one = x_reference <= 50.0;
        let yard_line = if side_one {
            x_reference
        } else {
            100.0 - x_reference
        };
        let horizontal = if x_offset.abs() < 0.05 {
            format!("on {yard_line:.0} yd")
        } else {
            let inside = if side_one {
                x_offset > 0.0
            } else {
                x_offset < 0.0
            };
            format!(
                "{:.1} {} {yard_line:.0} yd",
                x_offset.abs(),
                if inside { "inside" } else { "outside" }
            )
        };
        let y_reference = field::nearest_vertical_reference(p.y);
        let y_offset = p.y - field::yards_to_steps(y_reference.yards());
        let vertical = if y_offset.abs() < 0.05 {
            format!("on {}", y_reference.label().to_ascii_lowercase())
        } else {
            format!(
                "{:.1} steps {} {}",
                y_offset.abs(),
                if y_offset > 0.0 {
                    "behind"
                } else {
                    "in front of"
                },
                y_reference.label().to_ascii_lowercase()
            )
        };
        format!(
            "Side {} / {horizontal} / {vertical}",
            if side_one { 1 } else { 2 }
        )
    }
    fn sync_reference_editor(&mut self) -> Option<ReferenceEditor> {
        if self.selected.len() != 1 {
            self.reference_editor = None;
            return None;
        }
        let id = *self.selected.iter().next().unwrap();
        let point = self.editable_position(id)?;
        if self
            .reference_editor
            .is_none_or(|editor| editor.performer_id != id || editor.source_point != point)
        {
            self.reference_editor = Some(ReferenceEditor::from_point(id, point));
        }
        self.reference_editor
    }
    fn yard_drag(ui: &mut egui::Ui, steps: &mut f32, maximum_yards: f32) -> bool {
        let mut yards = field::steps_to_yards(*steps);
        let changed = ui
            .add(
                egui::DragValue::new(&mut yards)
                    .speed(0.1)
                    .range(0.0..=maximum_yards)
                    .suffix(" yd"),
            )
            .changed();
        if changed {
            *steps = field::yards_to_steps(yards);
        }
        changed
    }
    fn selected_instrument(&self) -> Option<String> {
        let mut instruments = self.selected.iter().filter_map(|id| {
            self.show
                .performers
                .iter()
                .find(|performer| performer.id == *id)
                .map(|performer| performer.section.as_str())
        });
        let first = instruments.next()?.to_owned();
        if instruments.all(|instrument| instrument == first) {
            Some(first)
        } else {
            Some("Mixed".into())
        }
    }
    fn toggle_play(&mut self, ctx: &egui::Context) {
        if self.playing {
            self.playing = false;
            self.stop_audio();
            return;
        }
        if self.playhead >= self.show.total_counts() as f32 {
            self.playhead = 0.0;
            self.follow_playhead();
        }
        self.playing = true;
        self.start_audio();
        self.last_frame = Instant::now();
        ctx.request_repaint();
    }
    fn start_audio(&mut self) {
        self.stop_audio();
        let Some(path) = self.show.music.path.as_deref() else {
            return;
        };
        let seek =
            self.show.music.offset_seconds + self.playhead * 60.0 / self.show.music.bpm.max(1.0);
        match platform::start_audio(Path::new(path), seek) {
            Ok(process) => self.audio_process = Some(process),
            Err(error) => self.status = format!("Music playback failed: {error}"),
        }
    }
    fn stop_audio(&mut self) {
        if let Some(mut process) = self.audio_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
    fn choose_music_file() -> Option<PathBuf> {
        let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        platform::choose(platform::DialogKind::OpenMusic, &current)
    }
    fn attach_music(&mut self) {
        if let Some(path) = Self::choose_music_file() {
            self.snapshot();
            self.show.music.path = Some(path.to_string_lossy().into_owned());
            self.status = format!("Attached music: {}", path.display());
        }
    }
    fn new_drill(&mut self) {
        self.stop_audio();
        let shortcuts = self.shortcuts.clone();
        let recent_files = self.recent_files.clone();
        *self = Self::default();
        self.shortcuts = shortcuts;
        self.recent_files = recent_files;
        self.show_project_hub = false;
        self.dirty = true;
        self.status = "New unsaved drill".into();
    }
    fn request_new_drill(&mut self) {
        if self.dirty {
            self.confirm_new = true;
        } else {
            self.new_drill();
        }
    }
    fn run_action(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::NewDrill => self.request_new_drill(),
            Action::OpenDrill => self.open(),
            Action::SaveDrill => self.save(),
            Action::ExportChart => self.export(),
            Action::Undo => self.undo(),
            Action::CopyComponents => self.copy_components(),
            Action::PasteComponents => self.paste_components(),
            Action::ComponentTools => self.show_component_tools = true,
            Action::Redo => self.redo(),
            Action::DeleteSelected => self.delete_selected(),
            Action::SelectAll => {
                self.selected = self.show.performers.iter().map(|dot| dot.id).collect();
            }
            Action::ClearSelection => self.selected.clear(),
            Action::SelectTool => self.tool = Tool::Select,
            Action::PlaceTool => self.tool = Tool::Place,
            Action::PanTool => self.tool = Tool::Pan,
            Action::LineGuideTool => self.tool = Tool::LineGuide,
            Action::CircleGuideTool => self.tool = Tool::CircleGuide,
            Action::SplineGuideTool => self.tool = Tool::SplineGuide,
            Action::ProjectToGuide => self.project_selected_to_guide(),
            Action::DistributeOnGuide => self.distribute_selected_on_guide(),
            Action::AddSymmetryAxis => self.add_symmetry_axis(),
            Action::ToggleGridSnap => self.snap_to_grid = !self.snap_to_grid,
            Action::OpenShapePalette => self.show_shape_palette = true,
            Action::AddSubset => self.add_subset_at_playhead(),
            Action::DeleteSubset => self.delete_active_subset(),
            Action::ZoomToFit => self.fit_view(),
            Action::ToggleGrid => self.show_grid = !self.show_grid,
            Action::ToggleLabels => self.show_labels = !self.show_labels,
            Action::TogglePreview => self.show_preview = !self.show_preview,
            Action::ToggleInspector => self.show_inspector = !self.show_inspector,
            Action::AddSet => self.add_set(),
            Action::PreviousSet if self.current > 0 => self.set_current(self.current - 1),
            Action::NextSet if self.current + 1 < self.show.sets.len() => {
                self.set_current(self.current + 1)
            }
            Action::PreviousSet | Action::NextSet => {}
            Action::PlayPause => self.toggle_play(ctx),
            Action::Rewind => {
                self.playhead = 0.0;
                self.playing = false;
                self.follow_playhead();
            }
            Action::NudgeLeft => self.nudge(Point {
                x: -self.nudge_steps,
                y: 0.0,
            }),
            Action::NudgeRight => self.nudge(Point {
                x: self.nudge_steps,
                y: 0.0,
            }),
            Action::NudgeUp => self.nudge(Point {
                x: 0.0,
                y: -self.nudge_steps,
            }),
            Action::NudgeDown => self.nudge(Point {
                x: 0.0,
                y: self.nudge_steps,
            }),
            Action::NudgeLeftLarge => self.nudge(Point {
                x: -self.nudge_steps * 4.0,
                y: 0.0,
            }),
            Action::NudgeRightLarge => self.nudge(Point {
                x: self.nudge_steps * 4.0,
                y: 0.0,
            }),
            Action::NudgeUpLarge => self.nudge(Point {
                x: 0.0,
                y: -self.nudge_steps * 4.0,
            }),
            Action::NudgeDownLarge => self.nudge(Point {
                x: 0.0,
                y: self.nudge_steps * 4.0,
            }),
        }
    }
    fn key_commands(&mut self, ctx: &egui::Context) {
        if self.tool == Tool::MotionRoute && !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.motion_route.clear();
                self.tool = Tool::Select;
                self.status = "Route cancelled".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.tool = Tool::Select;
                self.status = "Route ready".into();
            }
            return;
        }
        if self.recording_shortcut.is_some() || ctx.egui_wants_keyboard_input() {
            return;
        }
        let action = ctx.input(|input| {
            input
                .events
                .iter()
                .find_map(|event| self.shortcuts.action_for_event(event))
        });
        if let Some(action) = action {
            self.run_action(action, ctx);
        }
    }
    fn shortcut_text(&self, label: &str, action: Action) -> String {
        format!("{label:<26}{}", self.shortcuts.display(action))
    }
    fn shortcut_tip(&self, label: &str, action: Action) -> String {
        format!("{} ({})", label, self.shortcuts.display(action))
    }
    fn save_shortcuts(&mut self) {
        let shortcut_path = platform::config_file(SHORTCUT_FILE);
        self.shortcut_status = match self.shortcuts.save(&shortcut_path) {
            Ok(()) => format!("Saved to {}", shortcut_path.display()),
            Err(error) => format!("Could not save shortcuts: {error}"),
        };
    }
    fn palette_title(ui: &mut egui::Ui, title: &str) -> bool {
        let mut close = false;
        egui::Frame::new().fill(theme::CHROME_DARK).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(theme::PALETTE_TITLE_HEIGHT);
            ui.horizontal(|ui| {
                ui.label(title);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui
                        .add_sized(
                            [theme::PALETTE_CLOSE_SIZE, theme::PALETTE_CLOSE_SIZE],
                            egui::Button::new("x"),
                        )
                        .on_hover_text("Close palette")
                        .clicked();
                });
            });
        });
        close
    }
    fn shortcut_editor(&mut self, ctx: &egui::Context) {
        if !self.show_shortcut_editor {
            return;
        }

        if let Some(action) = self.recording_shortcut {
            let pressed = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    let egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = event
                    else {
                        return None;
                    };
                    Some((*key, *modifiers))
                })
            });
            if let Some((key, modifiers)) = pressed {
                if key == egui::Key::Escape {
                    self.recording_shortcut = None;
                    self.shortcut_status = "Recording cancelled".into();
                } else if key == egui::Key::Backspace {
                    self.shortcuts
                        .bindings
                        .insert(action, Shortcut::unassigned());
                    self.recording_shortcut = None;
                    self.save_shortcuts();
                } else {
                    let candidate = Shortcut::from_key(key, modifiers);
                    if let Some(conflict) = self.shortcuts.conflict(action, &candidate) {
                        self.shortcut_status = format!(
                            "{} is already assigned to {}",
                            candidate.display(),
                            conflict.label()
                        );
                    } else {
                        self.shortcuts.bindings.insert(action, candidate);
                        self.recording_shortcut = None;
                        self.save_shortcuts();
                    }
                }
            }
        }

        let mut open = self.show_shortcut_editor;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .default_size([theme::SHORTCUT_WINDOW_WIDTH, theme::SHORTCUT_WINDOW_HEIGHT])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Restore Defaults").clicked() {
                        self.shortcuts = ShortcutConfig::default();
                        self.recording_shortcut = None;
                        self.save_shortcuts();
                    }
                    ui.label("Click a binding, then press a shortcut. Backspace clears it.");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("shortcut_grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                        .show(ui, |ui| {
                            for action in Action::ALL {
                                ui.label(action.label());
                                let text = if self.recording_shortcut == Some(action) {
                                    "Press shortcut…".into()
                                } else {
                                    self.shortcuts.display(action)
                                };
                                if ui
                                    .add_sized(
                                        [theme::SHORTCUT_BINDING_WIDTH, theme::TOOL_SIZE],
                                        egui::Button::new(text),
                                    )
                                    .clicked()
                                {
                                    self.recording_shortcut = Some(action);
                                    self.shortcut_status =
                                        "Press Escape to cancel or Backspace to clear".into();
                                }
                                if ui
                                    .add_sized(
                                        [theme::SHORTCUT_CLEAR_WIDTH, theme::TOOL_SIZE],
                                        egui::Button::new("Clear"),
                                    )
                                    .clicked()
                                {
                                    self.shortcuts
                                        .bindings
                                        .insert(action, Shortcut::unassigned());
                                    self.recording_shortcut = None;
                                    self.save_shortcuts();
                                }
                                ui.end_row();
                            }
                        });
                });
                ui.separator();
                ui.label(&self.shortcut_status);
            });
        self.show_shortcut_editor = open;
        if !open {
            self.recording_shortcut = None;
        }
    }
    fn shape_palette(&mut self, ctx: &egui::Context) {
        if !self.show_shape_palette {
            return;
        }
        let mut close = false;
        let mut chosen = None;
        let mut import_outline = false;
        egui::Window::new("Shapes")
            .title_bar(false)
            .frame(theme::palette_frame())
            .drag_area(egui::WindowDrag::Anywhere)
            .resizable(false)
            .show(ctx, |ui| {
                close = Self::palette_title(ui, "Shapes");
                egui::Grid::new("shape_palette_grid")
                    .num_columns(2)
                    .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                    .show(ui, |ui| {
                        for (index, (label, tip, preset)) in [
                            ("Arc", "Circular arc", ShapePreset::Arc),
                            (
                                "3-Pt Arc",
                                "Circular arc through three editable points",
                                ShapePreset::ThreePointArc,
                            ),
                            ("Block", "Rows and columns", ShapePreset::Block),
                            ("Polygon", "Open or filled polygon", ShapePreset::Polygon),
                            ("Spiral", "Spiral path", ShapePreset::Spiral),
                            (
                                "Polyline",
                                "Editable joined segments",
                                ShapePreset::Polyline,
                            ),
                            ("Fan", "Rays from one pivot", ShapePreset::Fan),
                            ("Rings", "Concentric circles", ShapePreset::Concentric),
                            ("Chevron", "V formation", ShapePreset::Chevron),
                            ("Text", "Bitmap text formation", ShapePreset::Text),
                            (
                                "Outline",
                                "Editable closed outline",
                                ShapePreset::ImportedPath,
                            ),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if ui
                                .add_sized(
                                    [theme::SHAPE_BUTTON_WIDTH, theme::INTERACT_HEIGHT],
                                    egui::Button::new(label),
                                )
                                .on_hover_text(tip)
                                .clicked()
                            {
                                chosen = Some(preset);
                            }
                            if index % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                import_outline = ui
                    .add_sized(
                        [
                            theme::SHAPE_BUTTON_WIDTH * 2.0 + theme::GRID_FORM_X_SPACING,
                            theme::INTERACT_HEIGHT,
                        ],
                        egui::Button::new("Import…"),
                    )
                    .on_hover_text("Import X,Y yard pairs from formation-outline.txt")
                    .clicked();
            });
        if close {
            self.show_shape_palette = false;
        }
        if let Some(preset) = chosen {
            self.add_shape(preset);
            self.show_shape_palette = false;
        }
        if import_outline {
            self.import_outline_file();
            self.show_shape_palette = false;
        }
    }
    fn music_setup_window(&mut self, ctx: &egui::Context) {
        if !self.show_music_setup {
            return;
        }
        let original = self.show.music.clone();
        let mut edited = original.clone();
        let mut close = false;
        let mut choose = false;
        let mut clear = false;
        egui::Window::new("Music Setup")
            .title_bar(false)
            .frame(theme::palette_frame())
            .drag_area(egui::WindowDrag::Anywhere)
            .resizable(false)
            .show(ctx, |ui| {
                close = Self::palette_title(ui, "Music Setup");
                egui::Grid::new("music_setup_grid")
                    .num_columns(2)
                    .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                    .show(ui, |ui| {
                        ui.label("File");
                        ui.label(
                            edited
                                .path
                                .as_deref()
                                .and_then(|path| Path::new(path).file_name())
                                .and_then(|name| name.to_str())
                                .unwrap_or("None"),
                        );
                        ui.end_row();
                        ui.label("BPM");
                        ui.add(
                            egui::DragValue::new(&mut edited.bpm)
                                .speed(0.5)
                                .range(20.0..=300.0)
                                .fixed_decimals(2),
                        );
                        ui.end_row();
                        ui.label("Audio offset");
                        ui.add(
                            egui::DragValue::new(&mut edited.offset_seconds)
                                .speed(0.1)
                                .range(0.0..=600.0)
                                .suffix(" s"),
                        );
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    choose = ui.button("Choose…").clicked();
                    clear = ui
                        .add_enabled(edited.path.is_some(), egui::Button::new("Remove"))
                        .clicked();
                });
            });
        if close {
            self.show_music_setup = false;
        }
        if edited != original {
            self.stop_audio();
            self.playing = false;
            self.snapshot();
            self.show.music = edited;
            self.status = format!("Music timing: {:.2} BPM", self.show.music.bpm);
        }
        if choose {
            self.attach_music();
        } else if clear {
            self.stop_audio();
            self.playing = false;
            self.snapshot();
            self.show.music.path = None;
            self.status = "Removed music track".into();
        }
    }
    fn compass_preview_window(&mut self, ctx: &egui::Context) {
        let Some(mut preview) = self.compass_preview.clone() else {
            return;
        };
        let mut approve = false;
        let mut cancel = false;
        egui::Window::new("Compass Match Proposal")
            .default_size([theme::COMPASS_WINDOW_WIDTH, theme::COMPASS_WINDOW_HEIGHT])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Plan");
                    let selected_name = preview
                        .candidates
                        .get(preview.selected_candidate)
                        .map_or("None", |candidate| candidate.name.as_str());
                    egui::ComboBox::from_id_salt("compass_candidate")
                        .selected_text(selected_name)
                        .show_ui(ui, |ui| {
                            for (index, candidate) in preview.candidates.iter().enumerate() {
                                ui.selectable_value(
                                    &mut preview.selected_candidate,
                                    index,
                                    candidate.name.as_str(),
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Scope");
                    egui::ComboBox::from_id_salt("compass_scope")
                        .selected_text(match preview.scope {
                            CompassScope::IncomingOnly => "Incoming only",
                            CompassScope::BothSides => "Both sides",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut preview.scope,
                                CompassScope::IncomingOnly,
                                "Incoming only",
                            );
                            ui.selectable_value(
                                &mut preview.scope,
                                CompassScope::BothSides,
                                "Both sides",
                            );
                        });
                });
                if let Some(candidate) = preview.candidates.get(preview.selected_candidate) {
                    ui.label(format!(
                        "Total {:.1} st   Max {:.1} st   Conflicts {}",
                        candidate.total_distance,
                        candidate.maximum_distance,
                        candidate.collision_risks
                    ));
                    if let Some(conflicts) = candidate.symmetry_conflicts {
                        ui.label(format!(
                            "Symmetry error {:.2} st   Conflicts {}",
                            candidate.symmetry_error, conflicts
                        ));
                    }
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("compass_proposal_grid")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            let spot_count = preview.spots.len();
                            let Some(candidate) =
                                preview.candidates.get_mut(preview.selected_candidate)
                            else {
                                return;
                            };
                            for index in 0..candidate.entries.len() {
                                let performer_id = candidate.entries[index].performer_id;
                                let label = self
                                    .show
                                    .performers
                                    .iter()
                                    .find(|performer| performer.id == performer_id)
                                    .map_or_else(
                                        || format!("Performer {performer_id}"),
                                        |performer| performer.label.clone(),
                                    );
                                ui.label(label);
                                let old_target = candidate.entries[index].target_index;
                                let mut chosen_target = old_target;
                                egui::ComboBox::from_id_salt(("compass_target", performer_id))
                                    .selected_text(format!("Spot {}", old_target + 1))
                                    .show_ui(ui, |ui| {
                                        for target in 0..spot_count {
                                            ui.selectable_value(
                                                &mut chosen_target,
                                                target,
                                                format!("Spot {}", target + 1),
                                            );
                                        }
                                    });
                                if chosen_target != old_target {
                                    if let Some(other) =
                                        candidate.entries.iter_mut().enumerate().find_map(
                                            |(other_index, entry)| {
                                                (other_index != index
                                                    && entry.target_index == chosen_target)
                                                    .then_some(entry)
                                            },
                                        )
                                    {
                                        other.target_index = old_target;
                                    }
                                    candidate.entries[index].target_index = chosen_target;
                                }
                                ui.end_row();
                            }
                        });
                });
                Self::refresh_compass_metrics(&mut preview);
                ui.separator();
                ui.horizontal(|ui| {
                    approve = ui.button("Approve Match").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if approve {
            self.compass_preview = Some(preview);
            self.approve_compass_preview();
        } else if cancel {
            self.compass_preview = None;
            self.status = "Cancelled compass proposal".into();
        } else {
            self.compass_preview = Some(preview);
        }
    }
    fn pick_chooser_window(&mut self, ctx: &egui::Context) {
        let Some(choice) = self.pick_chooser else {
            return;
        };
        let performer_label = self
            .show
            .performers
            .iter()
            .find(|performer| performer.id == choice.performer_id)
            .map_or("Performer", |performer| performer.label.as_str());
        let mut selected = None;
        egui::Window::new("Choose Overlapping Item")
            .current_pos(choice.position + Vec2::new(theme::PICKER_OFFSET, theme::PICKER_OFFSET))
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .frame(theme::palette_frame())
            .show(ctx, |ui| {
                ui.label("Overlapping items");
                if ui
                    .button(format!("Performer only: {performer_label}"))
                    .clicked()
                {
                    selected = Some(0);
                }
                if ui.button("Guide only").clicked() {
                    selected = Some(1);
                }
                if ui.button("Both").clicked() {
                    selected = Some(2);
                }
                if ui.button("Close").clicked() {
                    selected = Some(3);
                }
            });
        match selected {
            Some(0) => {
                self.selected.clear();
                self.selected.insert(choice.performer_id);
                self.selected_guide = None;
                self.pick_chooser = None;
            }
            Some(1) => {
                self.selected.clear();
                self.selected_guide = Some(choice.guide_id);
                self.pick_chooser = None;
            }
            Some(2) => {
                self.selected.clear();
                self.selected.insert(choice.performer_id);
                self.selected_guide = Some(choice.guide_id);
                self.pick_chooser = None;
            }
            Some(3) => self.pick_chooser = None,
            _ => {}
        }
    }
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .button(self.shortcut_text("New", Action::NewDrill))
                    .clicked()
                {
                    self.request_new_drill();
                }
                if ui
                    .button(self.shortcut_text("Open…", Action::OpenDrill))
                    .clicked()
                {
                    self.open();
                }
                if ui
                    .button(self.shortcut_text("Save", Action::SaveDrill))
                    .clicked()
                {
                    self.save();
                }
                if ui.button("Save As…").clicked() {
                    self.save_as();
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Export chart…", Action::ExportChart))
                    .clicked()
                {
                    self.export();
                }
                if ui.button("Export Performer Packets…").clicked() {
                    self.export_performer_packets();
                }
                ui.separator();
                if ui.button("Project Hub").clicked() {
                    self.playing = false;
                    self.stop_audio();
                    self.show_project_hub = true;
                }
            });
            ui.menu_button("Edit", |ui| {
                for (label, action) in [
                    ("Copy", Action::CopyComponents),
                    ("Paste", Action::PasteComponents),
                    ("Transform / Rotation Drill…", Action::ComponentTools),
                ] {
                    if ui.button(self.shortcut_text(label, action)).clicked() {
                        self.run_action(action, ui.ctx());
                    }
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Undo", Action::Undo))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .button(self.shortcut_text("Redo", Action::Redo))
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Delete selected", Action::DeleteSelected))
                    .clicked()
                {
                    self.delete_selected();
                }
                if ui
                    .button(self.shortcut_text("Select all", Action::SelectAll))
                    .clicked()
                {
                    self.run_action(Action::SelectAll, ui.ctx());
                }
                if ui
                    .button(self.shortcut_text("Clear selection", Action::ClearSelection))
                    .clicked()
                {
                    self.selected.clear();
                }
                ui.separator();
                if ui.button("Keyboard Shortcuts…").clicked() {
                    self.show_shortcut_editor = true;
                }
            });
            ui.menu_button("View", |ui| {
                if ui
                    .button(self.shortcut_text("Zoom to fit", Action::ZoomToFit))
                    .clicked()
                {
                    self.fit_view();
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Toggle grid", Action::ToggleGrid))
                    .clicked()
                {
                    self.show_grid = !self.show_grid;
                }
                if ui
                    .button(self.shortcut_text("Toggle labels", Action::ToggleLabels))
                    .clicked()
                {
                    self.show_labels = !self.show_labels;
                }
                if ui
                    .button(self.shortcut_text("Toggle grid snap", Action::ToggleGridSnap))
                    .clicked()
                {
                    self.snap_to_grid = !self.snap_to_grid;
                }
            });
            ui.menu_button("Set", |ui| {
                if ui
                    .button(self.shortcut_text("Add set", Action::AddSet))
                    .clicked()
                {
                    self.add_set();
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Add subset here", Action::AddSubset))
                    .clicked()
                {
                    self.add_subset_at_playhead();
                }
                if ui
                    .add_enabled(
                        self.active_subset.is_some(),
                        egui::Button::new(
                            self.shortcut_text("Delete active subset", Action::DeleteSubset),
                        ),
                    )
                    .clicked()
                {
                    self.delete_active_subset();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.current > 0,
                        egui::Button::new(self.shortcut_text("Previous set", Action::PreviousSet)),
                    )
                    .clicked()
                {
                    self.set_current(self.current - 1);
                }
                if ui
                    .add_enabled(
                        self.current + 1 < self.show.sets.len(),
                        egui::Button::new(self.shortcut_text("Next set", Action::NextSet)),
                    )
                    .clicked()
                {
                    self.set_current(self.current + 1);
                }
            });
            ui.menu_button("Perform", |ui| {
                if ui
                    .button(self.shortcut_text("Play / pause", Action::PlayPause))
                    .clicked()
                {
                    self.toggle_play(ui.ctx());
                }
                if ui.button("Music Setup…").clicked() {
                    self.show_music_setup = true;
                }
                if ui.button("Attach Music…").clicked() {
                    self.attach_music();
                }
            });
            ui.menu_button("Performers", |ui| {
                if ui
                    .button(self.shortcut_text("Place performer", Action::PlaceTool))
                    .clicked()
                {
                    self.tool = Tool::Place;
                }
                if ui
                    .button(self.shortcut_text("Delete selected", Action::DeleteSelected))
                    .clicked()
                {
                    self.delete_selected();
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui
                    .button(self.shortcut_text("Select", Action::SelectTool))
                    .clicked()
                {
                    self.tool = Tool::Select;
                }
                if ui
                    .button(self.shortcut_text("Pan", Action::PanTool))
                    .clicked()
                {
                    self.tool = Tool::Pan;
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Draw line guide", Action::LineGuideTool))
                    .clicked()
                {
                    self.tool = Tool::LineGuide;
                }
                if ui
                    .button(self.shortcut_text("Draw circle guide", Action::CircleGuideTool))
                    .clicked()
                {
                    self.tool = Tool::CircleGuide;
                }
                if ui
                    .button(self.shortcut_text("Draw spline guide", Action::SplineGuideTool))
                    .clicked()
                {
                    self.tool = Tool::SplineGuide;
                }
                if ui.button("Add Shape…").clicked() {
                    self.show_shape_palette = true;
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Project to guide", Action::ProjectToGuide))
                    .clicked()
                {
                    self.project_selected_to_guide();
                }
                if ui
                    .button(self.shortcut_text("Distribute on guide", Action::DistributeOnGuide))
                    .clicked()
                {
                    self.distribute_selected_on_guide();
                }
                ui.separator();
                if ui
                    .button(self.shortcut_text("Add symmetry axis", Action::AddSymmetryAxis))
                    .clicked()
                {
                    self.add_symmetry_axis();
                }
            });
            ui.menu_button("Window", |ui| {
                if ui
                    .button(self.shortcut_text("Preview", Action::TogglePreview))
                    .clicked()
                {
                    self.show_preview = !self.show_preview;
                }
                if ui
                    .button(self.shortcut_text("Inspector", Action::ToggleInspector))
                    .clicked()
                {
                    self.show_inspector = !self.show_inspector;
                }
            });
            ui.menu_button("Help", |ui| {
                ui.label(concat!("Cadence ", env!("CARGO_PKG_VERSION")));
                ui.label("Shortcuts appear in tooltips.");
            });
        });
    }
    fn draw_dashed_line(painter: &egui::Painter, start: Pos2, end: Pos2, stroke: Stroke) {
        let delta = end - start;
        let length = delta.length();
        if length <= f32::EPSILON {
            return;
        }
        let direction = delta / length;
        let stride = theme::CONSTRUCTION_DASH + theme::CONSTRUCTION_GAP;
        let mut distance = 0.0;
        while distance < length {
            let segment_end = (distance + theme::CONSTRUCTION_DASH).min(length);
            painter.line_segment(
                [
                    start + direction * distance,
                    start + direction * segment_end,
                ],
                stroke,
            );
            distance += stride;
        }
    }
    fn draw_field(&self, painter: &egui::Painter, canvas: Rect, view: FieldView) {
        painter.rect_filled(canvas, theme::ZERO_RADIUS, theme::CANVAS);
        let field = Rect::from_min_max(
            view.screen(Point { x: 0.0, y: 0.0 }),
            view.screen(Point {
                x: FIELD_W,
                y: FIELD_H,
            }),
        );
        painter.rect_filled(field, theme::ZERO_RADIUS, theme::CANVAS);
        if self.show_grid {
            let minor_step = if view.scale >= 5.0 {
                1
            } else if view.scale >= 2.5 {
                2
            } else {
                4
            };
            for x in (0..=160).step_by(minor_step) {
                let major = x % 5 == 0;
                let sx = view
                    .screen(Point {
                        x: x as f32,
                        y: 0.0,
                    })
                    .x;
                painter.line_segment(
                    [Pos2::new(sx, field.top()), Pos2::new(sx, field.bottom())],
                    Stroke::new(
                        if major {
                            theme::GRID_MAJOR_WIDTH
                        } else {
                            theme::GRID_MINOR_WIDTH
                        },
                        if major {
                            theme::GRID_MAJOR
                        } else {
                            theme::GRID_MINOR
                        },
                    ),
                );
            }
            for y in (0..=85).step_by(minor_step) {
                let major = y % 5 == 0;
                let sy = view
                    .screen(Point {
                        x: 0.0,
                        y: y as f32,
                    })
                    .y;
                painter.line_segment(
                    [Pos2::new(field.left(), sy), Pos2::new(field.right(), sy)],
                    Stroke::new(
                        if major {
                            theme::GRID_MAJOR_WIDTH
                        } else {
                            theme::GRID_MINOR_WIDTH
                        },
                        if major {
                            theme::GRID_MAJOR
                        } else {
                            theme::GRID_MINOR
                        },
                    ),
                );
            }
        }
        for yard in 0..=20 {
            let x = view
                .screen(Point {
                    x: yard as f32 * 8.0,
                    y: 0.0,
                })
                .x;
            painter.line_segment(
                [Pos2::new(x, field.top()), Pos2::new(x, field.bottom())],
                Stroke::new(
                    if yard % 2 == 0 {
                        theme::YARD_MAJOR_WIDTH
                    } else {
                        theme::YARD_MINOR_WIDTH
                    },
                    theme::FIELD_LINE,
                ),
            );
            if yard > 0 && yard < 20 {
                let n = if yard <= 10 {
                    yard * 5
                } else {
                    (20 - yard) * 5
                };
                let font = FontId::proportional(theme::FONT_FIELD);
                painter.text(
                    Pos2::new(
                        x,
                        field.top() + field.height() * theme::FIELD_NUMBER_VERTICAL_OFFSET,
                    ),
                    Align2::CENTER_CENTER,
                    n.to_string(),
                    font.clone(),
                    theme::YARD_NUMBER,
                );
                painter.text(
                    Pos2::new(
                        x,
                        field.bottom() - field.height() * theme::FIELD_NUMBER_VERTICAL_OFFSET,
                    ),
                    Align2::CENTER_CENTER,
                    n.to_string(),
                    font,
                    theme::YARD_NUMBER,
                );
            }
        }
        for hash in [
            field::yards_to_steps(field::FRONT_HASH_YARDS),
            field::yards_to_steps(field::BACK_HASH_YARDS),
        ] {
            let y = view.screen(Point { x: 0.0, y: hash }).y;
            for step in 0..=40 {
                let x = view
                    .screen(Point {
                        x: step as f32 * 4.0,
                        y: 0.0,
                    })
                    .x;
                painter.line_segment(
                    [
                        Pos2::new(x, y - theme::HASH_HALF_LENGTH),
                        Pos2::new(x, y + theme::HASH_HALF_LENGTH),
                    ],
                    Stroke::new(theme::HASH_LINE_WIDTH, theme::FIELD_LINE),
                );
            }
        }
        painter.rect_stroke(
            field,
            theme::ZERO_RADIUS,
            Stroke::new(theme::FIELD_BORDER_WIDTH, theme::FIELD_LINE),
            egui::StrokeKind::Inside,
        );
        for axis in &self.show.sets[self.current].symmetry_axes {
            let selected = self.selected_symmetry_axis == Some(axis.id);
            let color = if selected {
                theme::ACCENT
            } else {
                theme::MUTED
            };
            Self::draw_dashed_line(
                painter,
                view.screen(axis.start),
                view.screen(axis.end),
                Stroke::new(
                    if selected {
                        theme::GUIDE_SELECTED_WIDTH
                    } else {
                        theme::GUIDE_WIDTH
                    },
                    color,
                ),
            );
            if selected {
                for point in [axis.start, axis.end] {
                    painter.circle_filled(
                        view.screen(point),
                        theme::GUIDE_HANDLE_RADIUS,
                        theme::ACCENT,
                    );
                }
            }
        }
        for guide in &self.show.sets[self.current].guides {
            let selected = self.selected_guide == Some(guide.id);
            let stroke = Stroke::new(
                if selected {
                    theme::GUIDE_SELECTED_WIDTH
                } else {
                    theme::GUIDE_WIDTH
                },
                if selected {
                    theme::ACCENT
                } else {
                    theme::GUIDE
                },
            );
            for path in Self::guide_paths(guide) {
                for segment in path.windows(2) {
                    painter
                        .line_segment([view.screen(segment[0]), view.screen(segment[1])], stroke);
                }
            }
            if selected {
                match &guide.kind {
                    GuideKind::Line { start, end } => {
                        for point in [*start, *end] {
                            painter.circle_filled(
                                view.screen(point),
                                theme::GUIDE_HANDLE_RADIUS,
                                theme::ACCENT,
                            );
                        }
                    }
                    GuideKind::Circle { center, radius } => {
                        painter.circle_filled(
                            view.screen(*center),
                            theme::GUIDE_HANDLE_RADIUS,
                            theme::ACCENT,
                        );
                        painter.circle_filled(
                            view.screen(Point {
                                x: center.x + radius,
                                y: center.y,
                            }),
                            theme::GUIDE_HANDLE_RADIUS,
                            theme::ACCENT,
                        );
                    }
                    GuideKind::Spline {
                        start,
                        control_1,
                        control_2,
                        end,
                    } => {
                        painter.line_segment(
                            [view.screen(*start), view.screen(*control_1)],
                            Stroke::new(theme::HAIRLINE_WIDTH, theme::MUTED),
                        );
                        painter.line_segment(
                            [view.screen(*control_2), view.screen(*end)],
                            Stroke::new(theme::HAIRLINE_WIDTH, theme::MUTED),
                        );
                        for point in [*start, *control_1, *control_2, *end] {
                            painter.circle_filled(
                                view.screen(point),
                                theme::GUIDE_HANDLE_RADIUS,
                                theme::ACCENT,
                            );
                        }
                    }
                    GuideKind::ThreePointArc {
                        start,
                        through,
                        end,
                    } => {
                        for point in [*start, *through, *end] {
                            painter.circle_filled(
                                view.screen(point),
                                theme::GUIDE_HANDLE_RADIUS,
                                theme::ACCENT,
                            );
                        }
                    }
                    _ => {}
                }
            }
            for spot in Self::guide_spots(guide) {
                painter.circle_filled(
                    view.screen(spot),
                    theme::GUIDE_SPOT_RADIUS,
                    if selected {
                        theme::SELECT
                    } else {
                        theme::GUIDE
                    },
                );
            }
        }
        if let (Some(start), Some(current)) = (self.guide_start, self.guide_current) {
            let stroke = Stroke::new(theme::GUIDE_SELECTED_WIDTH, theme::ACCENT);
            match self.tool {
                Tool::LineGuide => {
                    painter.line_segment([view.screen(start), view.screen(current)], stroke);
                }
                Tool::CircleGuide => {
                    painter.circle_stroke(
                        view.screen(start),
                        start.distance(current) * view.scale,
                        stroke,
                    );
                }
                Tool::SplineGuide => {
                    painter.line_segment([view.screen(start), view.screen(current)], stroke);
                }
                _ => {}
            }
        }
        if let Some(preview) = &self.compass_preview
            && let Some(candidate) = preview.candidates.get(preview.selected_candidate)
        {
            for entry in &candidate.entries {
                let source = preview
                    .source_positions
                    .iter()
                    .find_map(|(id, point)| (*id == entry.performer_id).then_some(*point));
                let target = preview.spots.get(entry.target_index).copied();
                if let (Some(source), Some(target)) = (source, target) {
                    match candidate.route {
                        CompassRoute::Straight => {
                            painter.line_segment(
                                [view.screen(source), view.screen(target)],
                                Stroke::new(theme::HAIRLINE_WIDTH, theme::COMPASS_PREVIEW),
                            );
                        }
                        CompassRoute::Curve(bend) => {
                            let mut previous = source;
                            for step in 1..=12 {
                                let point = Self::curved_compass_point(
                                    source,
                                    target,
                                    step as f32 / 12.0,
                                    bend,
                                );
                                painter.line_segment(
                                    [view.screen(previous), view.screen(point)],
                                    Stroke::new(theme::HAIRLINE_WIDTH, theme::COMPASS_PREVIEW),
                                );
                                previous = point;
                            }
                        }
                    }
                    painter.circle_stroke(
                        view.screen(target),
                        theme::COMPASS_TARGET_RADIUS,
                        Stroke::new(theme::NORMAL_LINE_WIDTH, theme::COMPASS_PREVIEW),
                    );
                }
            }
        }
        for dot in &self.show.performers {
            if let Some(at) = self.show.position_at(dot.id, self.playhead) {
                let pos = view.screen(at);
                let selected = self.selected.contains(&dot.id);
                if selected
                    && self.current + 1 < self.show.sets.len()
                    && let Some(end) = self.show.sets[self.current + 1].positions.get(&dot.id)
                {
                    painter.line_segment(
                        [pos, view.screen(*end)],
                        Stroke::new(theme::NORMAL_LINE_WIDTH, theme::SELECT),
                    );
                    if at.distance(*end) / self.show.sets[self.current].counts_to_next as f32 > 1.0
                    {
                        painter.circle_stroke(
                            pos,
                            theme::SELECT_RADIUS + theme::SELECT_OUTER_RADIUS_OFFSET,
                            Stroke::new(theme::NORMAL_LINE_WIDTH, theme::ACCENT),
                        );
                    }
                }
                if selected {
                    painter.circle_stroke(
                        pos,
                        theme::SELECT_RADIUS,
                        Stroke::new(theme::SELECT_STROKE_WIDTH, theme::SELECT),
                    );
                }
                painter.circle_filled(pos, theme::DOT_RADIUS, self.dot_color(&dot.section));
                if self.show_labels && view.scale >= theme::LABEL_ZOOM_THRESHOLD {
                    painter.text(
                        pos + Vec2::new(theme::LABEL_OFFSET, -theme::LABEL_OFFSET),
                        Align2::LEFT_CENTER,
                        &dot.label,
                        FontId::proportional(theme::FONT_DOT),
                        theme::TEXT,
                    );
                }
            }
        }
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let painter = painter.with_clip_rect(response.rect);
        let mut view = self.view(response.rect);
        self.draw_field(&painter, response.rect, view);
        self.cursor_field = response.hover_pos().map(|p| view.field(p));
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01
                && let Some(cursor) = response.hover_pos()
            {
                let anchor = view.field(cursor);
                self.zoom = (self.zoom * (1.0 + scroll * 0.0025)).clamp(0.35, 8.0);
                let next = self.view(response.rect);
                self.pan += cursor - next.screen(anchor);
                view = self.view(response.rect);
            }
        }
        let pan_down = ui.input(|i| i.pointer.secondary_down())
            || self.tool == Tool::Pan && ui.input(|i| i.pointer.primary_down());
        if response.hovered() && pan_down {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if self.motion_route_canvas(ui, &response, &painter, view) {
            return;
        }
        if response.drag_started_by(PointerButton::Primary)
            && self.tool != Tool::Pan
            && let Some(pos) = response.interact_pointer_pos()
        {
            if matches!(
                self.tool,
                Tool::LineGuide | Tool::CircleGuide | Tool::SplineGuide
            ) {
                let at = self.snap_point(view.field(pos));
                self.guide_start = Some(at);
                self.guide_current = Some(at);
            } else if self.tool == Tool::Place {
                let at = self.snap_point(view.field(pos));
                if (0.0..=FIELD_W).contains(&at.x) && (0.0..=FIELD_H).contains(&at.y) {
                    self.snapshot();
                    let id = self.show.add_performer(at);
                    self.selected.clear();
                    self.selected.insert(id);
                }
            } else if let Some(id) = self.hit_dot(view, pos) {
                let shift = ui.input(|i| i.modifiers.shift);
                if shift {
                    if !self.selected.insert(id) {
                        self.selected.remove(&id);
                    }
                } else if !self.selected.contains(&id) {
                    self.selected.clear();
                    self.selected.insert(id);
                }
                self.snapshot();
                self.dragging_dots = true;
            } else {
                if !ui.input(|i| i.modifiers.shift) {
                    self.selected.clear();
                }
                self.box_start = Some(pos);
                self.box_current = Some(pos);
            }
        }
        if self.guide_start.is_some()
            && ui.input(|input| input.pointer.primary_down())
            && let Some(pos) = ui.input(|input| input.pointer.interact_pos())
        {
            self.guide_current = Some(self.snap_point(view.field(pos)));
        }
        if self.dragging_dots && ui.input(|i| i.pointer.primary_down()) {
            let delta = ui.input(|i| i.pointer.delta()) / view.scale;
            if delta != Vec2::ZERO {
                let updates: Vec<(u32, Point)> = self
                    .selected
                    .iter()
                    .filter_map(|id| {
                        self.editable_position(*id).map(|point| {
                            (
                                *id,
                                Point {
                                    x: (point.x + delta.x).clamp(0.0, FIELD_W),
                                    y: (point.y + delta.y).clamp(0.0, FIELD_H),
                                },
                            )
                        })
                    })
                    .collect();
                for (id, point) in updates {
                    self.set_editable_position(id, point);
                }
            }
        }
        if let Some(start) = self.box_start
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            self.box_current = Some(pos);
            let r = Rect::from_two_pos(start, pos);
            painter.rect_filled(r, theme::ZERO_RADIUS, theme::BOX_SELECT_FILL);
            painter.rect_stroke(
                r,
                theme::ZERO_RADIUS,
                Stroke::new(theme::NORMAL_LINE_WIDTH, theme::SELECT),
                egui::StrokeKind::Inside,
            );
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            let was_dragging_dots = self.dragging_dots;
            self.dragging_dots = false;
            if was_dragging_dots && self.snap_to_grid {
                let interval = self.grid_snap_steps.max(0.125);
                let updates: Vec<(u32, Point)> = self
                    .selected
                    .iter()
                    .filter_map(|id| {
                        self.editable_position(*id).map(|point| {
                            (
                                *id,
                                Point {
                                    x: (point.x / interval).round() * interval,
                                    y: (point.y / interval).round() * interval,
                                },
                            )
                        })
                    })
                    .collect();
                for (id, point) in updates {
                    self.set_editable_position(id, point);
                }
            }
            if let (Some(start), Some(end)) = (self.guide_start.take(), self.guide_current.take())
                && start.distance(end) >= 0.25
            {
                let id = self.show.next_guide_id();
                let kind = match self.tool {
                    Tool::LineGuide => Some(GuideKind::Line { start, end }),
                    Tool::CircleGuide => Some(GuideKind::Circle {
                        center: start,
                        radius: start.distance(end),
                    }),
                    Tool::SplineGuide => {
                        let dx = end.x - start.x;
                        let dy = end.y - start.y;
                        let length = (dx * dx + dy * dy).sqrt().max(0.001);
                        let bow = length * 0.2;
                        let normal = Point {
                            x: -dy / length * bow,
                            y: dx / length * bow,
                        };
                        let control_1 = self.snap_point(Point {
                            x: start.x + dx / 3.0 + normal.x,
                            y: start.y + dy / 3.0 + normal.y,
                        });
                        let control_2 = self.snap_point(Point {
                            x: start.x + dx * 2.0 / 3.0 + normal.x,
                            y: start.y + dy * 2.0 / 3.0 + normal.y,
                        });
                        Some(GuideKind::Spline {
                            start,
                            control_1,
                            control_2,
                            end,
                        })
                    }
                    _ => None,
                };
                if let Some(kind) = kind {
                    self.snapshot();
                    self.show.sets[self.current].guides.push(FormationGuide {
                        id,
                        spacing_steps: 2.0,
                        spot_count: if self.selected.is_empty() {
                            8
                        } else {
                            self.selected.len().max(2)
                        },
                        fill_guide: true,
                        reverse_spots: false,
                        kind,
                    });
                    self.selected_guide = Some(id);
                    self.status = "Created formation guide".into();
                }
            }
            if let (Some(a), Some(b)) = (self.box_start.take(), self.box_current.take()) {
                let r = Rect::from_two_pos(a, b);
                for dot in &self.show.performers {
                    if let Some(p) = self.show.position_at(dot.id, self.playhead)
                        && r.contains(view.screen(p))
                    {
                        self.selected.insert(dot.id);
                    }
                }
            }
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            if self.tool == Tool::Place {
                let at = self.snap_point(view.field(pos));
                if (0.0..=FIELD_W).contains(&at.x) && (0.0..=FIELD_H).contains(&at.y) {
                    self.snapshot();
                    let id = self.show.add_performer(at);
                    self.selected.clear();
                    self.selected.insert(id);
                }
            } else if self.tool == Tool::Select {
                let dot = self.hit_dot(view, pos);
                let guide = self.hit_guide(view.field(pos), theme::GUIDE_HIT_PIXELS / view.scale);
                let axis =
                    self.hit_symmetry_axis(view.field(pos), theme::GUIDE_HIT_PIXELS / view.scale);
                match (dot, guide) {
                    (Some(performer_id), Some(guide_id)) => {
                        if !ui.input(|input| input.modifiers.shift) {
                            self.selected.clear();
                        }
                        self.selected.insert(performer_id);
                        self.selected_guide = Some(guide_id);
                        self.selected_symmetry_axis = None;
                        self.pick_chooser = Some(PickChooser {
                            position: pos,
                            performer_id,
                            guide_id,
                        });
                    }
                    (Some(id), None) => {
                        if ui.input(|i| i.modifiers.shift) {
                            if !self.selected.insert(id) {
                                self.selected.remove(&id);
                            }
                        } else {
                            self.selected.clear();
                            self.selected.insert(id);
                        }
                        self.pick_chooser = None;
                        self.selected_symmetry_axis = None;
                    }
                    (None, Some(id)) => {
                        self.selected_guide = Some(id);
                        self.selected_symmetry_axis = None;
                        self.pick_chooser = None;
                    }
                    (None, None) if axis.is_some() => {
                        self.selected_symmetry_axis = axis;
                        self.selected_guide = None;
                        self.pick_chooser = None;
                    }
                    (None, None) if !ui.input(|i| i.modifiers.shift) => {
                        self.selected.clear();
                        self.selected_guide = None;
                        self.selected_symmetry_axis = None;
                        self.pick_chooser = None;
                    }
                    (None, None) => {}
                }
            }
        }
    }
    fn preview_palette(&mut self, ctx: &egui::Context) {
        if !self.show_preview {
            return;
        }
        egui::Window::new("3D Preview")
            .default_pos([
                theme::PREVIEW_START_X,
                theme::MENU_HEIGHT + theme::TOOLBAR_HEIGHT + theme::PALETTE_TOP_GAP,
            ])
            .default_size([theme::PREVIEW_WIDTH, theme::PREVIEW_HEIGHT])
            .frame(theme::palette_frame())
            .title_bar(false)
            .drag_area(egui::WindowDrag::Anywhere)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                if Self::palette_title(ui, "3D Preview") {
                    self.show_preview = false;
                }
                let (response, p) = ui.allocate_painter(ui.available_size(), Sense::hover());
                let r = response.rect;
                p.rect_filled(r, theme::ZERO_RADIUS, theme::PREVIEW_SURROUND);
                let top_y = r.top() + theme::PREVIEW_TOP_INSET;
                let bottom_y = r.bottom() - theme::PREVIEW_BOTTOM_INSET;
                let top_left = r.left() + theme::PREVIEW_TOP_SIDE_INSET;
                let top_right = r.right() - theme::PREVIEW_TOP_SIDE_INSET;
                let bottom_left = r.left() + theme::PREVIEW_BOTTOM_SIDE_INSET;
                let bottom_right = r.right() - theme::PREVIEW_BOTTOM_SIDE_INSET;
                p.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(top_left, top_y),
                        Pos2::new(top_right, top_y),
                        Pos2::new(bottom_right, bottom_y),
                        Pos2::new(bottom_left, bottom_y),
                    ],
                    theme::PREVIEW_FIELD,
                    Stroke::new(theme::NORMAL_LINE_WIDTH, theme::PREVIEW_BORDER),
                ));
                for yard in 0..=10 {
                    let t = yard as f32 / 10.0;
                    p.line_segment(
                        [
                            Pos2::new(egui::lerp(top_left..=top_right, t), top_y),
                            Pos2::new(egui::lerp(bottom_left..=bottom_right, t), bottom_y),
                        ],
                        Stroke::new(theme::HAIRLINE_WIDTH, theme::PREVIEW_LINE),
                    );
                }
                for dot in &self.show.performers {
                    if let Some(at) = self.show.position_at(dot.id, self.playhead) {
                        let depth = at.y / FIELD_H;
                        let left = egui::lerp(top_left..=bottom_left, depth);
                        let right = egui::lerp(top_right..=bottom_right, depth);
                        let pos = Pos2::new(
                            egui::lerp(left..=right, at.x / FIELD_W),
                            egui::lerp(top_y..=bottom_y, depth),
                        );
                        p.circle_filled(
                            pos,
                            theme::PREVIEW_DOT_RADIUS,
                            self.dot_color(&dot.section),
                        );
                    }
                }
            });
    }
    fn inspector_palette(&mut self, ctx: &egui::Context) {
        if !self.show_inspector {
            return;
        }
        let selected_instrument = self
            .selected_instrument()
            .unwrap_or_else(|| "No selection".into());
        let original_reference_editor = self.sync_reference_editor();
        let mut edited_reference_editor = original_reference_editor;
        let original_dot = if self.selected.len() == 1 {
            let id = *self.selected.iter().next().unwrap();
            self.editable_position(id).map(|point| (id, point))
        } else {
            None
        };
        let mut edited_dot = original_dot;
        let original_subset = self.active_subset().cloned();
        let mut edited_subset = original_subset.clone();
        let original_transition_counts = (self.current + 1 < self.show.sets.len())
            .then_some(self.show.sets[self.current].counts_to_next);
        let mut edited_transition_counts = original_transition_counts;
        let mut instrument_to_apply = None;
        let original_guide = self.selected_guide();
        let mut edited_guide = original_guide.clone();
        let original_axis = self.selected_symmetry_axis();
        let mut edited_axis = original_axis.clone();
        let mut delete_axis = false;
        let mut delete_guide = false;
        let mut project_to_guide = false;
        let mut distribute_on_guide = false;
        let mut apply_transform = false;
        let mut mirror_x = false;
        let mut mirror_y = false;
        let mut duplicate_offset = false;
        let mut convert_spline = false;
        let mut split_path = false;
        let mut join_path = false;
        let mut save_prefab = false;
        let mut insert_prefab = false;
        egui::Window::new("Performer / Set Inspector")
            .constrain_to(self.palette_bounds(ctx))
            .default_pos([
                self.palette_bounds(ctx).right() - theme::PANEL_WIDTH - theme::PALETTE_RIGHT_GAP,
                theme::MENU_HEIGHT + theme::TOOLBAR_HEIGHT + theme::OPTIONS_HEIGHT + theme::PALETTE_TOP_GAP,
            ])
            .default_size([theme::PANEL_WIDTH, theme::PANEL_HEIGHT])
            .frame(theme::palette_frame())
            .title_bar(false)
            .drag_area(egui::WindowDrag::Anywhere)
            .resizable(true)
            .vscroll(true)
            .collapsible(false)
            .show(ctx, |ui| {
                if Self::palette_title(ui, "Performer / Set Inspector") {
                    self.show_inspector = false;
                }
                egui::Grid::new("inspector_grid")
                    .num_columns(3)
                    .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                    .show(ui, |ui| {
                        ui.label("Set");
                        ui.label(format!("{} / {}", self.current + 1, self.show.sets.len()));
                        ui.end_row();
                        ui.label("Counts to next");
                        if let Some(counts) = &mut edited_transition_counts {
                            let minimum = self.show.sets[self.current]
                                .subsets
                                .iter()
                                .map(|subset| subset.offset_count.floor() as u32 + 1)
                                .max()
                                .unwrap_or(1);
                            ui.add(
                                egui::DragValue::new(counts)
                                    .speed(1)
                                    .range(minimum..=1024),
                            )
                            .on_hover_text("Length of this set-to-set transition");
                        } else {
                            ui.label("—");
                        }
                        ui.end_row();
                        ui.label("Count");
                        ui.label(format!(
                            "{:.2} / {}",
                            self.playhead,
                            self.show.total_counts()
                        ));
                        ui.end_row();
                        if let Some(subset) = &mut edited_subset {
                            ui.label("Subset");
                            ui.text_edit_singleline(&mut subset.name);
                            let maximum = self.show.sets[self.current].counts_to_next as f32 - 0.125;
                            ui.add(
                                egui::DragValue::new(&mut subset.offset_count)
                                    .speed(0.125)
                                    .range(0.125..=maximum.max(0.125))
                                    .fixed_decimals(3),
                            );
                            ui.end_row();
                        }
                        ui.label("Selected");
                        ui.label(self.selected.len().to_string());
                        ui.end_row();
                        ui.label("Coordinate");
                        ui.label(self.selected_coordinate());
                        ui.end_row();
                        ui.label("Drill terms");
                        ui.label(self.selected_drill_terms());
                        ui.end_row();
                        ui.label("Nudge steps");
                        ui.add(
                            egui::DragValue::new(&mut self.nudge_steps)
                                .speed(0.1)
                                .range(0.1..=16.0)
                                .suffix(" st"),
                        );
                        ui.end_row();
                        ui.label("Grid snap");
                        ui.checkbox(&mut self.snap_to_grid, "Enabled");
                        ui.add(
                            egui::DragValue::new(&mut self.grid_snap_steps)
                                .speed(0.125)
                                .range(0.125..=8.0)
                                .suffix(" st"),
                        );
                        ui.end_row();
                        ui.label("Instrument");
                        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
                            egui::ComboBox::from_id_salt("selected_instrument")
                                .selected_text(&selected_instrument)
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for instrument in INSTRUMENTS {
                                        if ui
                                            .selectable_label(
                                                selected_instrument == *instrument,
                                                *instrument,
                                            )
                                            .on_hover_text(format!(
                                                "Set instrument and relabel {} selected performer(s)",
                                                self.selected.len()
                                            ))
                                            .clicked()
                                        {
                                            instrument_to_apply = Some(*instrument);
                                            ui.close();
                                        }
                                    }
                                });
                        });
                        ui.end_row();
                        if self.selected.len() == 1 {
                            let id = *self.selected.iter().next().unwrap();
                            if let Some(i) = self.show.performers.iter().position(|p| p.id == id) {
                                let before = self.show.clone();
                                ui.label("Label");
                                let label_changed = ui
                                    .text_edit_singleline(&mut self.show.performers[i].label)
                                    .changed();
                                ui.end_row();
                                ui.label("Dot X / Y (yd)");
                                if let Some((point_id, point)) = &mut edited_dot
                                    && *point_id == id
                                {
                                    Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                                }
                                ui.end_row();
                                if let Some(editor) = &mut edited_reference_editor {
                                    ui.label("X reference");
                                    egui::ComboBox::from_id_salt("x_coordinate_reference")
                                        .selected_text(format!(
                                            "{:.0} yd line",
                                            editor.x_reference_yards
                                        ))
                                        .show_ui(ui, |ui| {
                                            for index in 0..=20 {
                                                let yard = index as f32 * 5.0;
                                                ui.selectable_value(
                                                    &mut editor.x_reference_yards,
                                                    yard,
                                                    format!("{yard:.0} yd line"),
                                                );
                                            }
                                        });
                                    ui.add(
                                        egui::DragValue::new(&mut editor.x_offset_steps)
                                            .speed(0.25)
                                            .range(-160.0..=160.0)
                                            .suffix(" st"),
                                    )
                                    .on_hover_text(
                                        "Positive is toward the 100-yard end; negative toward the 0-yard end",
                                    );
                                    ui.end_row();
                                    ui.label("Y reference");
                                    egui::ComboBox::from_id_salt("y_coordinate_reference")
                                        .selected_text(editor.y_reference.label())
                                        .show_ui(ui, |ui| {
                                            for reference in field::VerticalReference::ALL {
                                                ui.selectable_value(
                                                    &mut editor.y_reference,
                                                    reference,
                                                    reference.label(),
                                                );
                                            }
                                        });
                                    ui.add(
                                        egui::DragValue::new(&mut editor.y_offset_steps)
                                            .speed(0.25)
                                            .range(-FIELD_H..=FIELD_H)
                                            .suffix(" st"),
                                    )
                                    .on_hover_text(
                                        "Positive is toward the back sideline; negative toward the front sideline",
                                    );
                                    ui.end_row();
                                }
                                if label_changed {
                                    self.undo.push(before);
                                    self.redo.clear();
                                    self.dirty = true;
                                }
                            }
                        }
                    });
                if let Some(axis) = &mut edited_axis {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Symmetry construction axis");
                        if ui.button("Delete").clicked() {
                            delete_axis = true;
                        }
                    });
                    egui::Grid::new("symmetry_axis_grid")
                        .num_columns(3)
                        .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                        .show(ui, |ui| {
                            ui.label("Name");
                            ui.text_edit_singleline(&mut axis.name);
                            ui.end_row();
                            ui.label("Start X / Y (yd)");
                            Self::yard_drag(ui, &mut axis.start.x, field::LENGTH_YARDS);
                            Self::yard_drag(ui, &mut axis.start.y, field::WIDTH_YARDS);
                            ui.end_row();
                            ui.label("End X / Y (yd)");
                            Self::yard_drag(ui, &mut axis.end.x, field::LENGTH_YARDS);
                            Self::yard_drag(ui, &mut axis.end.y, field::WIDTH_YARDS);
                            ui.end_row();
                            ui.label("Mode");
                            egui::ComboBox::from_id_salt("symmetry_mode")
                                .selected_text(match axis.mode {
                                    SymmetryMode::Perfect => "Perfect",
                                    SymmetryMode::Try => "Try",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut axis.mode,
                                        SymmetryMode::Perfect,
                                        "Perfect",
                                    );
                                    ui.selectable_value(
                                        &mut axis.mode,
                                        SymmetryMode::Try,
                                        "Try",
                                    );
                                });
                            ui.end_row();
                            ui.label("Threshold");
                            ui.add_enabled(
                                axis.mode == SymmetryMode::Try,
                                egui::DragValue::new(&mut axis.threshold_steps)
                                    .speed(0.25)
                                    .range(0.0..=32.0)
                                    .suffix(" st"),
                            );
                            ui.end_row();
                        });
                }
                if let Some(guide) = &mut edited_guide {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(match &guide.kind {
                            GuideKind::Line { .. } => "Line guide",
                            GuideKind::Circle { .. } => "Circle guide",
                            GuideKind::Spline { .. } => "Spline guide",
                            GuideKind::Arc { .. } => "Arc guide",
                            GuideKind::ThreePointArc { .. } => "Three-point arc",
                            GuideKind::Block { .. } => "Block / grid",
                            GuideKind::Polygon { .. } => "Polygon",
                            GuideKind::Spiral { .. } => "Spiral",
                            GuideKind::Polyline { .. } => "Freeform polyline",
                            GuideKind::Fan { .. } => "Fan / wedge",
                            GuideKind::Concentric { .. } => "Concentric rings",
                            GuideKind::Chevron { .. } => "Chevron",
                            GuideKind::Text { .. } => "Text formation",
                            GuideKind::ImportedPath { .. } => "Imported outline",
                        });
                        if ui.button("Delete Guide").clicked() {
                            delete_guide = true;
                        }
                    });
                    egui::Grid::new("guide_grid")
                        .num_columns(3)
                        .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                        .show(ui, |ui| {
                            match &mut guide.kind {
                                GuideKind::Line { start, end } => {
                                    ui.label("Start X / Y (yd)");
                                    Self::yard_drag(ui, &mut start.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut start.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("End X / Y (yd)");
                                    Self::yard_drag(ui, &mut end.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut end.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                }
                                GuideKind::Circle { center, radius } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Radius (yd)");
                                    Self::yard_drag(ui, radius, field::LENGTH_YARDS);
                                    *radius = (*radius).max(0.25);
                                    ui.end_row();
                                }
                                GuideKind::Spline {
                                    start,
                                    control_1,
                                    control_2,
                                    end,
                                } => {
                                    for (label, point) in [
                                        ("Start X / Y (yd)", start),
                                        ("Control 1 X / Y", control_1),
                                        ("Control 2 X / Y", control_2),
                                        ("End X / Y (yd)", end),
                                    ] {
                                        ui.label(label);
                                        Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                                        Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                                        ui.end_row();
                                    }
                                }
                                GuideKind::Arc {
                                    center,
                                    radius,
                                    start_degrees,
                                    sweep_degrees,
                                } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Radius / angles");
                                    Self::yard_drag(ui, radius, field::LENGTH_YARDS);
                                    ui.add(egui::DragValue::new(start_degrees).suffix("° start"));
                                    ui.add(egui::DragValue::new(sweep_degrees).suffix("° sweep"));
                                    ui.end_row();
                                }
                                GuideKind::ThreePointArc {
                                    start,
                                    through,
                                    end,
                                } => {
                                    for (label, point) in [
                                        ("Start X / Y (yd)", start),
                                        ("Through X / Y", through),
                                        ("End X / Y (yd)", end),
                                    ] {
                                        ui.label(label);
                                        Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                                        Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                                        ui.end_row();
                                    }
                                }
                                GuideKind::Block {
                                    center,
                                    width,
                                    height,
                                    rows,
                                    columns,
                                    stagger,
                                    rotation_degrees,
                                } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Width / height");
                                    Self::yard_drag(ui, width, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, height, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Rows / columns");
                                    ui.add(egui::DragValue::new(rows).range(1..=64));
                                    ui.add(egui::DragValue::new(columns).range(1..=64));
                                    ui.end_row();
                                    ui.label("Block options");
                                    ui.checkbox(stagger, "Stagger rows");
                                    ui.add(
                                        egui::DragValue::new(rotation_degrees).suffix("° rotate"),
                                    );
                                    ui.end_row();
                                }
                                GuideKind::Polygon {
                                    center,
                                    radius,
                                    sides,
                                    rotation_degrees,
                                    filled,
                                } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Radius / sides");
                                    Self::yard_drag(ui, radius, field::LENGTH_YARDS);
                                    ui.add(egui::DragValue::new(sides).range(3..=32));
                                    ui.end_row();
                                    ui.label("Polygon options");
                                    ui.checkbox(filled, "Filled");
                                    ui.add(
                                        egui::DragValue::new(rotation_degrees).suffix("° rotate"),
                                    );
                                    ui.end_row();
                                }
                                GuideKind::Spiral {
                                    center,
                                    start_radius,
                                    end_radius,
                                    turns,
                                    rotation_degrees,
                                } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Start / end radius");
                                    Self::yard_drag(ui, start_radius, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, end_radius, field::LENGTH_YARDS);
                                    ui.end_row();
                                    ui.label("Turns / rotation");
                                    ui.add(egui::DragValue::new(turns).range(-12.0..=12.0));
                                    ui.add(
                                        egui::DragValue::new(rotation_degrees).suffix("° rotate"),
                                    );
                                    ui.end_row();
                                }
                                GuideKind::Polyline { points, closed }
                                | GuideKind::ImportedPath { points, closed } => {
                                    let mut point_count = points.len().max(2);
                                    ui.label("Point count / closed");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut point_count).range(2..=64),
                                        )
                                        .changed()
                                    {
                                        let last = points.last().copied().unwrap_or_default();
                                        points.resize(point_count, last);
                                    }
                                    ui.checkbox(closed, "Closed");
                                    ui.end_row();
                                    for (index, point) in points.iter_mut().enumerate() {
                                        ui.label(format!("Point {} X / Y", index + 1));
                                        Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                                        Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                                        ui.end_row();
                                    }
                                }
                                GuideKind::Fan {
                                    pivot,
                                    radius,
                                    start_degrees,
                                    sweep_degrees,
                                    rays,
                                    spots_per_ray,
                                } => {
                                    ui.label("Pivot X / Y (yd)");
                                    Self::yard_drag(ui, &mut pivot.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut pivot.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Radius / angles");
                                    Self::yard_drag(ui, radius, field::LENGTH_YARDS);
                                    ui.add(egui::DragValue::new(start_degrees).suffix("° start"));
                                    ui.add(egui::DragValue::new(sweep_degrees).suffix("° sweep"));
                                    ui.end_row();
                                    ui.label("Rays / spots per ray");
                                    ui.add(egui::DragValue::new(rays).range(1..=64));
                                    ui.add(egui::DragValue::new(spots_per_ray).range(1..=64));
                                    ui.end_row();
                                }
                                GuideKind::Concentric {
                                    center,
                                    start_radius,
                                    ring_spacing,
                                    rings,
                                    spots_per_ring,
                                } => {
                                    ui.label("Center X / Y (yd)");
                                    Self::yard_drag(ui, &mut center.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut center.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Start / ring spacing");
                                    Self::yard_drag(ui, start_radius, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, ring_spacing, field::LENGTH_YARDS);
                                    ui.end_row();
                                    ui.label("Rings / spots per ring");
                                    ui.add(egui::DragValue::new(rings).range(1..=32));
                                    ui.add(egui::DragValue::new(spots_per_ring).range(1..=128));
                                    ui.end_row();
                                }
                                GuideKind::Chevron { left, pivot, right } => {
                                    for (label, point) in [
                                        ("Left X / Y", left),
                                        ("Pivot X / Y", pivot),
                                        ("Right X / Y", right),
                                    ] {
                                        ui.label(label);
                                        Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                                        Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                                        ui.end_row();
                                    }
                                }
                                GuideKind::Text {
                                    origin,
                                    text,
                                    cell_size,
                                } => {
                                    ui.label("Origin X / Y (yd)");
                                    Self::yard_drag(ui, &mut origin.x, field::LENGTH_YARDS);
                                    Self::yard_drag(ui, &mut origin.y, field::WIDTH_YARDS);
                                    ui.end_row();
                                    ui.label("Text / cell size");
                                    ui.text_edit_singleline(text);
                                    Self::yard_drag(ui, cell_size, 10.0);
                                    ui.end_row();
                                }
                            }
                            let uses_spot_count = !matches!(
                                &guide.kind,
                                GuideKind::Block { .. }
                                    | GuideKind::Fan { .. }
                                    | GuideKind::Concentric { .. }
                                    | GuideKind::Text { .. }
                            );
                            ui.label(if uses_spot_count {
                                "Spot count"
                            } else {
                                "Generated spots"
                            });
                            if uses_spot_count {
                                ui.add(
                                    egui::DragValue::new(&mut guide.spot_count)
                                        .speed(1)
                                        .range(1..=256),
                                );
                            } else {
                                ui.label(Self::guide_spots(guide).len().to_string());
                            }
                            ui.end_row();
                            ui.label("Dot spacing");
                            ui.checkbox(&mut guide.fill_guide, "Fill guide");
                            ui.add_enabled(
                                !guide.fill_guide,
                                egui::DragValue::new(&mut guide.spacing_steps)
                                    .speed(0.1)
                                    .range(0.25..=32.0)
                                    .suffix(" st"),
                            )
                            .on_hover_text(
                                "Turn off Fill guide to use this exact step interval",
                            );
                            ui.end_row();
                            ui.label("Actual interval");
                            ui.label(format!("{:.2} st", Self::guide_actual_spacing(guide)));
                            ui.end_row();
                            ui.label("Spot order");
                            ui.checkbox(&mut guide.reverse_spots, "Reverse");
                            ui.end_row();
                        });
                    ui.horizontal(|ui| {
                        project_to_guide = ui
                            .add_enabled(!self.selected.is_empty(), egui::Button::new("Project"))
                            .on_hover_text(
                                "Place selected performers uniformly on the configured guide spots",
                            )
                            .clicked();
                        distribute_on_guide = ui
                            .add_enabled(
                                !self.selected.is_empty(),
                                egui::Button::new("Compass Match"),
                            )
                            .on_hover_text(
                                "Match selected performers to guide spots with minimum total travel",
                            )
                            .clicked();
                    });
                    ui.separator();
                    egui::Grid::new("guide_transform_grid")
                        .num_columns(4)
                        .spacing([theme::GRID_FORM_X_SPACING, theme::GRID_FORM_Y_SPACING])
                        .show(ui, |ui| {
                            ui.label("Offset X / Y");
                            ui.add(
                                egui::DragValue::new(&mut self.guide_offset_x_yards)
                                    .speed(0.25)
                                    .suffix(" yd"),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.guide_offset_y_yards)
                                    .speed(0.25)
                                    .suffix(" yd"),
                            );
                            ui.end_row();
                            ui.label("Rotate / scale");
                            ui.add(
                                egui::DragValue::new(&mut self.guide_rotation_degrees)
                                    .speed(1.0)
                                    .suffix("°"),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.guide_scale)
                                    .speed(0.05)
                                    .range(0.05..=20.0),
                            );
                            ui.end_row();
                        });
                    ui.horizontal_wrapped(|ui| {
                        apply_transform = ui.button("Apply Transform").clicked();
                        mirror_x = ui.button("Mirror X").clicked();
                        mirror_y = ui.button("Mirror Y").clicked();
                        duplicate_offset = ui.button("Offset Copy").clicked();
                    });
                    ui.horizontal_wrapped(|ui| {
                        convert_spline = ui.button("Convert to Spline").clicked();
                        split_path = ui.button("Split Path").clicked();
                        join_path = ui.button("Join Nearest").clicked();
                    });
                    ui.horizontal_wrapped(|ui| {
                        save_prefab = ui.button("Save Prefab").clicked();
                        insert_prefab = ui.button("Insert Prefab").clicked();
                    });
                }
            });
        if delete_axis {
            if let Some(id) = self.selected_symmetry_axis {
                self.snapshot();
                self.show.sets[self.current]
                    .symmetry_axes
                    .retain(|axis| axis.id != id);
                self.selected_symmetry_axis = None;
                self.status = "Deleted symmetry construction axis".into();
            }
        } else if let (Some(original), Some(edited)) = (original_axis, edited_axis)
            && original != edited
        {
            self.snapshot();
            if let Some(saved) = self.show.sets[self.current]
                .symmetry_axes
                .iter_mut()
                .find(|axis| axis.id == edited.id)
            {
                *saved = edited;
            }
        }
        if edited_transition_counts != original_transition_counts
            && let Some(counts) = edited_transition_counts
        {
            self.snapshot();
            let timing_ratio = counts as f32 / self.show.sets[self.current].counts_to_next as f32;
            for timing in self.show.sets[self.current].timing.values_mut() {
                timing.delay *= timing_ratio;
                timing.move_counts *= timing_ratio;
            }
            self.show.sets[self.current].counts_to_next = counts;
            let transition_end = self.show.set_start(self.current) + counts as f32;
            if self.playhead > transition_end {
                self.playhead = transition_end;
                self.follow_playhead();
            }
            self.status = format!("Set {}: {counts} counts", self.current + 1);
        }
        if edited_subset != original_subset
            && let Some(edited) = edited_subset
        {
            self.snapshot();
            if let Some(saved) = self.show.sets[self.current]
                .subsets
                .iter_mut()
                .find(|subset| subset.id == edited.id)
            {
                *saved = edited.clone();
            }
            self.show.sets[self.current]
                .subsets
                .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
            self.playhead = self.show.set_start(self.current) + edited.offset_count;
            self.status = format!("{} at count {:.3}", edited.name, edited.offset_count);
        }
        if edited_dot != original_dot
            && let Some((id, point)) = edited_dot
        {
            self.snapshot();
            self.set_editable_position(id, point);
        }
        if let Some(instrument) = instrument_to_apply {
            self.snapshot();
            self.show.assign_instrument(&self.selected, instrument);
            self.status = format!(
                "Assigned {instrument} to {} performer(s)",
                self.selected.len()
            );
        }
        if let (Some(original), Some(mut edited)) =
            (original_reference_editor, edited_reference_editor)
            && original != edited
        {
            let point = edited.point();
            let point = Point {
                x: point.x.clamp(0.0, FIELD_W),
                y: point.y.clamp(0.0, FIELD_H),
            };
            self.snapshot();
            self.set_editable_position(edited.performer_id, point);
            edited.source_point = point;
            self.reference_editor = Some(edited);
            self.status = "Positioned performer from field references".into();
        }
        if delete_guide {
            if let Some(id) = self.selected_guide {
                self.snapshot();
                self.show.sets[self.current]
                    .guides
                    .retain(|guide| guide.id != id);
                self.selected_guide = None;
                self.status = "Deleted formation guide".into();
            }
        } else if let (Some(original), Some(edited)) = (original_guide, edited_guide)
            && original != edited
        {
            self.snapshot();
            if let Some(saved) = self.show.sets[self.current]
                .guides
                .iter_mut()
                .find(|guide| guide.id == edited.id)
            {
                *saved = edited;
            }
        }
        if project_to_guide {
            self.project_selected_to_guide();
        }
        if distribute_on_guide {
            self.distribute_selected_on_guide();
        }
        if apply_transform || mirror_x || mirror_y {
            self.apply_selected_guide_transform(mirror_x, mirror_y);
        }
        if duplicate_offset {
            self.duplicate_selected_guide();
        }
        if convert_spline && let Some(mut guide) = self.selected_guide() {
            self.snapshot();
            Self::convert_guide_to_spline(&mut guide);
            if let Some(saved) = self.show.sets[self.current]
                .guides
                .iter_mut()
                .find(|saved| saved.id == guide.id)
            {
                *saved = guide;
            }
        }
        if split_path {
            self.split_selected_guide();
        }
        if join_path {
            self.join_selected_to_nearest();
        }
        if save_prefab {
            self.save_selected_prefab();
        }
        if insert_prefab {
            self.insert_saved_prefab();
        }
    }
    fn timeline(&mut self, ui: &mut egui::Ui) {
        let total = self.show.total_counts().max(1) as f32;
        let mut beat_changed = false;
        ui.horizontal(|ui| {
            ui.label("Beat");
            beat_changed = ui
                .add(
                    egui::DragValue::new(&mut self.playhead)
                        .speed(0.25)
                        .range(0.0..=total)
                        .fixed_decimals(2),
                )
                .changed();
            let local = self.playhead - self.show.set_start(self.current);
            ui.label(format!("Transition count {local:.2}"));
            ui.label(format!("{:.1} BPM", self.show.music.bpm));
            if self.show.music.path.is_some() {
                ui.label("MUSIC");
            }
            ui.checkbox(&mut self.snap_playhead, "Snap");
            ui.add_enabled(
                self.snap_playhead,
                egui::DragValue::new(&mut self.beat_snap)
                    .speed(0.25)
                    .range(0.125..=8.0)
                    .suffix(" beat"),
            );
            if ui.button("Add Subset").clicked() {
                self.add_subset_at_playhead();
            }
            if ui
                .add_enabled(
                    self.active_subset.is_some(),
                    egui::Button::new("Delete Subset"),
                )
                .clicked()
            {
                self.delete_active_subset();
            }
            if let Some(subset) = self.active_subset() {
                ui.label(format!("Active: {}", subset.name));
            }
        });
        if beat_changed {
            self.playing = false;
            self.playhead = self.playhead.clamp(0.0, total);
            self.follow_playhead();
        }
        let (response, p) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height()),
            Sense::click_and_drag(),
        );
        let r = response.rect;
        p.rect_filled(r, theme::ZERO_RADIUS, theme::CHROME_LIGHT);
        let left = r.left() + theme::TIMELINE_LEFT_INSET;
        let right = r.right() - theme::TIMELINE_RIGHT_INSET;
        let y = r.center().y;
        p.line_segment(
            [Pos2::new(left, y), Pos2::new(right, y)],
            Stroke::new(theme::NORMAL_LINE_WIDTH, theme::BORDER),
        );
        for count in 0..=total.ceil() as u32 {
            let x = egui::lerp(left..=right, count as f32 / total);
            let major = count % 8 == 0;
            p.line_segment(
                [
                    Pos2::new(
                        x,
                        y - if major {
                            theme::TIMELINE_MAJOR_TICK
                        } else {
                            theme::TIMELINE_MINOR_TICK
                        },
                    ),
                    Pos2::new(
                        x,
                        y + if major {
                            theme::TIMELINE_MAJOR_TICK
                        } else {
                            theme::TIMELINE_MINOR_TICK
                        },
                    ),
                ],
                Stroke::new(
                    theme::NORMAL_LINE_WIDTH,
                    if major { theme::TEXT } else { theme::BORDER },
                ),
            );
            if major {
                p.text(
                    Pos2::new(x, y + theme::TIMELINE_LABEL_GAP),
                    Align2::CENTER_TOP,
                    count.to_string(),
                    FontId::proportional(theme::FONT_SMALL),
                    theme::MUTED,
                );
            }
        }
        for i in 0..self.show.sets.len() {
            let count = self.show.set_start(i);
            let x = egui::lerp(left..=right, count / total);
            p.line_segment(
                [
                    Pos2::new(x, r.top() + theme::TIMELINE_EDGE_GAP),
                    Pos2::new(x, r.bottom() - theme::TIMELINE_EDGE_GAP),
                ],
                Stroke::new(
                    theme::SET_MARKER_WIDTH,
                    if i == self.current {
                        theme::ACCENT
                    } else {
                        theme::DOT_PERCUSSION
                    },
                ),
            );
            p.text(
                Pos2::new(
                    x + theme::TIMELINE_EDGE_GAP,
                    r.top() + theme::NORMAL_LINE_WIDTH,
                ),
                Align2::LEFT_TOP,
                (i + 1).to_string(),
                FontId::proportional(theme::FONT_SMALL),
                theme::TEXT,
            );
        }
        for (set_index, set) in self.show.sets.iter().enumerate() {
            let set_start = self.show.set_start(set_index);
            for subset in &set.subsets {
                let count = set_start + subset.offset_count;
                let x = egui::lerp(left..=right, count / total);
                let active = self.active_subset == Some(subset.id);
                p.line_segment(
                    [
                        Pos2::new(x, r.top() + theme::TIMELINE_EDGE_GAP),
                        Pos2::new(x, r.bottom() - theme::TIMELINE_EDGE_GAP),
                    ],
                    Stroke::new(
                        if active {
                            theme::SET_MARKER_WIDTH
                        } else {
                            theme::NORMAL_LINE_WIDTH
                        },
                        if active { theme::SELECT } else { theme::GUIDE },
                    ),
                );
                p.text(
                    Pos2::new(x + theme::TIMELINE_EDGE_GAP, r.top()),
                    Align2::LEFT_TOP,
                    format!("s{}", subset.id),
                    FontId::proportional(theme::FONT_SMALL),
                    theme::MUTED,
                );
            }
        }
        let x = egui::lerp(left..=right, (self.playhead / total).clamp(0.0, 1.0));
        p.line_segment(
            [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
            Stroke::new(theme::EMPHASIS_LINE_WIDTH, theme::ACCENT),
        );
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x - theme::TIMELINE_PLAYHEAD_HALF, r.top()),
                Pos2::new(x + theme::TIMELINE_PLAYHEAD_HALF, r.top()),
                Pos2::new(x, r.top() + theme::TIMELINE_PLAYHEAD_POINT),
            ],
            theme::ACCENT,
            Stroke::NONE,
        ));
        if (response.clicked() || response.dragged()) && response.interact_pointer_pos().is_some() {
            let px = response.interact_pointer_pos().unwrap().x;
            let raw = ((px - left) / (right - left) * total).clamp(0.0, total);
            let subset_hit = self
                .show
                .sets
                .iter()
                .enumerate()
                .flat_map(|(set_index, set)| {
                    let start = self.show.set_start(set_index);
                    set.subsets
                        .iter()
                        .map(move |subset| (start + subset.offset_count, subset.id))
                })
                .filter_map(|(count, id)| {
                    let marker_x = egui::lerp(left..=right, count / total);
                    ((marker_x - px).abs() <= theme::GUIDE_HIT_PIXELS).then_some((
                        count,
                        id,
                        (marker_x - px).abs(),
                    ))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((count, id, _)) = subset_hit {
                self.playhead = count;
                self.active_subset = Some(id);
            } else {
                self.playhead = if self.snap_playhead {
                    let interval = self.beat_snap.max(0.125);
                    (raw / interval).round() * interval
                } else {
                    raw
                }
                .clamp(0.0, total);
                self.follow_playhead();
            }
            self.playing = false;
        }
    }
    fn project_hub(&mut self, ui: &mut egui::Ui) {
        let mut open_path = None;
        let mut create_new = false;
        let mut browse = false;
        let mut resume = false;
        let mut clear_recent = false;
        ui.vertical_centered(|ui| {
            ui.set_max_width(theme::HUB_CONTENT_WIDTH);
            ui.add_space(theme::TOOLBAR_HEIGHT);
            ui.horizontal(|ui| {
                let (mark, _) =
                    ui.allocate_exact_size(Vec2::splat(theme::HUB_MARK_SIZE), Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(mark, theme::ZERO_RADIUS, theme::CANVAS);
                painter.rect_stroke(
                    mark,
                    theme::ZERO_RADIUS,
                    Stroke::new(theme::BORDER_WIDTH, theme::BORDER),
                    egui::StrokeKind::Inside,
                );
                for division in 1..4 {
                    let x = egui::lerp(mark.left()..=mark.right(), division as f32 / 4.0);
                    let y = egui::lerp(mark.top()..=mark.bottom(), division as f32 / 4.0);
                    painter.line_segment(
                        [Pos2::new(x, mark.top()), Pos2::new(x, mark.bottom())],
                        Stroke::new(theme::HAIRLINE_WIDTH, theme::GRID_MAJOR),
                    );
                    painter.line_segment(
                        [Pos2::new(mark.left(), y), Pos2::new(mark.right(), y)],
                        Stroke::new(theme::HAIRLINE_WIDTH, theme::GRID_MAJOR),
                    );
                }
                painter.text(
                    mark.center(),
                    Align2::CENTER_CENTER,
                    "M",
                    FontId::proportional(theme::FONT_FIELD),
                    theme::ACCENT,
                );
                ui.vertical(|ui| {
                    ui.heading("Cadence");
                    ui.label("Project Hub");
                });
            });
            ui.add_space(theme::TOOLBAR_HEIGHT);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(theme::HUB_SIDEBAR_WIDTH, ui.available_height()),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.label("PROJECT");
                        ui.separator();
                        create_new = ui
                            .add_sized(
                                [ui.available_width(), theme::HUB_COMMAND_HEIGHT],
                                egui::Button::new("New Drill"),
                            )
                            .clicked();
                        browse = ui
                            .add_sized(
                                [ui.available_width(), theme::HUB_COMMAND_HEIGHT],
                                egui::Button::new("Open Drill…"),
                            )
                            .clicked();
                        if self.file_path.is_some() || self.dirty {
                            resume = ui
                                .add_sized(
                                    [ui.available_width(), theme::HUB_COMMAND_HEIGHT],
                                    egui::Button::new("Return to Workspace"),
                                )
                                .clicked();
                        }
                        ui.add_space(theme::TOOLBAR_HEIGHT);
                        ui.label("DOCUMENT FORMAT");
                        ui.separator();
                        ui.label(format!("*.{FILE_EXTENSION}"));
                        ui.label("Version 1");
                        ui.label("Legacy JSON opens normally");
                        if !self.recent_files.is_empty() {
                            ui.add_space(theme::TOOLBAR_HEIGHT);
                            clear_recent = ui.button("Clear Recent Files").clicked();
                        }
                    },
                );
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label("RECENT PROJECTS");
                    ui.separator();
                    if self.recent_files.is_empty() {
                        ui.label("No recent projects");
                    }
                    for path_text in self.recent_files.clone() {
                        let path = PathBuf::from(&path_text);
                        let name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(&path_text);
                        let exists = path.is_file();
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), theme::HUB_RECENT_ROW_HEIGHT),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(exists, egui::Button::new(name))
                                    .on_hover_text(&path_text)
                                    .clicked()
                                {
                                    open_path = Some(path.clone());
                                }
                                ui.label(if exists { &path_text } else { "Missing" });
                            },
                        );
                    }
                });
            });
        });
        if create_new {
            self.request_new_drill();
        } else if browse {
            self.open();
        } else if resume {
            self.show_project_hub = false;
        } else if clear_recent {
            self.recent_files.clear();
            let _ = fs::write(platform::config_file(RECENT_FILE), b"[]");
        } else if let Some(path) = open_path {
            self.request_open(path);
        }
    }
    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let cursor = self
                .cursor_field
                .map(|p| {
                    format!(
                        "X {:>6.2} yd  Y {:>6.2} yd",
                        field::steps_to_yards(p.x),
                        field::steps_to_yards(p.y)
                    )
                })
                .unwrap_or_else(|| "X   --    Y   --".into());
            ui.label(cursor);
            ui.separator();
            ui.label(format!("Selected: {}", self.selected.len()));
            ui.separator();
            ui.label(format!(
                "Set {} / {}",
                self.current + 1,
                self.show.sets.len()
            ));
            ui.separator();
            ui.label(format!(
                "Count {:.1} / {}",
                self.playhead,
                self.show.total_counts()
            ));
            ui.separator();
            ui.label(if self.snap_to_grid {
                format!("Snap: {:.3} st", self.grid_snap_steps)
            } else {
                "Snap: Off".into()
            });
            ui.separator();
            ui.label(&self.status);
        });
    }
}

impl eframe::App for DrillApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let document_name = self
            .file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.mdrill");
        let title = if self.show_project_hub {
            "Cadence — Project Hub".to_owned()
        } else if self.dirty {
            format!("Cadence — {document_name} *")
        } else {
            format!("Cadence — {document_name}")
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        if ctx.input(|input| input.viewport().close_requested()) && self.dirty && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if self.playing && !self.show_project_hub {
            let dt = self.last_frame.elapsed().as_secs_f32();
            self.last_frame = Instant::now();
            self.playhead = (self.playhead + dt * self.show.music.bpm / 60.0)
                .min(self.show.total_counts() as f32);
            self.follow_playhead();
            if self.playhead >= self.show.total_counts() as f32 {
                self.playing = false;
                self.stop_audio();
            } else {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }
        if !self.playing && self.audio_process.is_some() {
            self.stop_audio();
        }
        if self.show_project_hub {
            egui::Panel::top("hub_header")
                .exact_size(theme::HUB_HEADER_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.label("CADENCE");
                        ui.separator();
                        ui.label("Project Hub");
                    });
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme::CHROME_LIGHT))
                .show(ui, |ui| self.project_hub(ui));
        } else {
            self.key_commands(&ctx);
            egui::Panel::top("menu")
                .exact_size(theme::MENU_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| self.menu_bar(ui));
            egui::Panel::top("toolbar")
                .exact_size(theme::TOOLBAR_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| self.workspace_toolbar(ui));
            egui::Panel::top("tool_options")
                .exact_size(theme::OPTIONS_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| self.workspace_options(ui));
            egui::Panel::bottom("status")
                .exact_size(theme::STATUS_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| self.status_bar(ui));
            egui::Panel::bottom("timeline")
                .exact_size(theme::TIMELINE_HEIGHT)
                .frame(theme::chrome_frame())
                .show(ui, |ui| self.timeline(ui));
            if self.show_component_tools {
                egui::Panel::right("tool_properties")
                    .default_size(theme::COMMAND_PANEL_WIDTH)
                    .resizable(true)
                    .frame(theme::chrome_frame())
                    .show(ui, |ui| self.component_tools_panel(ui));
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme::CANVAS))
                .show(ui, |ui| self.canvas(ui));
            self.preview_palette(&ctx);
            self.inspector_palette(&ctx);
            self.enforce_sketch_constraints();
            self.shortcut_editor(&ctx);
            self.shape_palette(&ctx);
            self.music_setup_window(&ctx);
            self.compass_preview_window(&ctx);
            self.pick_chooser_window(&ctx);
        }
        if self.confirm_new {
            egui::Window::new("Unsaved Drill")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Save changes before creating a new drill?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_new = false;
                        }
                        if ui.button("Save and Create").clicked() {
                            self.save();
                            if !self.dirty {
                                self.confirm_new = false;
                                self.new_drill();
                            }
                        }
                        if ui.button("Discard and Create").clicked() {
                            self.confirm_new = false;
                            self.new_drill();
                        }
                    });
                });
        }
        if self.confirm_open {
            egui::Window::new("Unsaved Drill")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Opening another drill will discard unsaved edits.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_open = false;
                            self.pending_open_path = None;
                        }
                        if ui.button("Save and Open").clicked() {
                            self.save();
                            if !self.dirty {
                                self.open_now();
                            }
                        }
                        if ui.button("Discard and Open").clicked() {
                            self.open_now();
                        }
                    });
                });
        }
        if self.confirm_close {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Save changes before closing Cadence?");
                    ui.horizontal(|ui| {
                        if ui.button("Save and Close").clicked() {
                            self.save();
                            if !self.dirty {
                                self.confirm_close = false;
                                self.allow_close = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        if ui.button("Discard and Close").clicked() {
                            self.confirm_close = false;
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_close = false;
                        }
                    });
                });
        }
    }

    fn on_exit(&mut self) {
        self.stop_audio();
    }
}

fn app_icon() -> egui::IconData {
    const SIZE: usize = 64;
    let mut rgba = vec![0_u8; SIZE * SIZE * 4];
    let mut pixel = |x: usize, y: usize, color: [u8; 4]| {
        let offset = (y * SIZE + x) * 4;
        rgba[offset..offset + 4].copy_from_slice(&color);
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = x < 2 || y < 2 || x >= SIZE - 2 || y >= SIZE - 2;
            pixel(
                x,
                y,
                if border {
                    [127, 37, 41, 255]
                } else {
                    [255, 255, 255, 255]
                },
            );
        }
    }
    for coordinate in (8..SIZE - 8).step_by(8) {
        for value in 5..SIZE - 5 {
            pixel(coordinate, value, [156, 211, 225, 255]);
            pixel(value, coordinate, [156, 211, 225, 255]);
        }
    }
    for y in 16..49 {
        for x in 16..48 {
            if x < 21 || !(21..44).contains(&y) {
                pixel(x, y, [127, 37, 41, 255]);
            }
        }
    }
    for y in 28..36 {
        for x in 43..51 {
            pixel(x, y, [237, 157, 30, 255]);
        }
    }
    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([theme::WINDOW_WIDTH, theme::WINDOW_HEIGHT])
            .with_title("Cadence")
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Cadence",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(DrillApp::default()))
        }),
    )
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    fn test_guide(kind: GuideKind, spot_count: usize) -> FormationGuide {
        FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count,
            fill_guide: true,
            reverse_spots: false,
            kind,
        }
    }

    #[test]
    fn projects_a_point_to_line_and_circle_guides() {
        let line = FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 3,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Line {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        };
        assert_eq!(
            DrillApp::project_to_guide(&line, Point { x: 4.0, y: 3.0 }),
            Point { x: 4.0, y: 0.0 }
        );

        let circle = FormationGuide {
            id: 2,
            spacing_steps: 2.0,
            spot_count: 8,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Circle {
                center: Point { x: 10.0, y: 10.0 },
                radius: 5.0,
            },
        };
        assert_eq!(
            DrillApp::project_to_guide(&circle, Point { x: 20.0, y: 10.0 }),
            Point { x: 15.0, y: 10.0 }
        );
    }

    #[test]
    fn three_point_arc_passes_through_all_construction_points() {
        let start = Point { x: 0.0, y: 10.0 };
        let through = Point { x: 10.0, y: 0.0 };
        let end = Point { x: 20.0, y: 10.0 };
        let guide = FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 9,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::ThreePointArc {
                start,
                through,
                end,
            },
        };
        let path = DrillApp::guide_paths(&guide).remove(0);

        assert!(path.first().unwrap().distance(start) < 0.001);
        assert!(path.last().unwrap().distance(end) < 0.001);
        assert!(path.iter().any(|point| point.distance(through) < 0.05));
        assert_eq!(DrillApp::guide_spots(&guide).len(), 9);
    }

    #[test]
    fn collinear_three_point_arc_has_a_safe_polyline_fallback() {
        let guide = FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 3,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::ThreePointArc {
                start: Point { x: 0.0, y: 0.0 },
                through: Point { x: 5.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        };

        assert_eq!(DrillApp::guide_paths(&guide)[0].len(), 3);
        assert_eq!(DrillApp::guide_spots(&guide).len(), 3);
    }

    #[test]
    fn distributes_selected_performers_at_guide_spacing() {
        let mut app = DrillApp::default();
        for x in 0..3 {
            let id = app.show.add_performer(Point {
                x: x as f32,
                y: 5.0,
            });
            app.selected.insert(id);
        }
        app.show.sets[0].guides.push(FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 3,
            fill_guide: false,
            reverse_spots: false,
            kind: GuideKind::Line {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        });
        app.selected_guide = Some(1);
        app.distribute_selected_on_guide();
        assert!(app.compass_preview.is_some());
        assert!(app.approve_compass_preview());

        let points: Vec<Point> = app.show.sets[0].positions.values().copied().collect();
        assert_eq!(points[0], Point { x: 3.0, y: 0.0 });
        assert_eq!(points[1], Point { x: 5.0, y: 0.0 });
        assert_eq!(points[2], Point { x: 7.0, y: 0.0 });
    }

    #[test]
    fn compass_match_uses_and_updates_the_active_subset() {
        let mut app = DrillApp::default();
        let first = app.show.add_performer(Point { x: 0.0, y: 0.0 });
        let second = app.show.add_performer(Point { x: 10.0, y: 0.0 });
        app.show.add_set(0);
        app.show.sets[0].subsets = vec![
            Subset {
                id: 1,
                name: "Subset 1".into(),
                offset_count: 2.0,
                positions: BTreeMap::from([
                    (first, Point { x: 10.0, y: 2.0 }),
                    (second, Point { x: 0.0, y: 2.0 }),
                ]),
            },
            Subset {
                id: 2,
                name: "Subset 2".into(),
                offset_count: 4.0,
                positions: BTreeMap::from([
                    (first, Point { x: 5.0, y: 3.0 }),
                    (second, Point { x: 5.0, y: 3.0 }),
                ]),
            },
        ];
        app.active_subset = Some(2);
        app.playhead = 4.0;
        app.selected = BTreeSet::from([first, second]);
        app.show.sets[0].guides.push(FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 2,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Line {
                start: Point { x: 0.0, y: 4.0 },
                end: Point { x: 10.0, y: 4.0 },
            },
        });
        app.selected_guide = Some(1);

        app.distribute_selected_on_guide();
        let names: Vec<&str> = app
            .compass_preview
            .as_ref()
            .unwrap()
            .candidates
            .iter()
            .map(|candidate| candidate.name.as_str())
            .collect();
        assert!(names.contains(&"Shortest total"));
        assert!(names.contains(&"Avoid collisions"));
        assert!(names.contains(&"Curve left"));
        assert!(names.contains(&"Avoid + curve left"));
        assert!(app.approve_compass_preview());

        assert_eq!(app.show.sets[0].positions[&first], Point { x: 0.0, y: 0.0 });
        assert_eq!(
            app.show.sets[0].subsets[0].positions[&first],
            Point { x: 10.0, y: 2.0 }
        );
        assert_eq!(
            app.show.sets[0].subsets[1].positions[&first],
            Point { x: 10.0, y: 4.0 }
        );
        assert_eq!(
            app.show.sets[0].subsets[1].positions[&second],
            Point { x: 0.0, y: 4.0 }
        );
    }

    #[test]
    fn even_effort_reduces_the_longest_move() {
        let sources = [
            Point { x: 3.0, y: 3.0 },
            Point { x: 1.0, y: 5.0 },
            Point { x: 4.0, y: 5.0 },
        ];
        let targets = [
            Point { x: 0.0, y: 4.0 },
            Point { x: 4.0, y: 1.0 },
            Point { x: 2.0, y: 2.0 },
        ];
        let shortest = DrillApp::minimum_cost_assignment(&sources, &targets);
        let even = DrillApp::minimum_maximum_assignment(&sources, &targets);
        let maximum = |assignment: &[usize]| {
            sources
                .iter()
                .enumerate()
                .map(|(index, source)| source.distance(targets[assignment[index]]))
                .fold(0.0, f32::max)
        };

        assert_eq!(shortest, vec![2, 0, 1]);
        assert_eq!(even, vec![1, 0, 2]);
        assert!(maximum(&even) < maximum(&shortest));
    }

    #[test]
    fn collision_plan_swaps_crossing_paths_when_a_clear_option_exists() {
        let sources = [Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }];
        let targets = [Point { x: 10.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }];
        let crossing = vec![0, 1];
        let before = DrillApp::assignment_collision_metrics(
            &sources,
            &targets,
            &crossing,
            CompassRoute::Straight,
        );
        let safe = DrillApp::collision_avoiding_assignment(
            &sources,
            &targets,
            std::slice::from_ref(&crossing),
        );
        let after = DrillApp::assignment_collision_metrics(
            &sources,
            &targets,
            &safe,
            CompassRoute::Straight,
        );

        assert_eq!(before.0, 1);
        assert_eq!(safe, vec![1, 0]);
        assert_eq!(after.0, 0);
    }

    #[test]
    fn incoming_only_carries_the_spot_assignment_forward() {
        let mut app = DrillApp::default();
        let first = app.show.add_performer(Point { x: 0.0, y: 0.0 });
        let second = app.show.add_performer(Point { x: 10.0, y: 0.0 });
        app.show.add_set(0);
        app.show.add_set(1);
        app.show.sets[2]
            .positions
            .insert(first, Point { x: 2.0, y: 8.0 });
        app.show.sets[2]
            .positions
            .insert(second, Point { x: 18.0, y: 8.0 });
        app.compass_preview = Some(CompassPreview {
            candidates: vec![CompassCandidate {
                name: "Shortest total".into(),
                entries: vec![
                    CompassEntry {
                        performer_id: first,
                        target_index: 0,
                    },
                    CompassEntry {
                        performer_id: second,
                        target_index: 1,
                    },
                ],
                route: CompassRoute::Straight,
                total_distance: 0.0,
                maximum_distance: 0.0,
                collision_risks: 0,
                symmetry_conflicts: None,
                symmetry_error: 0.0,
                symmetry_axis: None,
            }],
            selected_candidate: 0,
            spots: vec![Point { x: 10.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }],
            source_positions: vec![
                (first, Point { x: 0.0, y: 0.0 }),
                (second, Point { x: 10.0, y: 0.0 }),
            ],
            original_destination_positions: vec![
                (first, Point { x: 0.0, y: 0.0 }),
                (second, Point { x: 10.0, y: 0.0 }),
            ],
            destination_set: 1,
            destination_subset: None,
            interval: Some(CompassInterval {
                set_index: 0,
                source_offset: 0.0,
                destination_offset: 8.0,
            }),
            scope: CompassScope::IncomingOnly,
        });

        assert!(app.approve_compass_preview());
        assert_eq!(
            app.show.sets[1].positions[&first],
            Point { x: 10.0, y: 0.0 }
        );
        assert_eq!(
            app.show.sets[2].positions[&first],
            Point { x: 18.0, y: 8.0 }
        );
        assert_eq!(
            app.show.sets[2].positions[&second],
            Point { x: 2.0, y: 8.0 }
        );
    }

    #[test]
    fn curved_compass_plan_creates_playable_intermediate_subsets() {
        let mut app = DrillApp::default();
        let performer = app.show.add_performer(Point { x: 10.0, y: 10.0 });
        app.show.add_set(0);
        app.compass_preview = Some(CompassPreview {
            candidates: vec![CompassCandidate {
                name: "Curve left".into(),
                entries: vec![CompassEntry {
                    performer_id: performer,
                    target_index: 0,
                }],
                route: CompassRoute::Curve(0.22),
                total_distance: 0.0,
                maximum_distance: 0.0,
                collision_risks: 0,
                symmetry_conflicts: None,
                symmetry_error: 0.0,
                symmetry_axis: None,
            }],
            selected_candidate: 0,
            spots: vec![Point { x: 20.0, y: 10.0 }],
            source_positions: vec![(performer, Point { x: 10.0, y: 10.0 })],
            original_destination_positions: vec![(performer, Point { x: 10.0, y: 10.0 })],
            destination_set: 1,
            destination_subset: None,
            interval: Some(CompassInterval {
                set_index: 0,
                source_offset: 0.0,
                destination_offset: 8.0,
            }),
            scope: CompassScope::IncomingOnly,
        });

        assert!(app.approve_compass_preview());
        assert_eq!(app.show.sets[0].subsets.len(), 2);
        assert!((app.show.sets[0].subsets[0].offset_count - 8.0 / 3.0).abs() < 0.001);
        assert!(app.show.sets[0].subsets[0].positions[&performer].y > 10.0);
        assert_eq!(
            app.show.sets[1].positions[&performer],
            Point { x: 20.0, y: 10.0 }
        );
    }

    #[test]
    fn spline_spots_follow_the_curve_and_include_endpoints_when_spacing_fills_it() {
        let guide = FormationGuide {
            id: 1,
            spacing_steps: 1000.0,
            spot_count: 3,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Spline {
                start: Point { x: 0.0, y: 0.0 },
                control_1: Point { x: 0.0, y: 10.0 },
                control_2: Point { x: 10.0, y: 10.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        };
        let spots = DrillApp::guide_spots(&guide);
        assert_eq!(spots[0], Point { x: 0.0, y: 0.0 });
        assert_eq!(spots[2], Point { x: 10.0, y: 0.0 });
        assert!(spots[1].y > 7.0);
    }

    #[test]
    fn compass_matching_minimizes_total_travel() {
        let sources = [Point { x: 9.0, y: 0.0 }, Point { x: 1.0, y: 0.0 }];
        let targets = [Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }];
        assert_eq!(
            DrillApp::minimum_cost_assignment(&sources, &targets),
            vec![1, 0]
        );
    }

    #[test]
    fn grid_snapping_defaults_on_and_uses_the_configured_interval() {
        let mut app = DrillApp {
            grid_snap_steps: 0.5,
            ..DrillApp::default()
        };
        assert_eq!(
            app.snap_point(Point { x: 1.24, y: 2.76 }),
            Point { x: 1.0, y: 3.0 }
        );
        app.snap_to_grid = false;
        assert_eq!(
            app.snap_point(Point { x: 1.24, y: 2.76 }),
            Point { x: 1.24, y: 2.76 }
        );
    }

    #[test]
    fn project_uses_uniform_guide_spots_instead_of_nearest_points() {
        let mut app = DrillApp::default();
        for point in [
            Point { x: 12.0, y: 10.0 },
            Point { x: 13.0, y: 10.0 },
            Point { x: 14.0, y: 10.0 },
            Point { x: 15.0, y: 10.0 },
        ] {
            let id = app.show.add_performer(point);
            app.selected.insert(id);
        }
        app.show.sets[0].guides.push(FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 4,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Circle {
                center: Point { x: 10.0, y: 10.0 },
                radius: 5.0,
            },
        });
        app.selected_guide = Some(1);
        app.project_selected_to_guide();

        let mut positions: Vec<(i32, i32)> = app.show.sets[0]
            .positions
            .values()
            .map(|point| (point.x.round() as i32, point.y.round() as i32))
            .collect();
        positions.sort_unstable();
        assert_eq!(positions, vec![(5, 10), (10, 5), (10, 15), (15, 10)]);
    }

    #[test]
    fn compound_shapes_generate_their_requested_spots() {
        let center = Point { x: 50.0, y: 40.0 };
        let block = test_guide(
            GuideKind::Block {
                center,
                width: 20.0,
                height: 10.0,
                rows: 3,
                columns: 4,
                stagger: false,
                rotation_degrees: 0.0,
            },
            8,
        );
        assert_eq!(DrillApp::guide_spots(&block).len(), 12);

        let fan = test_guide(
            GuideKind::Fan {
                pivot: center,
                radius: 12.0,
                start_degrees: -90.0,
                sweep_degrees: 180.0,
                rays: 3,
                spots_per_ray: 4,
            },
            8,
        );
        assert_eq!(DrillApp::guide_spots(&fan).len(), 12);

        let rings = test_guide(
            GuideKind::Concentric {
                center,
                start_radius: 4.0,
                ring_spacing: 4.0,
                rings: 2,
                spots_per_ring: 8,
            },
            8,
        );
        assert_eq!(DrillApp::guide_spots(&rings).len(), 16);

        let text = test_guide(
            GuideKind::Text {
                origin: center,
                text: "A1".into(),
                cell_size: 1.0,
            },
            8,
        );
        assert!(DrillApp::guide_spots(&text).len() > 20);
    }

    #[test]
    fn arc_spiral_polygon_and_chevron_use_shared_path_spotting() {
        let center = Point { x: 50.0, y: 40.0 };
        let kinds = [
            GuideKind::Arc {
                center,
                radius: 10.0,
                start_degrees: 0.0,
                sweep_degrees: 180.0,
            },
            GuideKind::Polygon {
                center,
                radius: 10.0,
                sides: 5,
                rotation_degrees: 0.0,
                filled: false,
            },
            GuideKind::Spiral {
                center,
                start_radius: 1.0,
                end_radius: 10.0,
                turns: 2.0,
                rotation_degrees: 0.0,
            },
            GuideKind::Chevron {
                left: Point { x: 40.0, y: 50.0 },
                pivot: center,
                right: Point { x: 60.0, y: 50.0 },
            },
        ];
        for kind in kinds {
            assert_eq!(DrillApp::guide_spots(&test_guide(kind, 9)).len(), 9);
        }
    }

    #[test]
    fn common_transform_and_reversal_apply_to_guide_spots() {
        let mut guide = test_guide(
            GuideKind::Line {
                start: Point { x: 10.0, y: 10.0 },
                end: Point { x: 20.0, y: 10.0 },
            },
            3,
        );
        DrillApp::transform_guide(
            &mut guide,
            Point { x: 5.0, y: 0.0 },
            2.0,
            90.0,
            false,
            false,
        );
        let forward = DrillApp::guide_spots(&guide);
        assert!((forward[0].distance(forward[2]) - 20.0).abs() < 0.001);
        guide.reverse_spots = true;
        let reversed = DrillApp::guide_spots(&guide);
        assert_eq!(forward[0], reversed[2]);
        assert_eq!(forward[2], reversed[0]);
    }

    #[test]
    fn perfect_symmetry_compass_finds_mirrored_destinations() {
        let axis = SymmetryAxis {
            id: 1,
            name: "Center".into(),
            start: Point { x: 10.0, y: 0.0 },
            end: Point { x: 10.0, y: 30.0 },
            threshold_steps: 0.5,
            mode: SymmetryMode::Perfect,
        };
        let sources = [
            Point { x: 5.0, y: 0.0 },
            Point { x: 15.0, y: 0.0 },
            Point { x: 7.0, y: 3.0 },
            Point { x: 13.0, y: 3.0 },
        ];
        let targets = [
            Point { x: 4.0, y: 12.0 },
            Point { x: 14.0, y: 20.0 },
            Point { x: 16.0, y: 12.0 },
            Point { x: 6.0, y: 20.0 },
        ];
        let shortest = DrillApp::minimum_cost_assignment(&sources, &targets);
        let (assignment, score) =
            DrillApp::symmetry_compass_assignment(&axis, &sources, &targets, &[shortest]).unwrap();

        assert_eq!(score.symmetry_conflicts, 0);
        assert!(score.symmetry_error < 0.001);
        assert_eq!(assignment.len(), sources.len());
    }

    #[test]
    fn symmetry_compass_resolves_collisions_without_breaking_symmetry() {
        let axis = SymmetryAxis {
            id: 1,
            name: "Center".into(),
            start: Point { x: 10.0, y: 0.0 },
            end: Point { x: 10.0, y: 20.0 },
            threshold_steps: 0.25,
            mode: SymmetryMode::Perfect,
        };
        let sources = [
            Point { x: 6.0, y: 0.0 },
            Point { x: 14.0, y: 0.0 },
            Point { x: 6.0, y: 10.0 },
            Point { x: 14.0, y: 10.0 },
        ];
        let targets = sources;
        let crossing = vec![2, 3, 0, 1];
        let before = DrillApp::assignment_collision_metrics(
            &sources,
            &targets,
            &crossing,
            CompassRoute::Straight,
        );
        let (assignment, score) =
            DrillApp::symmetry_compass_assignment(&axis, &sources, &targets, &[crossing]).unwrap();

        assert!(before.0 > 0);
        assert_eq!(score.symmetry_conflicts, 0);
        assert_eq!(score.collision_conflicts, 0);
        assert_eq!(assignment, vec![0, 1, 2, 3]);
    }

    #[test]
    fn try_symmetry_compass_uses_the_pairing_threshold() {
        let mut axis = SymmetryAxis {
            id: 1,
            name: "Center".into(),
            start: Point { x: 10.0, y: 0.0 },
            end: Point { x: 10.0, y: 20.0 },
            threshold_steps: 0.5,
            mode: SymmetryMode::Try,
        };
        let sources = [Point { x: 6.0, y: 10.0 }, Point { x: 14.0, y: 11.0 }];

        assert!(DrillApp::compass_symmetry_constraint(&axis, &sources).is_none());
        axis.threshold_steps = 1.1;
        let constraint = DrillApp::compass_symmetry_constraint(&axis, &sources).unwrap();
        assert_eq!(constraint.pairs, vec![(1, 0)]);
    }
}
