use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_follow_requires_no_guide_and_preserves_spacing() {
        let mut app = DrillApp::default();
        let leader = app.show.add_performer(Point { x: 20.0, y: 20.0 });
        let follower = app.show.add_performer(Point { x: 16.0, y: 20.0 });
        app.selected.extend([leader, follower]);
        app.follow_order = vec![leader, follower];
        app.follow_travel_steps = 8.0;
        app.follow_the_leader();
        assert!(app.show.sets[0].guides.is_empty());
        assert_eq!(
            app.show.sets[1].positions[&leader],
            Point { x: 28.0, y: 20.0 }
        );
        assert_eq!(
            app.show.sets[1].positions[&follower],
            Point { x: 24.0, y: 20.0 }
        );
        assert_eq!(
            app.show.position_at(follower, 4.0),
            Some(Point { x: 20.0, y: 20.0 })
        );
    }

    #[test]
    fn drawn_follow_route_works_without_a_formation_guide() {
        let mut app = DrillApp::default();
        let leader = app.show.add_performer(Point { x: 20.0, y: 20.0 });
        let follower = app.show.add_performer(Point { x: 16.0, y: 20.0 });
        app.selected.extend([leader, follower]);
        app.follow_order = vec![leader, follower];
        app.follow_route_mode = 1;
        app.follow_travel_steps = 8.0;
        app.motion_route = vec![
            Point { x: 20.0, y: 20.0 },
            Point { x: 24.0, y: 20.0 },
            Point { x: 24.0, y: 24.0 },
        ];
        app.follow_the_leader();
        assert!(app.show.sets[0].guides.is_empty());
        assert!(app.show.sets[1].positions[&leader].distance(Point { x: 24.0, y: 24.0 }) < 0.01);
        assert!(app.show.sets[1].positions[&follower].distance(Point { x: 24.0, y: 20.0 }) < 0.01);
    }

    #[test]
    fn rotation_works_on_arbitrary_dots_without_a_guide() {
        let mut app = DrillApp::default();
        let a = app.show.add_performer(Point { x: 20.0, y: 20.0 });
        let b = app.show.add_performer(Point { x: 24.0, y: 20.0 });
        app.selected.extend([a, b]);
        app.rotation_drill();
        assert!(app.show.sets[0].guides.is_empty());
        assert!(app.show.sets[1].positions[&a].distance(Point { x: 22.0, y: 18.0 }) < 0.01);
        assert!(app.show.sets[1].positions[&b].distance(Point { x: 22.0, y: 22.0 }) < 0.01);
    }

    #[test]
    fn path_rotation_accepts_an_independent_drawn_route() {
        let mut app = DrillApp::default();
        let id = app.show.add_performer(Point { x: 20.0, y: 20.0 });
        app.selected.insert(id);
        app.rotation_use_path = true;
        app.rotation_fraction = 0.5;
        app.motion_route = vec![Point { x: 20.0, y: 20.0 }, Point { x: 28.0, y: 20.0 }];
        app.rotation_drill();
        assert!(app.show.sets[0].guides.is_empty());
        assert_eq!(app.show.sets[1].positions[&id], Point { x: 24.0, y: 20.0 });
    }

    fn follow_fixture() -> (DrillApp, u32, u32) {
        let mut app = DrillApp::default();
        let leader = app.show.add_performer(Point { x: 20.0, y: 20.0 });
        let follower = app.show.add_performer(Point { x: 16.0, y: 20.0 });
        app.selected.extend([leader, follower]);
        app.follow_order = vec![leader, follower];
        app.follow_route_mode = 2;
        app.follow_travel_steps = 8.0;
        app.show.sets[0].guides.push(FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 8,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Polyline {
                points: vec![
                    Point { x: 20.0, y: 20.0 },
                    Point { x: 24.0, y: 20.0 },
                    Point { x: 24.0, y: 30.0 },
                ],
                closed: false,
            },
        });
        app.selected_guide = Some(1);
        (app, leader, follower)
    }

    #[test]
    fn followers_take_leaders_trail_around_corners_and_undo() {
        let (mut app, leader, follower) = follow_fixture();
        app.follow_the_leader();
        assert!(
            app.show
                .position_at(leader, 4.0)
                .unwrap()
                .distance(Point { x: 24.0, y: 20.0 })
                < 0.02
        );
        assert!(
            app.show
                .position_at(follower, 4.0)
                .unwrap()
                .distance(Point { x: 20.0, y: 20.0 })
                < 0.02
        );
        assert!(app.show.sets[1].positions[&leader].distance(Point { x: 24.0, y: 24.0 }) < 0.02);
        assert!(app.show.sets[1].positions[&follower].distance(Point { x: 24.0, y: 20.0 }) < 0.02);
        app.undo();
        assert_eq!(app.show.sets.len(), 1);
        assert!(app.show.sets[0].subsets.is_empty());
    }

    #[test]
    fn follow_stops_at_next_subset_and_preserves_other_dots() {
        let (mut app, leader, _) = follow_fixture();
        let other = app.show.add_performer(Point { x: 60.0, y: 60.0 });
        app.show.add_set(0);
        app.show.sets[0].subsets.push(Subset {
            id: 1,
            name: "Target".into(),
            offset_count: 4.0,
            positions: BTreeMap::from([(other, Point { x: 62.0, y: 60.0 })]),
        });
        let end = app.show.sets[1].positions.clone();
        app.follow_the_leader();
        let target = app.show.sets[0].subsets.iter().find(|s| s.id == 1).unwrap();
        assert!(target.positions[&leader].distance(Point { x: 24.0, y: 24.0 }) < 0.02);
        assert_eq!(target.positions[&other], Point { x: 62.0, y: 60.0 });
        assert_eq!(app.show.sets[1].positions, end);
    }

    #[test]
    fn follow_rejects_endpoint_overrun_without_mutation() {
        let (mut app, _, _) = follow_fixture();
        app.follow_travel_steps = 100.0;
        app.follow_the_leader();
        assert!(app.undo.is_empty());
        assert_eq!(app.show.sets.len(), 1);
    }

    #[test]
    fn selection_transforms_about_custom_pivot_and_undo_restores() {
        let mut app = DrillApp::default();
        let id = app.show.add_performer(Point { x: 12.0, y: 10.0 });
        app.selected.insert(id);
        app.component_pivot_override = Some(Point { x: 10.0, y: 10.0 });
        app.component_angle = 90.0;
        app.component_scale = 2.0;
        app.transform_components();
        let point = app.editable_position(id).unwrap();
        assert!((point.x - 10.0).abs() < 0.001);
        assert!((point.y - 14.0).abs() < 0.001);
        app.undo();
        assert_eq!(app.editable_position(id), Some(Point { x: 12.0, y: 10.0 }));
    }

    #[test]
    fn copied_dots_keep_instruments_and_receive_unique_identity() {
        let mut app = DrillApp::default();
        let id = app.show.add_performer(Point { x: 12.0, y: 10.0 });
        app.show.performers[0].section = "Trumpet".into();
        app.selected.insert(id);
        app.copy_components();
        app.paste_components();
        assert_eq!(app.show.performers.len(), 2);
        assert_ne!(app.show.performers[0].id, app.show.performers[1].id);
        assert_ne!(app.show.performers[0].label, app.show.performers[1].label);
        assert_eq!(app.show.performers[1].section, "Trumpet");
    }

    #[test]
    fn full_circle_rotation_animates_and_returns_to_source() {
        let mut app = DrillApp::default();
        let id = app.show.add_performer(Point { x: 50.0, y: 30.0 });
        app.selected.insert(id);
        app.show.sets[0].guides.push(FormationGuide {
            id: 1,
            spacing_steps: 2.0,
            spot_count: 8,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Circle {
                center: Point { x: 50.0, y: 40.0 },
                radius: 10.0,
            },
        });
        app.selected_guide = Some(1);
        app.rotation_fraction = 1.0;
        app.rotation_use_path = true;
        app.rotation_drill();
        assert_eq!(app.show.sets.len(), 2);
        assert!(
            app.show
                .position_at(id, 4.0)
                .unwrap()
                .distance(Point { x: 50.0, y: 50.0 })
                < 0.05
        );
        assert!(app.show.sets[1].positions[&id].distance(Point { x: 50.0, y: 30.0 }) < 0.05);
    }
}

