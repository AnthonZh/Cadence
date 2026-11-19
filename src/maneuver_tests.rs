use super::*;

#[test]
fn symmetry_and_concentric_constraints_follow_their_drivers() {
    let mut app = DrillApp::default();
    let a = app.show.add_performer(Point { x: 30.0, y: 20.0 });
    let b = app.show.add_performer(Point::default());
    app.show.sets[0].symmetry_axes.push(SymmetryAxis {
        id: 1,
        name: "Center".into(),
        start: Point { x: 40.0, y: 0.0 },
        end: Point { x: 40.0, y: 80.0 },
        threshold_steps: 0.0,
        mode: SymmetryMode::Perfect,
    });
    app.add_sketch_rule(SketchRule::Symmetry {
        axis: 1,
        pairs: vec![(a, b)],
    });
    assert_eq!(app.editable_position(b), Some(Point { x: 50.0, y: 20.0 }));
    app.set_editable_position(a, Point { x: 32.0, y: 22.0 });
    app.enforce_sketch_constraints();
    assert_eq!(app.editable_position(b), Some(Point { x: 48.0, y: 22.0 }));
    for (id, center) in [
        (1, Point { x: 40.0, y: 40.0 }),
        (2, Point { x: 20.0, y: 20.0 }),
    ] {
        app.show.sets[0].guides.push(FormationGuide {
            id,
            spacing_steps: 2.0,
            spot_count: 8,
            fill_guide: true,
            reverse_spots: false,
            kind: GuideKind::Circle {
                center,
                radius: 5.0,
            },
        });
    }
    app.add_sketch_rule(SketchRule::Concentric {
        guide: 2,
        reference: 1,
    });
    assert!(matches!(
        app.show.sets[0].guides[1].kind,
        GuideKind::Circle {
            center: Point { x: 40.0, y: 40.0 },
            ..
        }
    ));
    let count = app.show.sets[0].constraints.len();
    app.add_sketch_rule(SketchRule::Concentric {
        guide: 1,
        reference: 2,
    });
    assert_eq!(app.show.sets[0].constraints.len(), count);
}

#[test]
fn timing_survives_document_roundtrip_and_rejects_overrun() {
    let mut app = DrillApp::default();
    let id = app.show.add_performer(Point::default());
    app.show.add_set(0);
    app.show.sets[1]
        .positions
        .insert(id, Point { x: 8.0, y: 0.0 });
    app.selected.insert(id);
    app.move_delay = 4.0;
    app.move_duration = 4.0;
    app.apply_move_timing(false);
    let loaded: Show = serde_json::from_slice(&serde_json::to_vec(&app.show).unwrap()).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded.position_at(id, 6.0), Some(Point { x: 4.0, y: 0.0 }));
    app.move_delay = 7.0;
    app.apply_move_timing(false);
    assert_eq!(app.show.sets[0].timing[&id].delay, 4.0);
}

#[test]
fn hold_stagger_and_pace_retime_existing_motion() {
    let mut app = DrillApp::default();
    let first = app.show.add_performer(Point::default());
    let second = app.show.add_performer(Point::default());
    app.show.add_set(0);
    for id in [first, second] {
        app.show.sets[1]
            .positions
            .insert(id, Point { x: 8.0, y: 0.0 });
    }
    app.selected.extend([first, second]);
    app.move_delay = 2.0;
    app.move_stagger = 2.0;
    app.move_duration = 4.0;
    app.move_pace = 1.0;
    app.apply_move_timing(false);
    assert_eq!(app.show.position_at(first, 2.0).unwrap().x, 0.0);
    assert_eq!(app.show.position_at(first, 4.0).unwrap().x, 4.0);
    assert_eq!(app.show.position_at(second, 4.0).unwrap().x, 0.0);
    assert_eq!(app.show.position_at(second, 8.0).unwrap().x, 8.0);
    app.move_pace = 2.0;
    app.apply_move_timing(false);
    assert_eq!(app.show.position_at(first, 4.0).unwrap().x, 8.0);
    app.hold_transition();
    assert_eq!(app.show.position_at(first, 7.0).unwrap(), Point::default());
}

#[test]
fn gate_keeps_pivot_and_interval_through_intermediate_counts() {
    let mut app = DrillApp::default();
    let pivot = Point { x: 40.0, y: 40.0 };
    let first = app.show.add_performer(pivot);
    let second = app.show.add_performer(Point { x: 50.0, y: 40.0 });
    app.selected.extend([first, second]);
    app.component_pivot_override = Some(pivot);
    app.component_angle = 180.0;
    app.rigid_maneuver(true);
    assert!(
        app.show
            .position_at(second, 4.0)
            .unwrap()
            .distance(Point { x: 40.0, y: 50.0 })
            < 0.001
    );
    assert_eq!(app.show.position_at(first, 4.0), Some(pivot));
    for count in 0..=8 {
        assert!(
            (app.show
                .position_at(first, count as f32)
                .unwrap()
                .distance(app.show.position_at(second, count as f32).unwrap())
                - 10.0)
                .abs()
                < 0.01
        );
    }
    app.undo();
    assert_eq!(app.show.sets.len(), 1);
}

#[test]
fn binding_survives_guide_edit_and_json_roundtrip_and_conflicts_reject() {
    let mut app = DrillApp::default();
    let first = app.show.add_performer(Point::default());
    let second = app.show.add_performer(Point::default());
    app.show.sets[0].guides.push(FormationGuide {
        id: 1,
        spacing_steps: 2.0,
        spot_count: 2,
        fill_guide: true,
        reverse_spots: false,
        kind: GuideKind::Line {
            start: Point { x: 10.0, y: 10.0 },
            end: Point { x: 20.0, y: 10.0 },
        },
    });
    app.add_sketch_rule(SketchRule::OnGuide {
        guide: 1,
        performers: vec![first, second],
    });
    app.show.sets[0].guides[0].kind = GuideKind::Line {
        start: Point { x: 30.0, y: 20.0 },
        end: Point { x: 40.0, y: 20.0 },
    };
    app.enforce_sketch_constraints();
    assert_eq!(
        app.editable_position(first),
        Some(Point { x: 30.0, y: 20.0 })
    );
    app.add_sketch_rule(SketchRule::Fixed {
        performer: first,
        point: Point::default(),
    });
    assert_eq!(app.show.sets[0].constraints.len(), 1);
    let bytes = serde_json::to_vec(&app.show).unwrap();
    let loaded: Show = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(loaded.sets[0].constraints, app.show.sets[0].constraints);
    loaded.validate().unwrap();
}

#[test]
fn float_translates_every_dot_identically() {
    let mut app = DrillApp::default();
    let a = app.show.add_performer(Point { x: 10.0, y: 10.0 });
    let b = app.show.add_performer(Point { x: 14.0, y: 10.0 });
    app.selected.extend([a, b]);
    app.component_translation = Point { x: 5.0, y: 0.0 };
    app.rigid_maneuver(false);
    assert_eq!(
        app.show.position_at(a, 4.0),
        Some(Point { x: 14.0, y: 10.0 })
    );
    assert_eq!(
        app.show.position_at(b, 4.0),
        Some(Point { x: 18.0, y: 10.0 })
    );
}
