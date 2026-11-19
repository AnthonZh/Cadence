use super::*;

pub(super) fn icon_button(ui: &mut egui::Ui, icon: &str, tip: &str, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(theme::TOOL_SIZE), Sense::click());
    let painter = ui.painter();
    if active || response.hovered() {
        painter.rect_filled(
            rect,
            theme::ZERO_RADIUS,
            if active {
                theme::TOOL_ACTIVE
            } else {
                theme::TOOL_HOVER
            },
        );
        painter.rect_stroke(
            rect,
            theme::ZERO_RADIUS,
            Stroke::new(theme::BORDER_WIDTH, theme::BORDER),
            egui::StrokeKind::Inside,
        );
    }
    let r = rect.shrink(theme::ICON_INSET);
    let p = |x: f32, y: f32| Pos2::new(r.left() + r.width() * x, r.top() + r.height() * y);
    let stroke = Stroke::new(
        theme::NORMAL_LINE_WIDTH,
        if ui.is_enabled() {
            theme::TEXT
        } else {
            theme::MUTED
        },
    );
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let path = |points: &[(f32, f32)]| {
        painter.add(egui::Shape::line(
            points.iter().map(|(x, y)| p(*x, *y)).collect(),
            stroke,
        ));
    };
    match icon {
        "new" => {
            path(&[
                (0.15, 1.0),
                (0.15, 0.0),
                (0.7, 0.0),
                (0.95, 0.25),
                (0.95, 1.0),
                (0.15, 1.0),
            ]);
            path(&[(0.7, 0.0), (0.7, 0.25), (0.95, 0.25)]);
        }
        "open" => {
            path(&[
                (0.0, 0.9),
                (0.0, 0.15),
                (0.4, 0.15),
                (0.5, 0.3),
                (1.0, 0.3),
                (0.85, 0.9),
                (0.0, 0.9),
                (0.15, 0.45),
                (0.95, 0.45),
            ]);
        }
        "save" => {
            painter.rect_stroke(r, theme::ZERO_RADIUS, stroke, egui::StrokeKind::Inside);
            path(&[(0.2, 0.0), (0.2, 0.4), (0.75, 0.4), (0.75, 0.0)]);
            path(&[(0.2, 1.0), (0.2, 0.65), (0.75, 0.65), (0.75, 1.0)]);
        }
        "copy" | "paste" => {
            painter.rect_stroke(
                egui::Rect::from_min_max(p(0.0, 0.0), p(0.7, 0.7)),
                theme::ZERO_RADIUS,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                egui::Rect::from_min_max(p(0.3, 0.3), p(1.0, 1.0)),
                theme::ZERO_RADIUS,
                theme::CHROME,
            );
            painter.rect_stroke(
                egui::Rect::from_min_max(p(0.3, 0.3), p(1.0, 1.0)),
                theme::ZERO_RADIUS,
                stroke,
                egui::StrokeKind::Inside,
            );
            if icon == "paste" {
                line((0.45, 0.65), (0.85, 0.65));
                line((0.65, 0.45), (0.65, 0.85));
            }
        }
        "undo" => {
            path(&[(0.1, 0.4), (0.65, 0.4), (0.9, 0.65), (0.65, 0.9)]);
            path(&[(0.4, 0.1), (0.1, 0.4), (0.4, 0.7)]);
        }
        "redo" => {
            path(&[(0.9, 0.4), (0.35, 0.4), (0.1, 0.65), (0.35, 0.9)]);
            path(&[(0.6, 0.1), (0.9, 0.4), (0.6, 0.7)]);
        }
        "select" => path(&[
            (0.1, 0.0),
            (0.1, 1.0),
            (0.4, 0.7),
            (0.65, 1.0),
            (0.8, 0.85),
            (0.55, 0.55),
            (0.95, 0.5),
            (0.1, 0.0),
        ]),
        "dot" => {
            painter.circle_filled(r.center(), r.width() * 0.2, stroke.color);
            line((0.65, 0.05), (1.0, 0.05));
            line((0.825, -0.1), (0.825, 0.2));
        }
        "line" => {
            line((0.1, 0.9), (0.9, 0.1));
            for (x, y) in [(0.1, 0.9), (0.9, 0.1)] {
                painter.circle_filled(p(x, y), theme::GUIDE_SPOT_RADIUS, stroke.color);
            }
        }
        "circle" => {
            painter.circle_stroke(r.center(), r.width() * 0.45, stroke);
        }
        "shape" => {
            path(&[
                (0.5, 0.0),
                (1.0, 0.4),
                (0.8, 1.0),
                (0.2, 1.0),
                (0.0, 0.4),
                (0.5, 0.0),
            ]);
        }
        "pan" => {
            line((0.0, 0.5), (1.0, 0.5));
            line((0.5, 0.0), (0.5, 1.0));
            path(&[(0.2, 0.3), (0.0, 0.5), (0.2, 0.7)]);
            path(&[(0.8, 0.3), (1.0, 0.5), (0.8, 0.7)]);
        }
        "axis" => {
            line((0.5, 0.0), (0.5, 1.0));
            path(&[(0.3, 0.2), (0.0, 0.8), (0.3, 0.8), (0.3, 0.2)]);
            path(&[(0.7, 0.2), (1.0, 0.8), (0.7, 0.8), (0.7, 0.2)]);
        }
        "set" | "subset" => {
            line((0.0, 0.8), (1.0, 0.8));
            line((0.2, 0.5), (0.2, 1.0));
            line((0.8, 0.5), (0.8, 1.0));
            line((0.5, 0.0), (0.5, 0.55));
            line((0.3, 0.25), (0.7, 0.25));
            if icon == "subset" {
                line((0.5, 0.65), (0.5, 1.0));
            }
        }
        "spline" | "follow" => {
            path(&[
                (0.0, 0.9),
                (0.15, 0.65),
                (0.4, 0.7),
                (0.6, 0.3),
                (0.85, 0.35),
                (1.0, 0.1),
            ]);
            if icon == "follow" {
                for (x, y) in [(0.0, 0.9), (0.4, 0.7), (0.85, 0.35)] {
                    painter.circle_filled(p(x, y), theme::GUIDE_SPOT_RADIUS, stroke.color);
                }
            }
        }
        "grid" | "snap" => {
            for at in [0.15, 0.5, 0.85] {
                line((at, 0.0), (at, 1.0));
                line((0.0, at), (1.0, at));
            }
            if icon == "snap" {
                painter.circle_filled(r.center(), theme::GUIDE_HANDLE_RADIUS, stroke.color);
            }
        }
        "play" => {
            painter.add(egui::Shape::convex_polygon(
                vec![p(0.2, 0.0), p(1.0, 0.5), p(0.2, 1.0)],
                stroke.color,
                Stroke::NONE,
            ));
        }
        "pause" => {
            line((0.3, 0.0), (0.3, 1.0));
            line((0.7, 0.0), (0.7, 1.0));
        }
        "previous" => {
            path(&[(0.8, 0.0), (0.2, 0.5), (0.8, 1.0)]);
            line((0.1, 0.0), (0.1, 1.0));
        }
        "next" => {
            path(&[(0.2, 0.0), (0.8, 0.5), (0.2, 1.0)]);
            line((0.9, 0.0), (0.9, 1.0));
        }
        "rotate" | "timing" => {
            painter.circle_stroke(r.center(), r.width() * 0.45, stroke);
            if icon == "timing" {
                path(&[(0.5, 0.1), (0.5, 0.5), (0.8, 0.5)]);
            } else {
                path(&[(0.55, 0.0), (1.0, 0.0), (1.0, 0.45)]);
            }
        }
        "constraint" => {
            painter.circle_stroke(p(0.3, 0.5), r.width() * 0.28, stroke);
            painter.circle_stroke(p(0.7, 0.5), r.width() * 0.28, stroke);
        }
        "fit" | "transform" => {
            for (x, y, sx, sy) in [
                (0.0, 0.0, 1.0, 1.0),
                (1.0, 0.0, -1.0, 1.0),
                (0.0, 1.0, 1.0, -1.0),
                (1.0, 1.0, -1.0, -1.0),
            ] {
                path(&[(x, y + sy * 0.3), (x, y), (x + sx * 0.3, y)]);
            }
            if icon == "transform" {
                line((0.3, 0.7), (0.7, 0.3));
            }
        }
        "project" | "compass" => {
            line((0.0, 0.85), (1.0, 0.85));
            for x in [0.15, 0.5, 0.85] {
                painter.circle_filled(p(x, 0.1), theme::GUIDE_SPOT_RADIUS, stroke.color);
                line((x, 0.3), (x, 0.7));
            }
            if icon == "compass" {
                line((0.15, 0.3), (0.85, 0.7));
            }
        }
        _ => {
            line((0.0, 0.5), (1.0, 0.5));
            line((0.5, 0.0), (0.5, 1.0));
        }
    }
    response.on_hover_text(tip).clicked()
}