#[derive(Clone)]
pub struct ComponentClipboard {
    performers: Vec<(model::Performer, Point)>,
    guide: Option<FormationGuide>,
}

impl DrillApp {
    pub(super) fn copy_components(&mut self) {
        let performers: Vec<_> = self
            .show
            .performers
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .filter_map(|p| self.editable_position(p.id).map(|point| (p.clone(), point)))
            .collect();
        let guide = self.selected_guide();
        if performers.is_empty() && guide.is_none() {
            self.status = "Select dots or geometry to copy".into();
            return;
        }
        self.component_clipboard = Some(ComponentClipboard { performers, guide });
        self.status = "Copied selection".into();
    }

    pub(super) fn paste_components(&mut self) {
        let Some(clipboard) = self.component_clipboard.clone() else {
            return;
        };
        self.snapshot();
        self.selected.clear();
        self.selected_guide = None;
        let offset = Point { x: 2.0, y: 2.0 };
        for (source, point) in clipboard.performers {
            let point = Point {
                x: point.x + offset.x,
                y: point.y + offset.y,
            };
            let id = self.show.add_performer(point);
            let mut label = format!("{} copy", source.label);
            let mut suffix = 2;
            while self.show.performers.iter().any(|p| p.label == label) {
                label = format!("{} copy {suffix}", source.label);
                suffix += 1;
            }
            let performer = self
                .show
                .performers
                .iter_mut()
                .find(|p| p.id == id)
                .unwrap();
            performer.label = label;
            performer.section = source.section;
            self.set_editable_position(id, point);
            self.selected.insert(id);
        }
        if let Some(mut guide) = clipboard.guide {
            guide.id = self.show.next_guide_id();
            Self::transform_guide(&mut guide, offset, 1.0, 0.0, false, false);
            self.selected_guide = Some(guide.id);
            self.show.sets[self.current].guides.push(guide);
        }
        self.status = "Pasted selection, offset 2 steps".into();
    }

