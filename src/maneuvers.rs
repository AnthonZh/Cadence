use super::*;
use model::{MoveTiming, SketchConstraint, SketchRule};

#[cfg(test)]
#[path = "maneuver_tests.rs"]
mod tests;

impl DrillApp {
    fn apply_move_timing(&mut self, clear: bool) {
        if self.selected.is_empty() {
            self.status = "Select performers".into();
            return;
        }
        let span = self.show.sets[self.current].counts_to_next as f32;
        let last_delay =
            self.move_delay + self.move_stagger * self.selected.len().saturating_sub(1) as f32;
        if !clear
            && (self.move_duration <= 0.0
                || last_delay + self.move_duration / self.move_pace > span + 0.001)
        {
            self.status = "Delay, stagger and movement must fit within Counts to next".into();
            return;
        }
        self.snapshot();
        for (rank, id) in self.selected.iter().enumerate() {
            if clear {
                self.show.sets[self.current].timing.remove(id);
            } else {
                self.show.sets[self.current].timing.insert(
                    *id,
                    MoveTiming {
                        delay: self.move_delay + rank as f32 * self.move_stagger,
                        move_counts: self.move_duration,
                        pace: self.move_pace,
                    },
                );
            }
        }
        self.status = if clear {
            "Reset transition timing"
        } else {
            "Applied transition timing (stagger in performer ID order)"
        }
        .into();
    }

