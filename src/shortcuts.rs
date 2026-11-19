use eframe::egui::{Event, Key, Modifiers};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Action {
    NewDrill,
    OpenDrill,
    SaveDrill,
    ExportChart,
    Undo,
    CopyComponents,
    PasteComponents,
    ComponentTools,
    Redo,
    DeleteSelected,
    SelectAll,
    ClearSelection,
    SelectTool,
    PlaceTool,
    PanTool,
    LineGuideTool,
    CircleGuideTool,
    SplineGuideTool,
    ProjectToGuide,
    DistributeOnGuide,
    AddSymmetryAxis,
    ToggleGridSnap,
    OpenShapePalette,
    AddSubset,
    DeleteSubset,
    ZoomToFit,
    ToggleGrid,
    ToggleLabels,
    TogglePreview,
    ToggleInspector,
    AddSet,
    PreviousSet,
    NextSet,
    PlayPause,
    Rewind,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    NudgeLeftLarge,
    NudgeRightLarge,
    NudgeUpLarge,
    NudgeDownLarge,
}

impl Action {
    pub const ALL: [Self; 43] = [
        Self::CopyComponents,
        Self::PasteComponents,
        Self::ComponentTools,
        Self::NewDrill,
        Self::OpenDrill,
        Self::SaveDrill,
        Self::ExportChart,
        Self::Undo,
        Self::Redo,
        Self::DeleteSelected,
        Self::SelectAll,
        Self::ClearSelection,
        Self::SelectTool,
        Self::PlaceTool,
        Self::PanTool,
        Self::LineGuideTool,
        Self::CircleGuideTool,
        Self::SplineGuideTool,
        Self::ProjectToGuide,
        Self::DistributeOnGuide,
        Self::AddSymmetryAxis,
        Self::ToggleGridSnap,
        Self::OpenShapePalette,
        Self::AddSubset,
        Self::DeleteSubset,
        Self::ZoomToFit,
        Self::ToggleGrid,
        Self::ToggleLabels,
        Self::TogglePreview,
        Self::ToggleInspector,
        Self::AddSet,
        Self::PreviousSet,
        Self::NextSet,
        Self::PlayPause,
        Self::Rewind,
        Self::NudgeLeft,
        Self::NudgeRight,
        Self::NudgeUp,
        Self::NudgeDown,
        Self::NudgeLeftLarge,
        Self::NudgeRightLarge,
        Self::NudgeUpLarge,
        Self::NudgeDownLarge,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::CopyComponents => "Copy components",
            Self::PasteComponents => "Paste components",
            Self::ComponentTools => "Transform / rotation drill",
            Self::NewDrill => "New drill",
            Self::OpenDrill => "Open drill",
            Self::SaveDrill => "Save drill",
            Self::ExportChart => "Export current chart",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::DeleteSelected => "Delete selected",
            Self::SelectAll => "Select all performers",
            Self::ClearSelection => "Clear selection",
            Self::SelectTool => "Select tool",
            Self::PlaceTool => "Place performer tool",
            Self::PanTool => "Pan tool",
            Self::LineGuideTool => "Draw line guide",
            Self::CircleGuideTool => "Draw circle guide",
            Self::SplineGuideTool => "Draw spline guide",
            Self::ProjectToGuide => "Project selected to guide",
            Self::DistributeOnGuide => "Compass-match selected to guide",
            Self::AddSymmetryAxis => "Add symmetry axis",
            Self::ToggleGridSnap => "Toggle step-grid snapping",
            Self::OpenShapePalette => "Open formation shape palette",
            Self::AddSubset => "Add subset at current beat",
            Self::DeleteSubset => "Delete active subset",
            Self::ZoomToFit => "Zoom to fit",
            Self::ToggleGrid => "Toggle grid",
            Self::ToggleLabels => "Toggle labels",
            Self::TogglePreview => "Toggle preview palette",
            Self::ToggleInspector => "Toggle inspector palette",
            Self::AddSet => "Add set",
            Self::PreviousSet => "Previous set",
            Self::NextSet => "Next set",
            Self::PlayPause => "Play / pause",
            Self::Rewind => "Rewind to count zero",
            Self::NudgeLeft => "Nudge left 1 step",
            Self::NudgeRight => "Nudge right 1 step",
            Self::NudgeUp => "Nudge up 1 step",
            Self::NudgeDown => "Nudge down 1 step",
            Self::NudgeLeftLarge => "Nudge left 4 steps",
            Self::NudgeRightLarge => "Nudge right 4 steps",
            Self::NudgeUpLarge => "Nudge up 4 steps",
            Self::NudgeDownLarge => "Nudge down 4 steps",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shortcut {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: String,
}

impl Shortcut {
    fn new(key: &str) -> Self {
        Self {
            command: false,
            shift: false,
            alt: false,
            key: key.into(),
        }
    }

    fn command(key: &str) -> Self {
        Self {
            command: true,
            ..Self::new(key)
        }
    }

    fn shift(key: &str) -> Self {
        Self {
            shift: true,
            ..Self::new(key)
        }
    }

    fn command_shift(key: &str) -> Self {
        Self {
            command: true,
            shift: true,
            ..Self::new(key)
        }
    }

    pub fn from_key(key: Key, modifiers: Modifiers) -> Self {
        Self {
            command: modifiers.command,
            shift: modifiers.shift,
            alt: modifiers.alt,
            key: format!("{key:?}"),
        }
    }

    pub fn unassigned() -> Self {
        Self::new("")
    }

    pub fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        !self.key.is_empty()
            && self.key == format!("{key:?}")
            && self.command == modifiers.command
            && self.shift == modifiers.shift
            && self.alt == modifiers.alt
    }

    pub fn display(&self) -> String {
        if self.key.is_empty() {
            return "Unassigned".into();
        }
        let mut parts = Vec::with_capacity(4);
        if self.command {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(display_key(&self.key));
        parts.join("+")
    }
}

fn display_key(key: &str) -> &str {
    match key {
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "PageUp" => "Page Up",
        "PageDown" => "Page Down",
        "Num0" => "0",
        "Num1" => "1",
        "Num2" => "2",
        "Num3" => "3",
        "Num4" => "4",
        "Num5" => "5",
        "Num6" => "6",
        "Num7" => "7",
        "Num8" => "8",
        "Num9" => "9",
        other => other,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub bindings: BTreeMap<Action, Shortcut>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        use Action::*;
        let mut bindings = BTreeMap::new();
        let defaults = [
            (CopyComponents, Shortcut::command("C")),
            (PasteComponents, Shortcut::command("V")),
            (ComponentTools, Shortcut::command("T")),
            (NewDrill, Shortcut::command("N")),
            (OpenDrill, Shortcut::command("O")),
            (SaveDrill, Shortcut::command("S")),
            (ExportChart, Shortcut::command("E")),
            (Undo, Shortcut::command("Z")),
            (Redo, Shortcut::command("Y")),
            (DeleteSelected, Shortcut::new("Delete")),
            (SelectAll, Shortcut::command("A")),
            (ClearSelection, Shortcut::new("Escape")),
            (SelectTool, Shortcut::new("V")),
            (PlaceTool, Shortcut::new("P")),
            (PanTool, Shortcut::new("H")),
            (LineGuideTool, Shortcut::new("I")),
            (CircleGuideTool, Shortcut::new("C")),
            (SplineGuideTool, Shortcut::new("B")),
            (ProjectToGuide, Shortcut::new("J")),
            (DistributeOnGuide, Shortcut::shift("J")),
            (AddSymmetryAxis, Shortcut::new("S")),
            (ToggleGridSnap, Shortcut::new("K")),
            (OpenShapePalette, Shortcut::new("F4")),
            (AddSubset, Shortcut::command_shift("Enter")),
            (DeleteSubset, Shortcut::shift("Delete")),
            (ZoomToFit, Shortcut::new("F")),
            (ToggleGrid, Shortcut::new("G")),
            (ToggleLabels, Shortcut::new("L")),
            (TogglePreview, Shortcut::new("Num1")),
            (ToggleInspector, Shortcut::new("Num2")),
            (AddSet, Shortcut::command("Enter")),
            (PreviousSet, Shortcut::new("PageUp")),
            (NextSet, Shortcut::new("PageDown")),
            (PlayPause, Shortcut::new("Space")),
            (Rewind, Shortcut::new("Home")),
            (NudgeLeft, Shortcut::new("ArrowLeft")),
            (NudgeRight, Shortcut::new("ArrowRight")),
            (NudgeUp, Shortcut::new("ArrowUp")),
            (NudgeDown, Shortcut::new("ArrowDown")),
            (NudgeLeftLarge, Shortcut::shift("ArrowLeft")),
            (NudgeRightLarge, Shortcut::shift("ArrowRight")),
            (NudgeUpLarge, Shortcut::shift("ArrowUp")),
            (NudgeDownLarge, Shortcut::shift("ArrowDown")),
        ];
        bindings.extend(defaults);
        Self { bindings }
    }
}

impl ShortcutConfig {
    pub fn load(path: &Path) -> Self {
        let mut merged = Self::default();
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(saved) = serde_json::from_str::<Self>(&text)
        {
            for (action, shortcut) in saved.bindings {
                merged.bindings.insert(action, shortcut);
            }
        }
        merged
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        fs::write(path, text).map_err(|error| error.to_string())
    }

    pub fn display(&self, action: Action) -> String {
        self.bindings
            .get(&action)
            .map_or_else(|| "Unassigned".into(), Shortcut::display)
    }

    pub fn action_for_event(&self, event: &Event) -> Option<Action> {
        let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            return None;
        };
        Action::ALL.into_iter().find(|action| {
            self.bindings
                .get(action)
                .is_some_and(|shortcut| shortcut.matches(*key, *modifiers))
        })
    }

    pub fn conflict(&self, action: Action, shortcut: &Shortcut) -> Option<Action> {
        if shortcut.key.is_empty() {
            return None;
        }
        self.bindings
            .iter()
            .find_map(|(other, bound)| (*other != action && bound == shortcut).then_some(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_no_conflicts() {
        let config = ShortcutConfig::default();
        for action in Action::ALL {
            let shortcut = config.bindings.get(&action).unwrap();
            assert_eq!(config.conflict(action, shortcut), None);
        }
    }

    #[test]
    fn shortcut_json_round_trip() {
        let config = ShortcutConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let reopened: ShortcutConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(reopened.bindings, config.bindings);
    }
}