    pub(super) fn component_pivot(&self) -> Option<Point> {
        if let Some(pivot) = self.component_pivot_override {
            return Some(pivot);
        }
        if let Some(guide) = self.selected_guide() {
            return Some(Self::guide_anchor(&guide));
        }
        let points: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| self.editable_position(*id))
            .collect();
        if points.is_empty() {
            return None;
        }
        let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        Some(Point {
            x: (min_x + max_x) * 0.5,
            y: (min_y + max_y) * 0.5,
        })
    }

    fn transform_components(&mut self) {
        let Some(pivot) = self.component_pivot() else {
            return;
        };
        let scale = self.component_scale;
        let angle = self.component_angle;
        if !scale.is_finite() || scale <= 0.0 || !angle.is_finite() {
            return;
        }
        let positions: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| {
                let point = self.editable_position(*id)?;
                let point = Point {
                    x: pivot.x + (point.x - pivot.x) * scale,
                    y: pivot.y + (point.y - pivot.y) * scale,
                };
                let mut point = Self::rotated_point(pivot, point, angle);
                point.x += field::yards_to_steps(self.component_translation.x);
                point.y += field::yards_to_steps(self.component_translation.y);
                Some((*id, point))
            })
            .collect();
        self.snapshot();
        for (id, point) in positions {
            self.set_editable_position(id, point);
        }
        if let Some(mut guide) = self.selected_guide() {
            let anchor = Self::guide_anchor(&guide);
            let target = Self::rotated_point(
                pivot,
                Point {
                    x: pivot.x + (anchor.x - pivot.x) * scale,
                    y: pivot.y + (anchor.y - pivot.y) * scale,
                },
                angle,
            );
            let translation = Point {
                x: target.x - anchor.x + field::yards_to_steps(self.component_translation.x),
                y: target.y - anchor.y + field::yards_to_steps(self.component_translation.y),
            };
            Self::transform_guide(&mut guide, translation, scale, angle, false, false);
            if let Some(saved) = self.show.sets[self.current]
                .guides
                .iter_mut()
                .find(|g| g.id == guide.id)
            {
                *saved = guide;
            }
        }
        self.status = "Transformed selection".into();
    }

    fn rotation_drill(&mut self) {
        if !self.rotation_use_path {
            let angle = self.component_angle;
            let translation = self.component_translation;
            self.component_angle = self.rotation_degrees;
            self.component_translation = Point::default();
            self.rigid_maneuver(true);
            self.component_angle = angle;
            self.component_translation = translation;
            return;
        }
        let Some(path) = self.optional_motion_path() else {
            self.status = "Draw a route or select a continuous guide".into();
            return;
        };
        if self.selected.is_empty() {
            self.status = "Select performers".into();
            return;
        }
        let path = &path;
        let length: f32 = path.windows(2).map(|s| s[0].distance(s[1])).sum();
        if length < 0.001 || !self.rotation_fraction.is_finite() {
            return;
        }
        let closed = path.first().unwrap().distance(*path.last().unwrap()) < 0.01;
        let sources: Vec<_> = self
            .selected
            .iter()
            .filter_map(|id| {
                let point = self.editable_position(*id)?;
                let mut nearest = (f32::INFINITY, 0.0);
                let mut along = 0.0;
                for segment in path.windows(2) {
                    let projected = Self::project_to_segment(segment[0], segment[1], point);
                    let distance = point.distance(projected);
                    if distance < nearest.0 {
                        nearest = (distance, along + segment[0].distance(projected));
                    }
                    along += segment[0].distance(segment[1]);
                }
                Some((*id, point, nearest.1))
            })
            .collect();
        let travel = length * self.rotation_fraction;
        if !closed
            && sources
                .iter()
                .any(|(_, _, start)| start + travel < -0.001 || start + travel > length + 0.001)
        {
            self.status =
                "Travel exceeds an open path endpoint; extend the guide or reduce travel".into();
            return;
        }
        let start_offset = self.active_subset().map_or(0.0, |s| s.offset_count);
        let end_offset = self.show.sets[self.current]
            .subsets
            .iter()
            .filter(|s| s.offset_count > start_offset + 0.001)
            .map(|s| s.offset_count)
            .min_by(f32::total_cmp)
            .unwrap_or(self.show.sets[self.current].counts_to_next as f32);
        self.snapshot();
        if self.current + 1 == self.show.sets.len() {
            self.show.add_set(self.current);
        }
        let samples = ((self.rotation_fraction.abs() * 96.0).ceil() as usize).clamp(8, 384);
        for index in 1..=samples {
            let t = index as f32 / samples as f32;
            let positions: BTreeMap<_, _> = sources
                .iter()
                .map(|(id, source, start)| {
                    let distance = if closed {
                        (start + travel * t).rem_euclid(length)
                    } else {
                        (start + travel * t).clamp(0.0, length)
                    };
                    let target = Self::point_on_polyline(path, distance);
                    let origin = Self::point_on_polyline(path, *start);
                    // Preserve each dot's offset from the guide, avoiding a jump onto it.
                    (
                        *id,
                        Point {
                            x: target.x + source.x - origin.x,
                            y: target.y + source.y - origin.y,
                        },
                    )
                })
                .collect();
            let offset = start_offset + (end_offset - start_offset) * t;
            if index == samples {
                if let Some(subset) = self.show.sets[self.current]
                    .subsets
                    .iter_mut()
                    .find(|s| (s.offset_count - end_offset).abs() < 0.001)
                {
                    subset.positions.extend(positions);
                } else {
                    self.show.sets[self.current + 1].positions.extend(positions);
                }
            } else {
                let id = self
                    .show
                    .sets
                    .iter()
                    .flat_map(|s| &s.subsets)
                    .map(|s| s.id)
                    .max()
                    .unwrap_or(0)
                    + 1;
                self.show.next_subset_id = self.show.next_subset_id.max(id + 1);
                self.show.sets[self.current].subsets.push(Subset {
                    id,
                    name: "Rotation".into(),
                    offset_count: offset,
                    positions,
                });
            }
        }
        self.show.sets[self.current]
            .subsets
            .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
        self.tool = Tool::Select;
        self.status = "Rotation created. Press Space to preview.".into();
    }

    fn synchronize_follow_order(&mut self) {
        self.follow_order.retain(|id| self.selected.contains(id));
        let mut missing: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|id| !self.follow_order.contains(id))
            .collect();
        while !missing.is_empty() {
            let last = self
                .follow_order
                .last()
                .and_then(|id| self.editable_position(*id));
            let index = last.map_or(0, |last| {
                missing
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        let distance = |id: &&u32| {
                            self.editable_position(**id)
                                .map_or(f32::INFINITY, |p| p.distance(last))
                        };
                        distance(a).total_cmp(&distance(b))
                    })
                    .map_or(0, |(index, _)| index)
            });
            self.follow_order.push(missing.remove(index));
        }
    }

    fn follow_the_leader(&mut self) {
        self.synchronize_follow_order();
        if self.follow_order.len() < 2 {
            self.status = "Select at least two performers".into();
            return;
        }
        let travel = self.follow_travel_steps;
        if !travel.is_finite() || travel <= 0.0 {
            return;
        }
        let Some(points) = self
            .follow_order
            .iter()
            .map(|id| self.editable_position(*id))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        if points.windows(2).any(|s| s[0].distance(s[1]) < 0.01) {
            self.status = "Separate coincident followers before generating the move".into();
            return;
        }
        let Some(mut guide_path) = self.follow_input_path(&points) else {
            return;
        };
        if self.follow_reverse && self.follow_route_mode != 0 {
            guide_path.reverse();
        }
        let length: f32 = guide_path.windows(2).map(|s| s[0].distance(s[1])).sum();
        if length < 0.001 {
            self.status = "Path has no length".into();
            return;
        }
        let closed = guide_path
            .first()
            .unwrap()
            .distance(*guide_path.last().unwrap())
            < 0.01;
        let leader = points[0];
        let mut nearest = (f32::INFINITY, 0.0, leader);
        let mut along = 0.0;
        for segment in guide_path.windows(2) {
            let point = Self::project_to_segment(segment[0], segment[1], leader);
            if point.distance(leader) < nearest.0 {
                nearest = (
                    point.distance(leader),
                    along + segment[0].distance(point),
                    point,
                );
            }
            along += segment[0].distance(segment[1]);
        }
        let connector = nearest.0;
        if !closed && travel > connector + length - nearest.1 + 0.001 {
            self.status = "Leader travel exceeds the path endpoint".into();
            return;
        }
        // The existing ordered formation is the trail behind the leader.
        // All dots advance by equal arc distance, preserving their initial intervals.
        let mut trail: Vec<_> = points.iter().rev().copied().collect();
        let mut starts = vec![0.0; points.len()];
        let mut distance = 0.0;
        for index in (0..points.len() - 1).rev() {
            distance += points[index].distance(points[index + 1]);
            starts[index] = distance;
        }
        if connector > 0.001 {
            trail.push(nearest.2);
        }
        let forward = (travel - connector).max(0.0);
        let segments = ((forward / 0.25).ceil() as usize).clamp(1, 8192);
        for index in 1..=segments {
            let at = nearest.1 + forward * index as f32 / segments as f32;
            let at = if closed {
                at.rem_euclid(length)
            } else {
                at.min(length)
            };
            trail.push(Self::point_on_polyline(&guide_path, at));
        }
        let start_offset = self.active_subset().map_or(0.0, |s| s.offset_count);
        let end_offset = self.show.sets[self.current]
            .subsets
            .iter()
            .filter(|s| s.offset_count > start_offset + 0.001)
            .map(|s| s.offset_count)
            .min_by(f32::total_cmp)
            .unwrap_or(self.show.sets[self.current].counts_to_next as f32);
        self.snapshot();
        if self.current + 1 == self.show.sets.len() {
            self.show.add_set(self.current);
        }
        let samples = ((travel / 0.25).ceil() as usize).clamp(8, 1024);
        let mut next_id = self.show.next_subset_id.max(
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
            let positions: BTreeMap<_, _> = self
                .follow_order
                .iter()
                .enumerate()
                .map(|(rank, id)| {
                    (
                        *id,
                        Self::point_on_polyline(&trail, starts[rank] + travel * t),
                    )
                })
                .collect();
            if index == samples {
                if let Some(subset) = self.show.sets[self.current]
                    .subsets
                    .iter_mut()
                    .find(|s| (s.offset_count - end_offset).abs() < 0.001)
                {
                    subset.positions.extend(positions);
                } else {
                    self.show.sets[self.current + 1].positions.extend(positions);
                }
            } else {
                self.show.sets[self.current].subsets.push(Subset {
                    id: next_id,
                    name: "Follow leader".into(),
                    offset_count: start_offset + (end_offset - start_offset) * t,
                    positions,
                });
                next_id += 1;
            }
        }
        self.show.next_subset_id = next_id;
        self.show.sets[self.current]
            .subsets
            .sort_by(|a, b| a.offset_count.total_cmp(&b.offset_count));
        self.tool = Tool::Select;
        self.status = "Follow motion created. Press Space to preview.".into();
    }

    pub(super) fn component_tools_panel(&mut self, ui: &mut egui::Ui) {
        if !self.show_component_tools {
            return;
        }
        self.synchronize_follow_order();
        if Self::palette_title(ui, "Tool Properties") {
            self.show_component_tools = false;
        }
        ui.horizontal_wrapped(|ui| {
            for (index, label) in [
                "Transform",
                "Rotate",
                "Follow",
                "Timing",
                "Gate",
                "Constraints",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.component_page, index, *label);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("tool_properties_scroll")
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if self.component_page == 0 || self.component_page == 4 || self.component_page == 1
                {
                    let mut custom = self.component_pivot_override.is_some();
                    if ui.checkbox(&mut custom, "Custom pivot").changed() {
                        self.component_pivot_override = if custom {
                            Some(self.component_pivot().unwrap_or_default())
                        } else {
                            None
                        };
                    }
                    if let Some(pivot) = &mut self.component_pivot_override {
                        ui.horizontal(|ui| {
                            ui.label("Pivot X");
                            Self::yard_drag(ui, &mut pivot.x, field::steps_to_yards(FIELD_W));
                            ui.label("Y");
                            Self::yard_drag(ui, &mut pivot.y, field::steps_to_yards(FIELD_H));
                        });
                    }
                    if self.component_page != 1 {
                        ui.horizontal(|ui| {
                            ui.label("Move X");
                            ui.add(
                                egui::DragValue::new(&mut self.component_translation.x)
                                    .suffix(" yd"),
                            );
                            ui.label("Y");
                            ui.add(
                                egui::DragValue::new(&mut self.component_translation.y)
                                    .suffix(" yd"),
                            );
                        });
                    }
                    if let Some(pivot) = self.component_pivot() {
                        ui.label(format!(
                            "Pivot  X {:.2} yd   Y {:.2} yd",
                            field::steps_to_yards(pivot.x),
                            field::steps_to_yards(pivot.y)
                        ));
                    }
                    if self.component_page != 1 {
                        ui.horizontal(|ui| {
                            ui.label("Scale");
                            ui.add(
                                egui::DragValue::new(&mut self.component_scale)
                                    .range(0.01..=100.0)
                                    .speed(0.01),
                            );
                            ui.label("Angle");
                            ui.add(egui::DragValue::new(&mut self.component_angle).suffix("°"));
                        });
                    }
                    if self.component_page == 0
                        && ui
                            .add_enabled(
                                !self.selected.is_empty() || self.selected_guide.is_some(),
                                egui::Button::new("Apply"),
                            )
                            .clicked()
                    {
                        self.transform_components();
                    }
                }
                if self.component_page == 1 {
                    ui.checkbox(&mut self.rotation_use_path, "Travel along a route");
                    if !self.rotation_use_path {
                        ui.horizontal(|ui| {
                            ui.label("Rotation");
                            ui.add(egui::DragValue::new(&mut self.rotation_degrees).suffix("°"));
                        });
                    }
                    if self.rotation_use_path {
                        self.route_input_controls(ui);
                        ui.horizontal(|ui| {
                            ui.label("Path travel");
                            ui.add(
                                egui::DragValue::new(&mut self.rotation_fraction)
                                    .speed(0.01)
                                    .range(-4.0..=4.0)
                                    .suffix(" × route length"),
                            );
                        });
                    }
                    if ui
                        .add_enabled(
                            !self.selected.is_empty(),
                            egui::Button::new("Generate rotation"),
                        )
                        .clicked()
                    {
                        self.rotation_drill();
                    }
                }
                if self.component_page == 2 {
                    egui::ComboBox::from_id_salt("follow_route_source")
                        .selected_text(
                            ["Continue forward", "Drawn route", "Selected guide"]
                                [self.follow_route_mode],
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.follow_route_mode, 0, "Continue forward");
                            ui.selectable_value(&mut self.follow_route_mode, 1, "Drawn route");
                            ui.selectable_value(&mut self.follow_route_mode, 2, "Selected guide");
                        });
                    if self.follow_route_mode == 1 {
                        self.route_input_controls(ui);
                    }
                    let mut swap = None;
                    egui::ScrollArea::vertical()
                        .id_salt("follow_order")
                        .max_height(theme::WINDOW_HEIGHT * 0.15)
                        .show(ui, |ui| {
                            for (rank, id) in self.follow_order.iter().enumerate() {
                                let label = self
                                    .show
                                    .performers
                                    .iter()
                                    .find(|p| p.id == *id)
                                    .map_or("?", |p| p.label.as_str());
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{}  {label}{}",
                                        rank + 1,
                                        if rank == 0 { " (leader)" } else { "" }
                                    ));
                                    if ui.add_enabled(rank > 0, egui::Button::new("Up")).clicked() {
                                        swap = Some((rank, rank - 1));
                                    }
                                    if ui
                                        .add_enabled(
                                            rank + 1 < self.follow_order.len(),
                                            egui::Button::new("Down"),
                                        )
                                        .clicked()
                                    {
                                        swap = Some((rank, rank + 1));
                                    }
                                });
                            }
                        });
                    if let Some((a, b)) = swap {
                        self.follow_order.swap(a, b);
                    }
                    ui.horizontal(|ui| {
                        ui.label("Travel");
                        ui.add(
                            egui::DragValue::new(&mut self.follow_travel_steps)
                                .range(0.125..=1600.0)
                                .suffix(" steps"),
                        );
                        ui.checkbox(&mut self.follow_reverse, "Reverse path");
                    });
                    if ui.add_enabled(self.selected.len() >= 2,egui::Button::new("Generate follow motion")).on_hover_text("Creates movement into the next subset or set. Space previews; Undo restores the previous move.").clicked() {
                        self.follow_the_leader();
                    }
                }
                self.maneuver_controls(ui);
            });
    }
}