    pub(super) fn rigid_maneuver(&mut self, rotate: bool) {
        let Some(pivot) = self.component_pivot() else {
            return;
        };
        let sources: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| self.editable_position(*id).map(|p| (*id, p)))
            .collect();
        if sources.is_empty() {
            self.status = "Select performers".into();
            return;
        }
        let start = self.active_subset().map_or(0.0, |s| s.offset_count);
        let end = self.show.sets[self.current]
            .subsets
            .iter()
            .filter(|s| s.offset_count > start + 0.001)
            .map(|s| s.offset_count)
            .min_by(f32::total_cmp)
            .unwrap_or(self.show.sets[self.current].counts_to_next as f32);
        let angle = if rotate { self.component_angle } else { 0.0 };
        let translation = Point {
            x: field::yards_to_steps(self.component_translation.x),
            y: field::yards_to_steps(self.component_translation.y),
        };
        if !angle.is_finite() || !translation.x.is_finite() || !translation.y.is_finite() {
            return;
        }
        self.snapshot();
        if self.current + 1 == self.show.sets.len() {
            self.show.add_set(self.current);
        }
        let samples = ((angle.abs() / 3.0).ceil() as usize).clamp(8, 720);
        let mut next = self.show.next_subset_id.max(
            self.show
                .sets
                .iter()
                .flat_map(|s| &s.subsets)
                .map(|s| s.id)
                .max()
                .unwrap_or(0)
                + 1,
        );
        for index in 1..=samples {
            let t = index as f32 / samples as f32;
            let positions: BTreeMap<_, _> = sources
                .iter()
                .map(|(id, p)| {
                    let mut point = Self::rotated_point(pivot, *p, angle * t);
                    point.x += translation.x * t;
                    point.y += translation.y * t;
                    (*id, point)
                })
                .collect();
            if index == samples {
                if let Some(subset) = self.show.sets[self.current]
                    .subsets
                    .iter_mut()
                    .find(|s| (s.offset_count - end).abs() < 0.001)
                {
                    subset.positions.extend(positions);
                } else {
                    self.show.sets[self.current + 1].positions.extend(positions);
                }
            } else {
                self.show.sets[self.current].subsets.push(Subset {
                    id: next,
                    name: if rotate { "Gate" } else { "Float" }.into(),
                    offset_count: start + (end - start) * t,
                    positions,
                });
                next += 1;
            }
        }
        self.show.next_subset_id = next;
        self.show.sets[self.current]
            .subsets
            .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
        self.tool = Tool::Select;
        self.status = "Transition created. Press Space to preview.".into();
    }

    fn add_sketch_rule(&mut self, rule: SketchRule) {
        let driven = |rule: &SketchRule| -> Vec<u32> {
            match rule {
                SketchRule::OnGuide { performers, .. } => performers.clone(),
                SketchRule::Fixed { performer, .. } => vec![*performer],
                SketchRule::Symmetry { pairs, .. } => {
                    pairs.iter().map(|(_, target)| *target).collect()
                }
                SketchRule::Concentric { .. } | SketchRule::FixedGuidePivot { .. } => vec![],
            }
        };
        let new_driven = driven(&rule);
        let guide_driver = |rule: &SketchRule| match rule {
            SketchRule::Concentric { guide, .. } | SketchRule::FixedGuidePivot { guide, .. } => {
                Some(*guide)
            }
            _ => None,
        };
        if self.show.sets[self.current].constraints.iter().filter(|c| c.subset == self.active_subset).any(|c| {
            driven(&c.rule).iter().any(|id| new_driven.contains(id))
                || guide_driver(&rule).is_some_and(|id| guide_driver(&c.rule) == Some(id))
                || matches!((&rule, &c.rule), (SketchRule::Concentric { guide: a, reference: b }, SketchRule::Concentric { guide: c, reference: d }) if a == c || a == d || b == c)
                || matches!((&rule, &c.rule), (SketchRule::Symmetry { pairs: a, .. }, SketchRule::Symmetry { pairs: b, .. }) if a.iter().any(|(s,t)| b.iter().any(|(u,v)| s == v || t == u)))
        }) {
            self.status = "Constraint conflict: remove the existing rule before assigning another driver".into(); return;
        }
        self.snapshot();
        self.show.sets[self.current]
            .constraints
            .push(SketchConstraint {
                subset: self.active_subset,
                rule,
            });
        self.enforce_sketch_constraints();
    }

    pub(super) fn enforce_sketch_constraints(&mut self) {
        if self.playing {
            return;
        }
        let mut constraints = self.show.sets[self.current].constraints.clone();
        constraints.sort_by_key(|c| match c.rule {
            SketchRule::FixedGuidePivot { .. } => 0,
            SketchRule::Concentric { .. } => 1,
            SketchRule::OnGuide { .. } | SketchRule::Fixed { .. } => 2,
            SketchRule::Symmetry { .. } => 3,
        });
        let mut changes = Vec::new();
        for constraint in constraints {
            if constraint.subset != self.active_subset {
                continue;
            }
            match constraint.rule {
                SketchRule::FixedGuidePivot { guide, point } => {
                    if let Some(mut target) = self.show.sets[self.current]
                        .guides
                        .iter()
                        .find(|g| g.id == guide)
                        .cloned()
                    {
                        let anchor = Self::guide_anchor(&target);
                        if anchor.distance(point) > 0.0001 {
                            Self::transform_guide(
                                &mut target,
                                Point {
                                    x: point.x - anchor.x,
                                    y: point.y - anchor.y,
                                },
                                1.0,
                                0.0,
                                false,
                                false,
                            );
                            if let Some(saved) = self.show.sets[self.current]
                                .guides
                                .iter_mut()
                                .find(|g| g.id == guide)
                            {
                                *saved = target;
                                self.dirty = true;
                            }
                        }
                    }
                }
                SketchRule::OnGuide { guide, performers } => {
                    if let Some(guide) = self.show.sets[self.current]
                        .guides
                        .iter()
                        .find(|g| g.id == guide)
                    {
                        let spots = Self::guide_spots(guide);
                        if spots.len() != performers.len() {
                            self.status = "Constraint conflict: guide spot count differs from bound performers".into();
                            continue;
                        }
                        changes.extend(performers.into_iter().zip(spots));
                    }
                }
                SketchRule::Fixed { performer, point } => changes.push((performer, point)),
                SketchRule::Symmetry { axis, pairs } => {
                    if let Some(axis) = self.show.sets[self.current]
                        .symmetry_axes
                        .iter()
                        .find(|a| a.id == axis)
                    {
                        for (source, target) in pairs {
                            if let Some(point) = self
                                .editable_position(source)
                                .and_then(|p| Self::reflect_across_axis(axis, p))
                            {
                                changes.push((target, point));
                            }
                        }
                    }
                }
                SketchRule::Concentric { guide, reference } => {
                    let center = self.show.sets[self.current]
                        .guides
                        .iter()
                        .find(|g| g.id == reference)
                        .and_then(|g| match g.kind {
                            GuideKind::Circle { center, .. } => Some(center),
                            _ => None,
                        });
                    if let Some(center) = center
                        && let Some(target) = self.show.sets[self.current]
                            .guides
                            .iter_mut()
                            .find(|g| g.id == guide)
                        && let GuideKind::Circle { center: target, .. } = &mut target.kind
                        && target.distance(center) > 0.0001
                    {
                        *target = center;
                        self.dirty = true;
                    }
                }
            }
            for (id, point) in changes.drain(..) {
                if self
                    .editable_position(id)
                    .is_some_and(|old| old.distance(point) > 0.0001)
                {
                    self.set_editable_position(id, point);
                    self.dirty = true;
                }
            }
        }
    }

    pub(super) fn maneuver_controls(&mut self, ui: &mut egui::Ui) {
        if self.component_page == 3 {
            egui::CollapsingHeader::new("Transition timing")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Current set → next set");
                    ui.horizontal(|ui| {
                        ui.label("Hold");
                        ui.add(
                            egui::DragValue::new(&mut self.move_delay)
                                .range(0.0..=1024.0)
                                .suffix(" counts"),
                        );
                        ui.label("Move");
                        ui.add(
                            egui::DragValue::new(&mut self.move_duration)
                                .range(0.125..=1024.0)
                                .suffix(" counts"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Stagger / performer");
                        ui.add(
                            egui::DragValue::new(&mut self.move_stagger)
                                .range(0.0..=128.0)
                                .suffix(" counts"),
                        );
                        egui::ComboBox::from_id_salt("move_pace")
                            .selected_text(format!("{}× pace", self.move_pace))
                            .show_ui(ui, |ui| {
                                for (pace, label) in
                                    [(0.5, "Half time"), (1.0, "Regular"), (2.0, "Double time")]
                                {
                                    ui.selectable_value(&mut self.move_pace, pace, label);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Apply timing").clicked() {
                            self.apply_move_timing(false);
                        }
                        if ui.button("Reset timing").clicked() {
                            self.apply_move_timing(true);
                        }
                        if ui.button("Hold entire transition").clicked() {
                            self.hold_transition();
                        }
                    });
                });
        }
        if self.component_page == 4 {
            egui::CollapsingHeader::new("Gate / Pinwheel / Float")
                .default_open(true)
                .show(ui, |ui| {
                    if ui.button("Gate / pinwheel → next keyframe").clicked() {
                        self.rigid_maneuver(true);
                    }
                    if ui.button("Float → next keyframe").clicked() {
                        self.rigid_maneuver(false);
                    }
                });
        }
        if self.component_page == 5 {
            egui::CollapsingHeader::new("Sketch constraints")
                .default_open(true)
                .show(ui, |ui| {
                    if ui.button("Fix guide pivot").clicked()
                        && let Some(guide) = self.selected_guide()
                    {
                        self.add_sketch_rule(SketchRule::FixedGuidePivot {
                            guide: guide.id,
                            point: Self::guide_anchor(&guide),
                        });
                    }
                    if ui.button("Bind dots to guide / equal intervals").clicked()
                        && let Some(guide) = self.selected_guide()
                    {
                        let ids: Vec<_> = self.selected.iter().copied().collect();
                        let points: Vec<_> = ids
                            .iter()
                            .filter_map(|id| self.editable_position(*id))
                            .collect();
                        let spots = Self::guide_spots(&guide);
                        if !ids.is_empty() && points.len() == spots.len() {
                            let assignment = Self::minimum_cost_assignment(&points, &spots);
                            let mut performers = vec![0; ids.len()];
                            for (index, id) in ids.into_iter().enumerate() {
                                performers[assignment[index]] = id;
                            }
                            self.add_sketch_rule(SketchRule::OnGuide {
                                guide: guide.id,
                                performers,
                            });
                        } else {
                            self.status = "Select the same number of dots as guide spots".into();
                        }
                    }
                    if ui.button("Fix selected dots / guide performers").clicked() {
                        let points: Vec<_> = self
                            .selected
                            .iter()
                            .filter_map(|id| self.editable_position(*id).map(|p| (*id, p)))
                            .collect();
                        if !points.is_empty() {
                            self.snapshot();
                            for (performer, point) in points {
                                // Fixed anchors supersede bindings only when explicitly removed by the user.
                                if self.show.sets[self.current].constraints.iter().any(|c| {
                                    c.subset == self.active_subset
                                        && match &c.rule {
                                            SketchRule::Fixed { performer: id, .. } => {
                                                *id == performer
                                            }
                                            SketchRule::OnGuide { performers, .. } => {
                                                performers.contains(&performer)
                                            }
                                            SketchRule::Symmetry { pairs, .. } => {
                                                pairs.iter().any(|(_, id)| *id == performer)
                                            }
                                            _ => false,
                                        }
                                }) {
                                    self.status =
                                        "Constraint conflict: dot already has a driver".into();
                                    continue;
                                }
                                self.show.sets[self.current]
                                    .constraints
                                    .push(SketchConstraint {
                                        subset: self.active_subset,
                                        rule: SketchRule::Fixed { performer, point },
                                    });
                            }
                        }
                    }
                    if ui.button("Mirror pairs about selected axis").clicked() {
                        let ids: Vec<_> = self.selected.iter().copied().collect();
                        if let Some(axis) = self.selected_symmetry_axis
                            && !ids.is_empty()
                            && ids.len().is_multiple_of(2)
                        {
                            self.add_sketch_rule(SketchRule::Symmetry {
                                axis,
                                pairs: ids.chunks_exact(2).map(|p| (p[0], p[1])).collect(),
                            });
                            self.status =
                                "Mirrored pairs in performer ID order (first drives second)".into();
                        } else {
                            self.status = "Select an axis and an even number of performers".into();
                        }
                    }
                    if let Some(selected) = self.selected_guide()
                        && matches!(selected.kind, GuideKind::Circle { .. })
                    {
                        let references: Vec<_> = self.show.sets[self.current]
                            .guides
                            .iter()
                            .filter(|g| {
                                g.id != selected.id && matches!(g.kind, GuideKind::Circle { .. })
                            })
                            .map(|g| g.id)
                            .collect();
                        for reference in references {
                            if ui
                                .button(format!("Concentric with circle {reference}"))
                                .clicked()
                            {
                                self.add_sketch_rule(SketchRule::Concentric {
                                    guide: selected.id,
                                    reference,
                                });
                            }
                        }
                    }
                    let mut remove = None;
                    for (index, c) in self.show.sets[self.current].constraints.iter().enumerate() {
                        let label = match &c.rule {
                            SketchRule::FixedGuidePivot { guide, .. } => {
                                format!("Fixed pivot: guide {guide}")
                            }
                            SketchRule::OnGuide { guide, .. } => {
                                format!("Guide {guide}: interval / alignment")
                            }
                            SketchRule::Fixed { performer, .. } => {
                                format!("Fixed performer {performer}")
                            }
                            SketchRule::Symmetry { axis, .. } => format!("Symmetry axis {axis}"),
                            SketchRule::Concentric { guide, reference } => {
                                format!("Concentric {guide} → {reference}")
                            }
                        };
                        ui.horizontal(|ui| {
                            ui.label(label);
                            if ui.small_button("Remove").clicked() {
                                remove = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove {
                        self.snapshot();
                        self.show.sets[self.current].constraints.remove(index);
                    }
                });
        }
    }

    fn hold_transition(&mut self) {
        if self.selected.is_empty() || self.current + 1 >= self.show.sets.len() {
            return;
        }
        let points: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| {
                self.show.sets[self.current]
                    .positions
                    .get(id)
                    .map(|p| (*id, *p))
            })
            .collect();
        self.snapshot();
        for (id, point) in points {
            self.show.sets[self.current].timing.remove(&id);
            for subset in &mut self.show.sets[self.current].subsets {
                subset.positions.remove(&id);
            }
            self.show.sets[self.current + 1].positions.insert(id, point);
        }
        self.status = "Selected performers hold throughout the transition".into();
    }
}
