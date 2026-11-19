use super::*;

impl DrillApp {
    pub(super) fn selected_motion_path(&self) -> Option<Vec<Point>> {
        let paths = Self::guide_paths(&self.selected_guide()?);
        (paths.len() == 1 && paths[0].len() >= 2).then(|| paths[0].clone())
    }

    pub(super) fn optional_motion_path(&self) -> Option<Vec<Point>> {
        if self.motion_route.len() >= 2 {
            Some(self.motion_route.clone())
        } else {
            self.selected_motion_path()
        }
    }

    pub(super) fn follow_input_path(&mut self, points: &[Point]) -> Option<Vec<Point>> {
        match self.follow_route_mode {
            0 => {
                let direction = Point {
                    x: points[0].x - points[1].x,
                    y: points[0].y - points[1].y,
                };
                let length = points[0].distance(points[1]);
                if length < 0.001 {
                    self.status = "Separate the leader and first follower".into();
                    return None;
                }
                let travel =
                    self.follow_travel_steps * if self.follow_reverse { -1.0 } else { 1.0 };
                Some(vec![
                    points[0],
                    Point {
                        x: points[0].x + direction.x / length * travel,
                        y: points[0].y + direction.y / length * travel,
                    },
                ])
            }
            1 if self.motion_route.len() >= 2 => Some(self.motion_route.clone()),
            2 => {
                let path = self.selected_motion_path();
                if path.is_none() {
                    self.status = "Select a continuous guide or choose Continue forward".into();
                }
                path
            }
            _ => {
                self.status = "Draw a route with at least two points".into();
                None
            }
        }
    }

    pub(super) fn route_input_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.selected.is_empty(), egui::Button::new("Draw route"))
                .clicked()
            {
                self.motion_route.clear();
                let first = self
                    .follow_order
                    .first()
                    .copied()
                    .or_else(|| self.selected.first().copied());
                if let Some(point) = first.and_then(|id| self.editable_position(id)) {
                    self.motion_route.push(point);
                }
                self.tool = Tool::MotionRoute;
                self.playing = false;
                self.status =
                    "Route: click waypoints or drag; Enter finishes, Escape cancels".into();
            }
            if ui
                .add_enabled(self.motion_route.len() >= 2, egui::Button::new("Finish"))
                .clicked()
            {
                self.tool = Tool::Select;
            }
            if ui.small_button("Clear").clicked() {
                self.motion_route.clear();
                self.tool = Tool::Select;
            }
        });
        ui.label(format!("{} route points", self.motion_route.len()));
        if self.tool == Tool::MotionRoute {
            ui.label("Click waypoints · Enter finishes · Esc cancels");
        }
        if self.component_page == 1 && self.motion_route.len() < 2 {
            ui.label(if self.selected_motion_path().is_some() {
                "Path: selected guide"
            } else {
                "Draw a route or select a guide"
            });
        }
        egui::CollapsingHeader::new("Waypoints").show(ui, |ui| {
            let mut remove = None;
            for (index, point) in self.motion_route.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}", index + 1));
                    Self::yard_drag(ui, &mut point.x, field::LENGTH_YARDS);
                    Self::yard_drag(ui, &mut point.y, field::WIDTH_YARDS);
                    if ui.small_button("×").clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                self.motion_route.remove(index);
            }
        });
    }

    pub(super) fn motion_route_canvas(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
        view: FieldView,
    ) -> bool {
        for segment in self.motion_route.windows(2) {
            painter.line_segment(
                [view.screen(segment[0]), view.screen(segment[1])],
                Stroke::new(theme::GUIDE_SELECTED_WIDTH, theme::ACCENT),
            );
        }
        for point in &self.motion_route {
            painter.circle_filled(
                view.screen(*point),
                theme::GUIDE_HANDLE_RADIUS,
                theme::ACCENT,
            );
        }
        if self.tool != Tool::MotionRoute {
            return false;
        }
        if let Some(pos) = response.interact_pointer_pos() {
            let point = self.snap_point(view.field(pos));
            if response.clicked_by(PointerButton::Primary)
                || response.dragged_by(PointerButton::Primary)
            {
                let spacing = if response.dragged_by(PointerButton::Primary) {
                    0.5
                } else {
                    0.01
                };
                if self
                    .motion_route
                    .last()
                    .is_none_or(|p| p.distance(point) > spacing)
                {
                    self.motion_route.push(point);
                }
            }
        }
        if let (Some(last), Some(pos)) = (self.motion_route.last(), response.hover_pos()) {
            Self::draw_dashed_line(
                painter,
                view.screen(*last),
                pos,
                Stroke::new(theme::GUIDE_WIDTH, theme::ACCENT),
            );
        }
        if response.double_clicked_by(PointerButton::Primary) {
            self.tool = Tool::Select;
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        true
    }
}
