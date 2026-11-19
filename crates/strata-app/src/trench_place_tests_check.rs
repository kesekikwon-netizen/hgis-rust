use std::collections::BTreeMap;

use strata_geo::trench_plan::Cell;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;

use super::{X0, Y0};
use crate::app_state::StrataAppState;
use crate::survey_paint_layers::{build_paint_layers, paint_order};

pub(super) fn assert_placed(app: &StrataAppState, cells: &[Cell]) {
    let table = app
        .survey_tables
        .iter()
        .find(|t| t.table == "trial_trench")
        .expect("trial_trench");
    assert_eq!(table.layer.features.len(), cells.len());
    assert_eq!(table.srs_id, 5187);
    let mut got: BTreeMap<String, (f64, f64, Vec<[f64; 2]>)> = BTreeMap::new();
    for feature in &table.layer.features {
        let name = feature.attributes.get("name").expect("name").clone();
        let width: f64 = feature.attributes.get("width").unwrap().parse().unwrap();
        let length: f64 = feature.attributes.get("length").unwrap().parse().unwrap();
        let ring = exterior(&feature.wkt_geom);
        assert!(
            ring.iter()
                .any(|p| (X0..=X0 + 80.0).contains(&p[0]) && (Y0..=Y0 + 80.0).contains(&p[1])),
            "vertex outside the EPSG:5187 square: {ring:?}"
        );
        got.insert(name, (width, length, ring));
    }
    for cell in cells {
        let (width, length, ring) = got
            .get(&cell.name)
            .unwrap_or_else(|| panic!("missing {}", cell.name));
        assert!(
            (width - cell.width).abs() < 1e-6 && (length - cell.length).abs() < 1e-6,
            "{} width/length",
            cell.name
        );
        assert_eq!(ring.len(), cell.ring.len());
        for (p, q) in ring.iter().zip(&cell.ring) {
            assert!(
                (p[0] - q[0]).abs() <= 1e-3 && (p[1] - q[1]).abs() <= 1e-3,
                "{} vertex",
                cell.name
            );
        }
    }
    let layers = build_paint_layers(&app.survey_tables, CrsCode::from(app.work_crs));
    let order = paint_order(&app.left_layers.entries, &layers);
    let painted = order
        .iter()
        .filter_map(|&i| layers.get(i))
        .find(|l| l.table == "trial_trench")
        .expect("legend draws trial_trench");
    assert_eq!(painted.features.len(), cells.len());
    let row = app
        .left_layers
        .entries
        .iter()
        .find(|e| e.layer_key == "trial_trench")
        .unwrap();
    assert_eq!(
        (row.label_ko.as_str(), row.feature_count),
        ("시굴격자", cells.len())
    );
}

fn exterior(wkt: &str) -> Vec<[f64; 2]> {
    let parts = strata_storage::gpkg_wkt::wkt_to_parts(wkt).expect("wkt");
    let WkbGeometry::Polygon(rings) = parts.into_iter().next().expect("part") else {
        panic!("not a polygon")
    };
    rings.into_iter().next().expect("ring")
}
