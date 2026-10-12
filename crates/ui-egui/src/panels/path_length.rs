//! Path Length panel. Two modes, chosen by what's selected:
//! - Whole selection (the default): the total path length (every path in it summed — a compound
//!   path's and a selected group's paths included), editable to uniformly scale the selection,
//!   about its centre, to a new length (`object.setLength`).
//! - Sub-path: with a direct-selected contiguous run of anchor points within one subpath (not the
//!   whole subpath), the length of just that run, editable to scale the curve's deviation from the
//!   straight chord between the run's two end anchors — which stay put (`path.setSegmentLength`).
//!
//! Unlike Illustrator's Document Info, which only reports a path's length, this field can be
//! typed into; both modes are beyond parity (see `docs/ui-parity.md`).

use egui::Ui;
use serde_json::json;

use crate::VectorcraftApp;
use crate::widgets;

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let Some(st) = app.session.active() else {
        widgets::dim_label(ui, tl!("No document"));
        return;
    };
    let units = app.session.general_unit();
    // Sub-path mode wins when the selection is a proper anchor run; it's a more specific pick
    // than a path (or several) selected whole.
    if let Some(len) = app.session.segment_run_length().filter(|l| *l > 1e-9) {
        ui.horizontal(|ui| {
            widgets::dim_label(ui, tl!("Length:"));
            if let Some(v) = widgets::num_field(ui, "pathlen-length", Some(len), units, 80.0)
                && v > 0.0
                && let Err(e) = app.run("path.setSegmentLength", json!({"length": v}))
            {
                app.status(e);
            }
        });
        return;
    }
    let ids = st.selection.objects.clone();
    let len = app.session.selection_length(&ids).filter(|l| *l > 1e-9);
    let has = !ids.is_empty() && len.is_some();
    ui.horizontal(|ui| {
        widgets::dim_label(ui, tl!("Length:"));
        ui.add_enabled_ui(has, |ui| {
            if let Some(v) = widgets::num_field(ui, "pathlen-length", len, units, 80.0)
                && v > 0.0
                && let Err(e) = app.run("object.setLength", json!({"length": v}))
            {
                app.status(e);
            }
        });
    });
    if !ids.is_empty() && len.is_none() {
        widgets::dim_label(ui, tl!("Selection has no length to scale"));
    }
}
