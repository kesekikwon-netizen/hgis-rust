//! `read_shapefile` 왕복 시험 — 점/선/폴리곤, 한글 CP949, `.cpg` UTF-8, extent.
//!
//! 모든 자료는 `shapefile` 크레이트 writer 로 **실제 규격 파일**을 만들어 읽는다.
//! (`shp_writer::write_shapefile_utf8` 은 `.shx`/`.dbf` 가 4바이트 스텁이라 쓰지 않는다.)

use std::collections::BTreeMap;

use shapefile::dbase::FieldValue;
use strata_types::geom::{Extent, Geometry};

use super::test_support::{
    point_layer, record, text_record, utf8_table, write_shapefile, FieldSpec, TempShpDir,
};
use super::{read_shapefile, ShpField, ShpFieldKind};

#[test]
fn reads_point_layer_with_fields_extent_and_index() -> Result<(), String> {
    let dir = TempShpDir::new("point");
    let base = dir.base("지점");
    point_layer(
        &base,
        &[[219_800.0, 361_500.0], [219_900.0, 361_600.0], [219_850.0, 361_450.0]],
        &["A", "B", "C"],
        utf8_table,
    )?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(layer.fields.len(), 1);
    assert_eq!(
        layer.fields[0],
        ShpField {
            name: "NAME".to_owned(),
            kind: ShpFieldKind::Text,
        }
    );
    assert_eq!(layer.features.len(), 3);
    // index 는 0 부터 차례로 매겨진다.
    assert_eq!(layer.features[0].index, 0);
    assert_eq!(layer.features[2].index, 2);
    assert_eq!(
        layer.features[0].geometry,
        Some(Geometry::Point([219_800.0, 361_500.0]))
    );
    assert_eq!(
        layer.features[1].values,
        vec![Some("B".to_owned())],
        "값은 필드 순서대로"
    );
    assert_eq!(
        layer.features[1].attributes,
        BTreeMap::from([("NAME".to_owned(), "B".to_owned())])
    );
    // extent 는 모든 점을 감싼다.
    assert_eq!(layer.extent, Some(Extent::new(219_800.0, 361_450.0, 219_900.0, 361_600.0)));
    Ok(())
}

#[test]
fn reads_polyline_layer_as_line_string() -> Result<(), String> {
    let dir = TempShpDir::new("line");
    let base = dir.base("선");
    let specs = vec![FieldSpec::text("NAME", 40)];
    let shapes = vec![shapefile::Polyline::new(vec![
        shapefile::Point::new(200_000.0, 500_000.0),
        shapefile::Point::new(200_100.0, 500_050.0),
        shapefile::Point::new(200_200.0, 500_120.0),
    ])];
    let records = vec![record(&specs, &[FieldValue::Character(Some("조사선".to_owned()))])];
    write_shapefile(&base, &shapes, utf8_table(&specs), &records)?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(layer.features.len(), 1);
    match layer.features[0].geometry.as_ref() {
        Some(Geometry::LineString(pts)) => {
            assert_eq!(pts.len(), 3);
            assert_eq!(pts[0], [200_000.0, 500_000.0]);
            assert_eq!(pts[2], [200_200.0, 500_120.0]);
        }
        other => return Err(format!("LineString 이 아니다: {other:?}")),
    }
    assert_eq!(
        layer.extent,
        Some(Extent::new(200_000.0, 500_000.0, 200_200.0, 500_120.0))
    );
    Ok(())
}

#[test]
fn reads_polygon_layer_with_hole_as_polygon_rings() -> Result<(), String> {
    let dir = TempShpDir::new("polygon");
    let base = dir.base("면");
    let specs = vec![FieldSpec::text("NAME", 40)];
    // 외곽은 시계방향, 구멍은 반시계방향이어야 SHP 규격대로다.
    let outer = vec![
        shapefile::Point::new(0.0, 0.0),
        shapefile::Point::new(0.0, 100.0),
        shapefile::Point::new(100.0, 100.0),
        shapefile::Point::new(100.0, 0.0),
        shapefile::Point::new(0.0, 0.0),
    ];
    let hole = vec![
        shapefile::Point::new(40.0, 40.0),
        shapefile::Point::new(60.0, 40.0),
        shapefile::Point::new(60.0, 60.0),
        shapefile::Point::new(40.0, 60.0),
        shapefile::Point::new(40.0, 40.0),
    ];
    let shapes = vec![shapefile::Polygon::with_rings(vec![
        shapefile::PolygonRing::Outer(outer),
        shapefile::PolygonRing::Inner(hole),
    ])];
    let records = vec![record(&specs, &[FieldValue::Character(Some("1호 유구".to_owned()))])];
    write_shapefile(&base, &shapes, utf8_table(&specs), &records)?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    match layer.features[0].geometry.as_ref() {
        Some(Geometry::Polygon {
            exterior,
            interiors,
        }) => {
            assert_eq!(exterior.len(), 5, "외곽 링 5점(닫힘 포함)");
            assert_eq!(interiors.len(), 1, "구멍 하나");
            assert_eq!(interiors[0].len(), 5);
            // 규격이 외곽을 시계방향으로 정렬한다.
            let signed: f64 = exterior
                .windows(2)
                .map(|w| (w[1][0] - w[0][0]) * (w[1][1] + w[0][1]))
                .sum();
            assert!(signed > 0.0, "외곽은 시계방향(부호 양수)이어야 한다");
        }
        other => return Err(format!("Polygon 이 아니다: {other:?}")),
    }
    assert_eq!(
        layer.extent,
        Some(Extent::new(0.0, 0.0, 100.0, 100.0)),
        "extent 는 구멍보다 바깥 외곽을 따른다"
    );
    Ok(())
}

