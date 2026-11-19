use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::Path,
};

pub const FILE_EXTENSION: &str = "mdrill";
const FILE_HEADER: &[u8] = b"MARCHING_DRILL_DOCUMENT\n1\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Performer {
    pub id: u32,
    pub label: String,
    pub section: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Set {
    pub name: String,
    pub counts_to_next: u32,
    pub positions: BTreeMap<u32, Point>,
    #[serde(default)]
    pub guides: Vec<FormationGuide>,
    #[serde(default)]
    pub subsets: Vec<Subset>,
    #[serde(default)]
    pub symmetry_axes: Vec<SymmetryAxis>,
    #[serde(default)]
    pub timing: BTreeMap<u32, MoveTiming>,
    #[serde(default)]
    pub constraints: Vec<SketchConstraint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveTiming {
    pub delay: f32,
    pub move_counts: f32,
    pub pace: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SketchConstraint {
    pub subset: Option<u32>,
    pub rule: SketchRule,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SketchRule {
    FixedGuidePivot { guide: u32, point: Point },
    OnGuide { guide: u32, performers: Vec<u32> },
    Fixed { performer: u32, point: Point },
    Symmetry { axis: u32, pairs: Vec<(u32, u32)> },
    Concentric { guide: u32, reference: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subset {
    pub id: u32,
    pub name: String,
    pub offset_count: f32,
    pub positions: BTreeMap<u32, Point>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SymmetryMode {
    Perfect,
    Try,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymmetryAxis {
    pub id: u32,
    pub name: String,
    pub start: Point,
    pub end: Point,
    pub threshold_steps: f32,
    pub mode: SymmetryMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormationGuide {
    pub id: u32,
    pub spacing_steps: f32,
    #[serde(default = "default_spot_count")]
    pub spot_count: usize,
    #[serde(default = "default_fill_guide")]
    pub fill_guide: bool,
    #[serde(default)]
    pub reverse_spots: bool,
    pub kind: GuideKind,
}

fn default_spot_count() -> usize {
    8
}

fn default_fill_guide() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GuideKind {
    Line {
        start: Point,
        end: Point,
    },
    Circle {
        center: Point,
        radius: f32,
    },
    Spline {
        start: Point,
        control_1: Point,
        control_2: Point,
        end: Point,
    },
    Arc {
        center: Point,
        radius: f32,
        start_degrees: f32,
        sweep_degrees: f32,
    },
    ThreePointArc {
        start: Point,
        through: Point,
        end: Point,
    },
    Block {
        center: Point,
        width: f32,
        height: f32,
        rows: usize,
        columns: usize,
        stagger: bool,
        rotation_degrees: f32,
    },
    Polygon {
        center: Point,
        radius: f32,
        sides: usize,
        rotation_degrees: f32,
        filled: bool,
    },
    Spiral {
        center: Point,
        start_radius: f32,
        end_radius: f32,
        turns: f32,
        rotation_degrees: f32,
    },
    Polyline {
        points: Vec<Point>,
        closed: bool,
    },
    Fan {
        pivot: Point,
        radius: f32,
        start_degrees: f32,
        sweep_degrees: f32,
        rays: usize,
        spots_per_ray: usize,
    },
    Concentric {
        center: Point,
        start_radius: f32,
        ring_spacing: f32,
        rings: usize,
        spots_per_ring: usize,
    },
    Chevron {
        left: Point,
        pivot: Point,
        right: Point,
    },
    Text {
        origin: Point,
        text: String,
        cell_size: f32,
    },
    ImportedPath {
        points: Vec<Point>,
        closed: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Show {
    pub version: u32,
    pub name: String,
    pub performers: Vec<Performer>,
    pub sets: Vec<Set>,
    pub next_id: u32,
    #[serde(default = "default_next_subset_id")]
    pub next_subset_id: u32,
    #[serde(default)]
    pub music: MusicSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicSettings {
    pub path: Option<String>,
    pub bpm: f32,
    pub offset_seconds: f32,
}

impl Default for MusicSettings {
    fn default() -> Self {
        Self {
            path: None,
            bpm: 120.0,
            offset_seconds: 0.0,
        }
    }
}

fn default_next_subset_id() -> u32 {
    1
}

impl Default for Show {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Untitled drill".into(),
            performers: vec![],
            sets: vec![Set {
                name: "Set 1".into(),
                counts_to_next: 8,
                positions: BTreeMap::new(),
                guides: vec![],
                subsets: vec![],
                symmetry_axes: vec![],
                timing: BTreeMap::new(),
                constraints: vec![],
            }],
            next_id: 1,
            next_subset_id: 1,
            music: MusicSettings::default(),
        }
    }
}

impl Show {
    pub fn add_performer(&mut self, at: Point) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.performers.push(Performer {
            id,
            label: format!("P{id}"),
            section: "Band".into(),
        });
        for set in &mut self.sets {
            set.positions.insert(id, at);
        }
        id
    }
    pub fn add_set(&mut self, from: usize) -> usize {
        let positions = self.sets[from].positions.clone();
        let symmetry_axes = self.sets[from].symmetry_axes.clone();
        self.sets.insert(
            from + 1,
            Set {
                name: format!("Set {}", from + 2),
                counts_to_next: 8,
                positions,
                guides: vec![],
                subsets: vec![],
                symmetry_axes,
                timing: BTreeMap::new(),
                constraints: vec![],
            },
        );
        for (index, set) in self.sets.iter_mut().enumerate() {
            set.name = format!("Set {}", index + 1);
        }
        from + 1
    }
    pub fn assign_instrument(&mut self, selected: &BTreeSet<u32>, instrument: &str) {
        let prefix = instrument_prefix(instrument);
        let mut used: HashSet<String> = self
            .performers
            .iter()
            .filter(|performer| !selected.contains(&performer.id))
            .map(|performer| performer.label.to_ascii_uppercase())
            .collect();
        let mut next = 1;
        for performer in self
            .performers
            .iter_mut()
            .filter(|performer| selected.contains(&performer.id))
        {
            loop {
                let label = format!("{prefix}{next}");
                next += 1;
                if used.insert(label.clone()) {
                    performer.section = instrument.into();
                    performer.label = label;
                    break;
                }
            }
        }
    }
    pub fn next_guide_id(&self) -> u32 {
        self.sets
            .iter()
            .flat_map(|set| &set.guides)
            .map(|guide| guide.id)
            .max()
            .unwrap_or(0)
            + 1
    }
    pub fn total_counts(&self) -> u32 {
        self.sets
            .iter()
            .take(self.sets.len().saturating_sub(1))
            .map(|s| s.counts_to_next)
            .sum()
    }
    pub fn position_at(&self, id: u32, count: f32) -> Option<Point> {
        let mut start = 0.0;
        for pair in self.sets.windows(2) {
            let end = start + pair[0].counts_to_next as f32;
            if count <= end {
                let mut local_count = (count - start).clamp(0.0, pair[0].counts_to_next as f32);
                if let Some(timing) = pair[0].timing.get(&id) {
                    let fraction = ((local_count - timing.delay) * timing.pace
                        / timing.move_counts)
                        .clamp(0.0, 1.0);
                    local_count = fraction * pair[0].counts_to_next as f32;
                }
                let mut previous_count = 0.0;
                let mut previous = *pair[0].positions.get(&id)?;
                let mut subsets: Vec<&Subset> = pair[0]
                    .subsets
                    .iter()
                    .filter(|subset| subset.positions.contains_key(&id))
                    .collect();
                subsets.sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
                for subset in subsets {
                    let subset_point = *subset.positions.get(&id)?;
                    if local_count <= subset.offset_count {
                        let span = subset.offset_count - previous_count;
                        let t = if span <= f32::EPSILON {
                            1.0
                        } else {
                            (local_count - previous_count) / span
                        };
                        return Some(previous.lerp(subset_point, t.clamp(0.0, 1.0)));
                    }
                    previous_count = subset.offset_count;
                    previous = subset_point;
                }
                let next = *pair[1].positions.get(&id)?;
                let span = pair[0].counts_to_next as f32 - previous_count;
                let t = if span <= f32::EPSILON {
                    1.0
                } else {
                    (local_count - previous_count) / span
                };
                return Some(previous.lerp(next, t.clamp(0.0, 1.0)));
            }
            start = end;
        }
        self.sets.last()?.positions.get(&id).copied()
    }
    pub fn set_start(&self, index: usize) -> f32 {
        self.sets
            .iter()
            .take(index)
            .map(|s| s.counts_to_next as f32)
            .sum()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.sets.is_empty() {
            return Err("Unsupported or empty show".into());
        }
        if !self.music.bpm.is_finite()
            || self.music.bpm <= 0.0
            || !self.music.offset_seconds.is_finite()
            || self.music.offset_seconds < 0.0
        {
            return Err("Show has invalid music timing".into());
        }
        if self.sets.iter().any(|s| {
            s.counts_to_next == 0
                || s.timing.values().any(|t| {
                    !t.delay.is_finite()
                        || t.delay < 0.0
                        || !t.move_counts.is_finite()
                        || t.move_counts <= 0.0
                        || !t.pace.is_finite()
                        || t.pace <= 0.0
                        || t.delay + t.move_counts / t.pace > s.counts_to_next as f32 + 0.001
                })
                || s.subsets.iter().any(|subset| {
                    !subset.offset_count.is_finite()
                        || subset.offset_count <= 0.0
                        || subset.offset_count >= s.counts_to_next as f32
                })
                || s.symmetry_axes.iter().any(|axis| {
                    !axis.start.x.is_finite()
                        || !axis.start.y.is_finite()
                        || !axis.end.x.is_finite()
                        || !axis.end.y.is_finite()
                        || !axis.threshold_steps.is_finite()
                        || axis.threshold_steps < 0.0
                        || axis.start.distance(axis.end) <= f32::EPSILON
                })
                || self
                    .performers
                    .iter()
                    .any(|p| !s.positions.contains_key(&p.id))
        }) {
            return Err("Show has missing positions or zero-count transitions".into());
        }
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let payload = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut bytes = Vec::with_capacity(FILE_HEADER.len() + payload.len());
        bytes.extend_from_slice(FILE_HEADER);
        bytes.extend_from_slice(&payload);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        fs::rename(tmp, path).map_err(|e| e.to_string())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let payload = bytes.strip_prefix(FILE_HEADER).unwrap_or(&bytes);
        let show: Self = serde_json::from_slice(payload).map_err(|e| e.to_string())?;
        show.validate()?;
        Ok(show)
    }
}

fn instrument_prefix(instrument: &str) -> String {
    match instrument.trim().to_ascii_lowercase().as_str() {
        "flute" | "piccolo" => "FL".into(),
        "clarinet" | "bass clarinet" => "CL".into(),
        "alto saxophone" => "AS".into(),
        "tenor saxophone" => "TS".into(),
        "baritone saxophone" => "BS".into(),
        "trumpet" => "T".into(),
        "mellophone" | "french horn" => "M".into(),
        "trombone" => "TB".into(),
        "baritone" | "euphonium" => "B".into(),
        "tuba" | "sousaphone" => "S".into(),
        "snare drum" => "SD".into(),
        "tenor drums" => "Q".into(),
        "bass drum" => "BD".into(),
        "cymbals" => "CY".into(),
        "front ensemble" => "FE".into(),
        "color guard" => "CG".into(),
        "drum major" => "DM".into(),
        _ => {
            let prefix: String = instrument
                .split_whitespace()
                .filter_map(|word| word.chars().next())
                .take(3)
                .flat_map(char::to_uppercase)
                .collect();
            if prefix.is_empty() {
                "P".into()
            } else {
                prefix
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolates_and_keeps_ids_across_sets() {
        let mut show = Show::default();
        let id = show.add_performer(Point { x: 0.0, y: 0.0 });
        show.add_set(0);
        show.sets[1].positions.insert(id, Point { x: 8.0, y: 0.0 });
        assert_eq!(show.position_at(id, 4.0), Some(Point { x: 4.0, y: 0.0 }));
        assert_eq!(show.position_at(id, 8.0), Some(Point { x: 8.0, y: 0.0 }));
    }

    #[test]
    fn interpolates_through_partial_subset_keyframes() {
        let mut show = Show::default();
        let first = show.add_performer(Point { x: 0.0, y: 0.0 });
        let second = show.add_performer(Point { x: 0.0, y: 10.0 });
        show.add_set(0);
        show.sets[1]
            .positions
            .insert(first, Point { x: 8.0, y: 0.0 });
        show.sets[1]
            .positions
            .insert(second, Point { x: 8.0, y: 10.0 });
        show.sets[0].subsets.push(Subset {
            id: 1,
            name: "Subset 1".into(),
            offset_count: 4.0,
            positions: BTreeMap::from([(first, Point { x: 2.0, y: 4.0 })]),
        });

        assert_eq!(show.position_at(first, 4.0), Some(Point { x: 2.0, y: 4.0 }));
        assert_eq!(show.position_at(first, 6.0), Some(Point { x: 5.0, y: 2.0 }));
        assert_eq!(
            show.position_at(second, 4.0),
            Some(Point { x: 4.0, y: 10.0 })
        );
    }

    #[test]
    fn interpolates_through_multiple_fractional_subsets() {
        let mut show = Show::default();
        let id = show.add_performer(Point { x: 0.0, y: 0.0 });
        show.add_set(0);
        show.sets[1].positions.insert(id, Point { x: 8.0, y: 0.0 });
        show.sets[0].subsets = vec![
            Subset {
                id: 1,
                name: "A".into(),
                offset_count: 2.5,
                positions: BTreeMap::from([(id, Point { x: 1.0, y: 3.0 })]),
            },
            Subset {
                id: 2,
                name: "B".into(),
                offset_count: 6.25,
                positions: BTreeMap::from([(id, Point { x: 7.0, y: 3.0 })]),
            },
        ];

        assert_eq!(show.position_at(id, 2.5), Some(Point { x: 1.0, y: 3.0 }));
        assert_eq!(show.position_at(id, 6.25), Some(Point { x: 7.0, y: 3.0 }));
        assert_eq!(show.position_at(id, 7.125), Some(Point { x: 7.5, y: 1.5 }));
    }

    #[test]
    fn save_and_reopen_preserves_fractional_subsets() {
        let mut show = Show::default();
        let id = show.add_performer(Point { x: 0.0, y: 0.0 });
        show.add_set(0);
        show.sets[0].subsets.push(Subset {
            id: 7,
            name: "2-and".into(),
            offset_count: 2.5,
            positions: BTreeMap::from([(id, Point { x: 3.0, y: 4.0 })]),
        });
        show.next_subset_id = 8;
        let path =
            std::env::temp_dir().join(format!("cadence-subset-test-{}.json", std::process::id()));
        show.save(&path).unwrap();
        let loaded = Show::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(loaded.next_subset_id, 8);
        assert_eq!(loaded.sets[0].subsets[0].name, "2-and");
        assert_eq!(loaded.position_at(id, 2.5), Some(Point { x: 3.0, y: 4.0 }));
    }

    #[test]
    fn save_and_reopen_preserves_positions() {
        let mut show = Show::default();
        let id = show.add_performer(Point { x: 18.0, y: 30.0 });
        show.add_set(0);
        show.sets[1]
            .positions
            .insert(id, Point { x: 26.0, y: 30.0 });
        let path = std::env::temp_dir().join(format!("cadence-test-{}.json", std::process::id()));
        show.save(&path).unwrap();
        let loaded = Show::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            loaded.position_at(id, 8.0),
            Some(Point { x: 26.0, y: 30.0 })
        );
    }

    #[test]
    fn project_files_have_a_versioned_document_header() {
        let show = Show::default();
        let path =
            std::env::temp_dir().join(format!("cadence-format-test-{}.mdrill", std::process::id()));
        show.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert!(bytes.starts_with(FILE_HEADER));
    }

    #[test]
    fn assigning_an_instrument_labels_the_whole_selection_uniquely() {
        let mut show = Show::default();
        let first = show.add_performer(Point { x: 0.0, y: 0.0 });
        let second = show.add_performer(Point { x: 1.0, y: 0.0 });
        let existing = show.add_performer(Point { x: 2.0, y: 0.0 });
        show.performers[2].label = "T1".into();
        show.performers[2].section = "Trumpet".into();
        show.assign_instrument(&BTreeSet::from([first, second]), "Trumpet");

        assert_eq!(show.performers[0].section, "Trumpet");
        assert_eq!(show.performers[0].label, "T2");
        assert_eq!(show.performers[1].label, "T3");
        assert_eq!(show.performers[2].id, existing);
        assert_eq!(show.performers[2].label, "T1");
    }

    #[test]
    fn old_projects_without_guides_still_open() {
        let json = r#"{
            "version": 1,
            "name": "Legacy",
            "performers": [],
            "sets": [{"name":"Set 1","counts_to_next":8,"positions":{}}],
            "next_id": 1
        }"#;
        let show: Show = serde_json::from_str(json).unwrap();
        assert!(show.sets[0].guides.is_empty());
        assert!(show.sets[0].subsets.is_empty());
        assert!(show.sets[0].symmetry_axes.is_empty());
        assert_eq!(show.next_subset_id, 1);
        assert_eq!(show.music, MusicSettings::default());
    }

    #[test]
    fn save_and_reopen_preserves_music_timing() {
        let show = Show {
            music: MusicSettings {
                path: Some("music/show.wav".into()),
                bpm: 168.0,
                offset_seconds: 2.75,
            },
            ..Show::default()
        };
        let path =
            std::env::temp_dir().join(format!("cadence-music-test-{}.mdrill", std::process::id()));
        show.save(&path).unwrap();
        let loaded = Show::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(loaded.music, show.music);
    }

    #[test]
    fn old_guides_receive_a_default_spot_count() {
        let json = r#"{
            "id": 3,
            "spacing_steps": 2.0,
            "kind": {"Line":{"start":{"x":0.0,"y":0.0},"end":{"x":8.0,"y":0.0}}}
        }"#;
        let guide: FormationGuide = serde_json::from_str(json).unwrap();
        assert_eq!(guide.spot_count, 8);
        assert!(guide.fill_guide);
        assert!(!guide.reverse_spots);
    }
}
