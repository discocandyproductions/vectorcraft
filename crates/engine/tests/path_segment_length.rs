//! `path.setSegmentLength` / the Path Length panel's sub-path reading: selecting one or more
//! *interior* anchors of a subpath (leaving an anchor unselected on each side) scales the curve
//! between those two flanking anchors to a target length, while the flanking anchors themselves
//! — and everything outside the widened run — stay exactly put.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use vectorcraft_doc::NodeId;
use vectorcraft_engine::Session;

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    s
}

fn anchor(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// A 3-anchor open path with a visible bow in the middle anchor, so scaling its deviation
/// actually changes the path's length.
fn bowed_open_path(s: &mut Session) -> NodeId {
    let id = s
        .execute(
            "path.create",
            &json!({"anchors": [anchor(0.0, 0.0), {"x": 50.0, "y": -20.0, "in": [30.0, -20.0], "out": [70.0, -20.0]}, anchor(100.0, 0.0)]}),
        )
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    NodeId(id)
}

fn path(s: &Session, id: NodeId) -> vectorcraft_geom::PathData {
    s.active().unwrap().doc.node(id).unwrap().path_data().unwrap().clone()
}

#[test]
fn selecting_the_middle_of_three_anchors_keeps_the_two_ends_fixed() {
    let mut s = session();
    let id = bowed_open_path(&mut s);
    let before = path(&s, id);
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 1]], "mode": "set"})).unwrap();

    let reported = s.segment_run_length().expect("a 3-anchor run with the middle one selected is a valid sub-path pick");
    let original_length = before.subpaths[0].partial_length(0..2);
    assert!((reported - original_length).abs() < 1e-6, "reported length should be the whole 0..2 span, not just anchor 1's segments");

    let target = original_length * 1.5;
    s.execute("path.setSegmentLength", &json!({"length": target})).unwrap();
    let after = path(&s, id);

    assert_eq!(after.subpaths[0].anchors[0].p, before.subpaths[0].anchors[0].p, "the first anchor must not move");
    assert_eq!(after.subpaths[0].anchors[2].p, before.subpaths[0].anchors[2].p, "the last anchor must not move");
    assert_ne!(after.subpaths[0].anchors[1].p, before.subpaths[0].anchors[1].p, "the selected middle anchor should have moved");
    assert!((after.subpaths[0].partial_length(0..2) - target).abs() < 1e-3);
}

#[test]
fn selecting_a_run_of_several_interior_anchors_keeps_only_the_flanking_ones_fixed() {
    let mut s = session();
    let anchors: Vec<Value> = vec![
        anchor(0.0, 0.0),
        json!({"x": 20.0, "y": -10.0, "in": [10.0, -10.0], "out": [30.0, -10.0]}),
        json!({"x": 40.0, "y": -15.0, "in": [30.0, -15.0], "out": [50.0, -15.0]}),
        json!({"x": 60.0, "y": -10.0, "in": [50.0, -10.0], "out": [70.0, -10.0]}),
        anchor(80.0, 0.0),
    ];
    let id = NodeId(s.execute("path.create", &json!({"anchors": anchors})).unwrap()["id"].as_u64().unwrap());
    let before = path(&s, id);
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    // Anchors 1, 2, 3 selected; 0 and 4 left unselected on purpose.
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 1], [0, 2], [0, 3]], "mode": "set"})).unwrap();

    let original_length = before.subpaths[0].partial_length(0..4);
    let reported = s.segment_run_length().unwrap();
    assert!((reported - original_length).abs() < 1e-6);

    let target = original_length * 1.3;
    s.execute("path.setSegmentLength", &json!({"length": target})).unwrap();
    let after = path(&s, id);

    assert_eq!(after.subpaths[0].anchors[0].p, before.subpaths[0].anchors[0].p);
    assert_eq!(after.subpaths[0].anchors[4].p, before.subpaths[0].anchors[4].p);
    for i in 1..4 {
        assert_ne!(after.subpaths[0].anchors[i].p, before.subpaths[0].anchors[i].p, "anchor {i} should move");
    }
    assert!((after.subpaths[0].partial_length(0..4) - target).abs() < 1e-3);
}

#[test]
fn selecting_an_open_paths_own_end_anchor_is_not_a_valid_sub_path_pick() {
    let mut s = session();
    let id = bowed_open_path(&mut s);
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    // Anchor 0 is the path's own first endpoint: there's no anchor before it to hold fixed.
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 0]], "mode": "set"})).unwrap();
    assert!(s.segment_run_length().is_none());
    assert!(s.execute("path.setSegmentLength", &json!({"length": 500.0})).is_err());
}

#[test]
fn a_closed_paths_run_can_wrap_through_the_start() {
    let mut s = session();
    // A closed quad; select anchors 3 and 0 (wrapping past the end), leaving 1 and 2 as the
    // flanking, unselected anchors.
    let anchors: Vec<Value> = vec![
        json!({"x": 0.0, "y": 0.0, "in": [-10.0, 10.0], "out": [10.0, -10.0]}),
        anchor(100.0, 0.0),
        anchor(100.0, 100.0),
        json!({"x": 0.0, "y": 100.0, "in": [10.0, 110.0], "out": [-10.0, 90.0]}),
    ];
    let id = NodeId(s.execute("path.create", &json!({"anchors": anchors, "closed": true})).unwrap()["id"].as_u64().unwrap());
    let before = path(&s, id);
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 3], [0, 0]], "mode": "set"})).unwrap();

    let reported = s.segment_run_length();
    assert!(reported.is_some(), "a wrapping run with a free anchor on each side (1 and 2) should be valid");

    s.execute("path.setSegmentLength", &json!({"length": reported.unwrap() * 1.2})).unwrap();
    let after = path(&s, id);
    assert_eq!(after.subpaths[0].anchors[1].p, before.subpaths[0].anchors[1].p, "anchor 1 flanks the run and must stay put");
    assert_eq!(after.subpaths[0].anchors[2].p, before.subpaths[0].anchors[2].p, "anchor 2 flanks the run and must stay put");
}

#[test]
fn selecting_every_anchor_but_one_on_a_closed_path_leaves_no_room_for_two_flanking_anchors() {
    let mut s = session();
    let anchors: Vec<Value> = vec![anchor(0.0, 0.0), anchor(100.0, 0.0), anchor(100.0, 100.0), anchor(0.0, 100.0), anchor(-50.0, 50.0)];
    let id = NodeId(s.execute("path.create", &json!({"anchors": anchors, "closed": true})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    // Every anchor but index 4 selected: only one anchor would be left to flank both ends.
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 0], [0, 1], [0, 2], [0, 3]], "mode": "set"})).unwrap();
    assert!(s.segment_run_length().is_none());
}

#[test]
fn the_panels_whole_selection_mode_still_applies_when_no_anchors_are_selected() {
    let mut s = session();
    let id = bowed_open_path(&mut s);
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    assert!(s.segment_run_length().is_none(), "with no direct-selected anchors, the panel should fall back to whole-selection mode");
    assert!(s.selection_length(&[id]).is_some());
}