#[test]
fn extent_covers_every_vertex_of_multipart_geometry() -> Result<(), String> {
    // 여러 조각 폴리라인은 L0 에 MultiLineString 이 없어 첫 조각만 geometry 로 담지만,
    // extent 는 모든 조각의 꼭짓점을 포함해야 한다(레이어 범위가 잘리면 안 된다).
    let dir = TempShpDir::new("extent");
    let base = dir.base("다중선");
    let specs = vec![FieldSpec::text("NAME", 20)];
    let shapes = vec![shapefile::Polyline::with_parts(vec![
        vec![
            shapefile::Point::new(0.0, 0.0),
            shapefile::Point::new(10.0, 10.0),
        ],
        vec![
            shapefile::Point::new(500.0, 500.0),
            shapefile::Point::new(900.0, 700.0),
        ],
    ])];
    let records = vec![record(&specs, &[FieldValue::Character(Some("조각둘".to_owned()))])];
    write_shapefile(&base, &shapes, utf8_table(&specs), &records)?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(
        layer.extent,
        Some(Extent::new(0.0, 0.0, 900.0, 700.0)),
        "두 번째 조각(500..900)도 범위에 들어와야 한다"
    );
    Ok(())
}

#[test]
fn numeric_date_and_logical_fields_become_strings() -> Result<(), String> {
    let dir = TempShpDir::new("types");
    let base = dir.base("속성");
    let specs = vec![
        FieldSpec::text("NAME", 40),
        FieldSpec::number("NO", 10, 0),
        FieldSpec::number("ELEV", 12, 2),
        FieldSpec::date("SURVEYED"),
        FieldSpec::logical("CHECKED"),
    ];
    let shapes = vec![shapefile::Point::new(1.0, 2.0)];
    let records = vec![record(
        &specs,
        &[
            FieldValue::Character(Some("1호".to_owned())),
            FieldValue::Numeric(Some(17.0)),
            FieldValue::Numeric(Some(42.5)),
            FieldValue::Date(Some(
                shapefile::dbase::Date::new(3, 4, 2026).map_err(|e| format!("{e:?}"))?,
            )),
            FieldValue::Logical(Some(true)),
        ],
    )];
    write_shapefile(&base, &shapes, utf8_table(&specs), &records)?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    let kinds: Vec<ShpFieldKind> = layer.fields.iter().map(|f| f.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ShpFieldKind::Text,
            ShpFieldKind::Integer,
            ShpFieldKind::Real,
            ShpFieldKind::Date,
            ShpFieldKind::Logical,
        ]
    );
    let f = &layer.features[0];
    assert_eq!(f.values[1].as_deref(), Some("17"));
    assert_eq!(f.values[2].as_deref(), Some("42.50"), "소수 자릿수 유지");
    assert_eq!(f.values[3].as_deref(), Some("2026-04-03"));
    assert_eq!(f.values[4].as_deref(), Some("T"));
    Ok(())
}

#[test]
fn reads_prj_into_epsg() -> Result<(), String> {
    let dir = TempShpDir::new("prj");
    let base = dir.base("좌표계");
    point_layer(&base, &[[1.0, 2.0]], &["A"], utf8_table)?;
    std::fs::write(base.with_extension("prj"), crate::shp_writer::EPSG_5179_PRJ_WKT)
        .map_err(|e| e.to_string())?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;
    assert_eq!(layer.epsg, Some(5179));
    Ok(())
}

#[test]
fn no_prj_leaves_epsg_none() -> Result<(), String> {
    let dir = TempShpDir::new("no-prj");
    let base = dir.base("좌표계없음");
    point_layer(&base, &[[1.0, 2.0]], &["A"], utf8_table)?;
    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;
    assert_eq!(layer.epsg, None);
    Ok(())
}

#[test]
fn base_path_may_name_the_shp_itself() -> Result<(), String> {
    let dir = TempShpDir::new("shp-path");
    let base = dir.base("지점");
    let (specs, rec) = text_record("NAME", "A");
    write_shapefile(
        &base,
        &[shapefile::Point::new(5.0, 6.0)],
        utf8_table(&specs),
        &[rec],
    )?;
    let layer = read_shapefile(&base.with_extension("shp")).map_err(|e| e.to_string())?;
    assert_eq!(layer.features.len(), 1);
    Ok(())
}
