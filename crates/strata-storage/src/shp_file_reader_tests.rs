//! SHP 읽기 시험.

use super::*;
use crate::shp_file_writer::{write_shapefile, ShpField, ShpKind, ShpRecord};
use crate::survey_doc::FieldValue;
use crate::wkb_codec::WkbGeometry;

#[test]
fn what_the_writer_writes_the_reader_reads_back_with_korean_text() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let base = dir.path().join("유구");
    // 구멍이 있는 면 하나.
    let outer = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]];
    let hole = vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]];
    let fields = vec![
        ShpField { name: "kind".to_owned(), numeric: false },
        ShpField { name: "depth_m".to_owned(), numeric: true },
    ];
    let records = vec![ShpRecord {
        geom: WkbGeometry::Polygon(vec![outer, hole]),
        values: vec![FieldValue::Text("주거지".to_owned()), FieldValue::Real(1.25)],
    }];
    write_shapefile(&base, ShpKind::Polygon, &fields, &records, "PRJ")?;

    let ds = read_shapefile(&base.with_extension("shp"))?;
    assert_eq!((ds.shape_type, ds.shapes.len(), ds.prj_wkt.as_str()), (5, 1, "PRJ"));
    assert_eq!(ds.fields, vec!["kind".to_owned(), "depth_m".to_owned()]);
    let shape = &ds.shapes[0];
    assert_eq!(shape.parts.len(), 2, "바깥 고리와 구멍");
    assert_eq!(shape.parts[0].len(), 5);
    assert_eq!(shape.attrs["kind"], "주거지");
    assert!((shape.attrs["depth_m"].parse::<f64>().unwrap_or(0.0) - 1.25).abs() < 1e-12);
    Ok(())
}

#[test]
fn cp949_attributes_without_a_cpg_file_are_read_as_korean() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let base = dir.path().join("jijeok");
    let records = vec![ShpRecord {
        geom: WkbGeometry::Point([200_000.0, 450_000.0]),
        values: vec![FieldValue::Text(String::new())],
    }];
    write_shapefile(&base, ShpKind::Point, &[ShpField { name: "JIBUN".to_owned(), numeric: false }], &records, "")?;
    // 국토부 지적도처럼: .cpg 가 없고 값이 CP949 로 적힌 DBF.
    std::fs::remove_file(base.with_extension("cpg"))?;
    let dbf_path = base.with_extension("dbf");
    let mut dbf = std::fs::read(&dbf_path)?;
    let header_len = u16::from_le_bytes([dbf[8], dbf[9]]) as usize;
    let (cp949, _, _) = encoding_rs::EUC_KR.encode("산81 임");
    dbf[header_len + 1..header_len + 1 + cp949.len()].copy_from_slice(&cp949);
    std::fs::write(&dbf_path, dbf)?;

    let ds = read_shapefile(&base.with_extension("shp"))?;
    assert_eq!(ds.shapes[0].attrs["JIBUN"], "산81 임");
    assert_eq!(ds.shapes[0].parts, vec![vec![[200_000.0, 450_000.0]]]);
    Ok(())
}

#[test]
fn reading_in_a_box_keeps_only_shapes_that_touch_it_with_their_own_attributes() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let base = dir.path().join("parcels");
    let square = |x: f64| {
        WkbGeometry::Polygon(vec![vec![[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0], [x, 0.0]]])
    };
    let records: Vec<ShpRecord> = (0..5)
        .map(|i| ShpRecord {
            geom: square(f64::from(i) * 100.0),
            values: vec![FieldValue::Text(format!("{i}번지"))],
        })
        .collect();
    write_shapefile(&base, ShpKind::Polygon, &[ShpField { name: "JIBUN".to_owned(), numeric: false }], &records, "")?;

    // x 195~305 범위: 2번(200~210)과 3번(300~310)만 걸친다.
    let ds = read_shapefile_in_box(&base.with_extension("shp"), Some([195.0, -5.0, 305.0, 5.0]))?;
    let labels: Vec<&str> = ds.shapes.iter().map(|s| s.attrs["JIBUN"].as_str()).collect();
    assert_eq!(labels, vec!["2번지", "3번지"], "거른 뒤에도 속성이 제 도형을 따라간다");
    assert_eq!(read_shapefile(&base.with_extension("shp"))?.shapes.len(), 5);
    Ok(())
}

#[test]
fn a_file_that_is_not_a_shapefile_is_refused() {
    let dir = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
    let path = dir.path().join("x.shp");
    let _ = std::fs::write(&path, vec![0_u8; 200]);
    assert!(read_shapefile(&path).is_err());
}