impl DrillApp {
    pub(super) fn palette_bounds(&self, ctx: &egui::Context) -> Rect {
        let mut bounds = ctx.content_rect();
        bounds.min.y += theme::MENU_HEIGHT + theme::TOOLBAR_HEIGHT + theme::OPTIONS_HEIGHT;
        bounds.max.y -= theme::TIMELINE_HEIGHT + theme::STATUS_HEIGHT;
        if self.show_component_tools {
            bounds.max.x -= theme::COMMAND_PANEL_WIDTH;
        }
        bounds
    }
    pub(super) fn workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal()
            .id_salt("command_toolbar")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (group, commands) in [
                        (
                            "File",
                            vec![
                                ("new", Action::NewDrill),
                                ("open", Action::OpenDrill),
                                ("save", Action::SaveDrill),
                            ],
                        ),
                        (
                            "Edit",
                            vec![
                                ("undo", Action::Undo),
                                ("redo", Action::Redo),
                                ("copy", Action::CopyComponents),
                                ("paste", Action::PasteComponents),
                            ],
                        ),
                        (
                            "Place",
                            vec![
                                ("select", Action::SelectTool),
                                ("pan", Action::PanTool),
                                ("dot", Action::PlaceTool),
                                ("line", Action::LineGuideTool),
                                ("circle", Action::CircleGuideTool),
                                ("spline", Action::SplineGuideTool),
                                ("shape", Action::OpenShapePalette),
                                ("axis", Action::AddSymmetryAxis),
                            ],
                        ),
                        (
                            "Match",
                            vec![
                                ("project", Action::ProjectToGuide),
                                ("compass", Action::DistributeOnGuide),
                            ],
                        ),
                        (
                            "View",
                            vec![
                                ("fit", Action::ZoomToFit),
                                ("grid", Action::ToggleGrid),
                                ("snap", Action::ToggleGridSnap),
                            ],
                        ),
                        (
                            "Set",
                            vec![("set", Action::AddSet), ("subset", Action::AddSubset)],
                        ),
                    ] {
                        if group != "File" {
                            ui.separator();
                            ui.add_space(theme::GROUP_GAP);
                        }
                        ui.label(egui::RichText::new(group).color(theme::MUTED));
                        for (icon, action) in commands {
                            let active = match action {
                                Action::SelectTool => self.tool == Tool::Select,
                                Action::PlaceTool => self.tool == Tool::Place,
                                Action::PanTool => self.tool == Tool::Pan,
                                Action::LineGuideTool => self.tool == Tool::LineGuide,
                                Action::CircleGuideTool => self.tool == Tool::CircleGuide,
                                Action::SplineGuideTool => self.tool == Tool::SplineGuide,
                                Action::ToggleGrid => self.show_grid,
                                Action::ToggleGridSnap => self.snap_to_grid,
                                _ => false,
                            };
                            let enabled = match action {
                                Action::Undo => !self.undo.is_empty(),
                                Action::Redo => !self.redo.is_empty(),
                                Action::CopyComponents => {
                                    !self.selected.is_empty() || self.selected_guide.is_some()
                                }
                                Action::PasteComponents => self.component_clipboard.is_some(),
                                Action::ProjectToGuide | Action::DistributeOnGuide => {
                                    !self.selected.is_empty() && self.selected_guide.is_some()
                                }
                                _ => true,
                            };
                            let clicked = ui
                                .add_enabled_ui(enabled, |ui| {
                                    icon_button(
                                        ui,
                                        icon,
                                        &self.shortcut_tip(action.label(), action),
                                        active,
                                    )
                                })
                                .inner;
                            if clicked {
                                self.run_action(action, ui.ctx());
                            }
                        }
                    }
                    ui.separator();
                    for (page, icon, label) in [
                        (0, "transform", "Transform"),
                        (1, "rotate", "Rotate performers"),
                        (2, "follow", "Follow leader"),
                        (3, "timing", "Timing"),
                        (4, "rotate", "Gate / float"),
                        (5, "constraint", "Constraints"),
                    ] {
                        if icon_button(
                            ui,
                            icon,
                            label,
                            self.show_component_tools && self.component_page == page,
                        ) {
                            self.component_page = page;
                            self.show_component_tools = true;
                        }
                    }
                    ui.separator();
                    for (icon, action) in [
                        ("previous", Action::PreviousSet),
                        (
                            if self.playing { "pause" } else { "play" },
                            Action::PlayPause,
                        ),
                        ("next", Action::NextSet),
                    ] {
                        if icon_button(ui, icon, &self.shortcut_tip(action.label(), action), false)
                        {
                            self.run_action(action, ui.ctx());
                        }
                    }
                    ui.menu_button("Output", |ui| {
                        if ui.button("Chart PNG…").clicked() {
                            self.export();
                            ui.close();
                        }
                        if ui.button("Performer packets…").clicked() {
                            self.export_performer_packets();
                            ui.close();
                        }
                    });
                    if ui.button("Music").clicked() {
                        self.show_music_setup = true;
                    }
                });
            });
    }

    pub(super) fn workspace_options(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong(
                self.file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|p| p.to_str())
                    .unwrap_or("Untitled.mdrill"),
            );
            if self.dirty {
                ui.label("*");
            }
            ui.separator();
            ui.label(format!("Set {}", self.current + 1));
            ui.label(
                egui::RichText::new(format!("{} selected", self.selected.len()))
                    .color(theme::MUTED),
            );
            ui.separator();
            ui.checkbox(&mut self.snap_to_grid, "Snap");
            ui.add(
                egui::DragValue::new(&mut self.grid_snap_steps)
                    .range(0.125..=16.0)
                    .suffix(" st"),
            );
            ui.label("Nudge");
            ui.add(
                egui::DragValue::new(&mut self.nudge_steps)
                    .range(0.125..=16.0)
                    .suffix(" st"),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.toggle_value(&mut self.show_component_tools, "Properties");
                ui.toggle_value(&mut self.show_inspector, "Inspector");
                ui.toggle_value(&mut self.show_preview, "Preview");
            });
        });
    }
}
