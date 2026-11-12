//! 새 조사 파일 형식 시험 — C++ `SurveyProjectFactory.cpp:72-170`(GDAL 이 쓰는 GPKG)과
//! 같은 구조인지 문자열·값 그대로 비교한다. 기준은 GDAL 3.14 로 만든 같은 조사 파일과
//! `samples/demo_survey/demo.gpkg`(임시 폴더 사본만 연다)이다.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};

use crate::gpkg::GpkgRepository;
use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::wkb_codec::{encode_gpkg_wkb, WkbGeometry};

/// GDAL 3.14 가 C++ `SurveySchema.cpp:28-93` 표로 만든 일곱 표의 DDL 그대로.
const LAYER_DDL: [(&str, &str); 7] = [
    ("survey_area", r#"CREATE TABLE "survey_area" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, "survey_name" TEXT, "site_name" TEXT, "note" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("feature_poly", r#"CREATE TABLE "feature_poly" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, "kind" TEXT, "period" TEXT, "feature_no" TEXT, "note" TEXT, "layer_no" TEXT, "depth_m" REAL, "top_el" REAL, "bottom_el" REAL, "relation" TEXT, "status" TEXT, "photo" TEXT, "surveyor" TEXT, "surv_date" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("feature_line", r#"CREATE TABLE "feature_line" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" LINESTRING, "kind" TEXT, "period" TEXT, "note" TEXT, "feature_no" TEXT, "layer_no" TEXT, "relation" TEXT, "status" TEXT, "photo" TEXT, "surveyor" TEXT, "surv_date" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("section_line", r#"CREATE TABLE "section_line" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" LINESTRING, "section_id" TEXT, "note" TEXT, "surveyor" TEXT, "surv_date" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("control_points", r#"CREATE TABLE "control_points" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POINT, "point_id" TEXT, "x" REAL, "y" REAL, "z" REAL, "datum" TEXT, "ellipsoid" TEXT, "projection" TEXT, "origin" TEXT, "accuracy" TEXT, "accuracy_m" REAL, "pdop" REAL, "fix_type" TEXT, "pixel_x" REAL, "pixel_y" REAL, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("artifact_point", r#"CREATE TABLE "artifact_point" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POINT, "kind" TEXT, "period" TEXT, "artifact_no" TEXT, "note" TEXT, "in_feature" TEXT, "z" REAL, "layer_no" TEXT, "photo" TEXT, "surveyor" TEXT, "surv_date" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#),
    ("trial_trench", r#"CREATE TABLE "trial_trench" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, "name" TEXT, "width" REAL, "length" REAL)"#),
];

/// GDAL 3.14 가 EPSG:5187 에 쓰는 정의(5186·4326 은 demo.gpkg 와 바이트 비교).
const WKT_5187: &str = r#"PROJCS["KGD2002 / East Belt 2010",GEOGCS["KGD2002",DATUM["Korean_Geodetic_Datum_2002",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","6737"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4737"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",38],PARAMETER["central_meridian",129],PARAMETER["scale_factor",1],PARAMETER["false_easting",200000],PARAMETER["false_northing",600000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Northing",NORTH],AXIS["Easting",EAST],AUTHORITY["EPSG","5187"]]"#;

/// GDAL `SetMetadataItem("KA_HGIS_SCHEMA_VERSION", "3")` 가 쓰는 바이트 그대로
/// (C++ `SurveySchema.cpp:146-162` writeVersion).
const STAMP_XML: &str = "<GDALMultiDomainMetadata>\n  <Metadata>\n    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">3</MDI>\n  </Metadata>\n</GDALMultiDomainMetadata>\n";

fn db(e: rusqlite::Error) -> StrataError {
    StrataError::Storage(e.to_string())
}

fn demo_copy(dir: &Path) -> StrataResult<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dst = dir.join("demo_copy.gpkg");
    std::fs::copy(src, &dst)?;
    Ok(dst)
}

fn rows(conn: &Connection, sql: &str) -> StrataResult<Vec<String>> {
    let mut stmt = conn.prepare(sql).map_err(db)?;
    let out = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db);
    out
}

fn one(conn: &Connection, sql: &str) -> StrataResult<String> {
    conn.query_row(sql, [], |r| r.get::<_, String>(0)).map_err(db)
}

/// GDAL 이 만드는 순서 그대로의 sqlite_master (type|name) 목록.
fn expected_master() -> Vec<String> {
    let mut v: Vec<String> = [
        "table|gpkg_spatial_ref_sys", "table|gpkg_contents", "index|sqlite_autoindex_gpkg_contents_1",
        "index|sqlite_autoindex_gpkg_contents_2", "table|gpkg_ogr_contents",
        "index|sqlite_autoindex_gpkg_ogr_contents_1", "table|gpkg_geometry_columns",
        "index|sqlite_autoindex_gpkg_geometry_columns_1", "index|sqlite_autoindex_gpkg_geometry_columns_2",
        "table|gpkg_tile_matrix_set", "index|sqlite_autoindex_gpkg_tile_matrix_set_1",
        "table|gpkg_tile_matrix", "index|sqlite_autoindex_gpkg_tile_matrix_1",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    for col in ["zoom_level", "matrix_width", "matrix_height", "pixel_x_size", "pixel_y_size"] {
        for op in ["insert", "update"] {
            v.push(format!("trigger|gpkg_tile_matrix_{col}_{op}"));
        }
    }
    for (i, (t, _)) in LAYER_DDL.iter().enumerate() {
        v.push(format!("table|{t}"));
        if i == 0 {
            v.push("table|sqlite_sequence".to_owned());
            v.push("table|gpkg_extensions".to_owned());
            v.push("index|sqlite_autoindex_gpkg_extensions_1".to_owned());
        }
        for suffix in ["", "_rowid", "_node", "_parent"] {
            v.push(format!("table|rtree_{t}_geom{suffix}"));
        }
        for trg in ["insert", "update6", "update7", "update2", "update5", "update4", "delete"] {
            v.push(format!("trigger|rtree_{t}_geom_{trg}"));
        }
        v.push(format!("trigger|trigger_insert_feature_count_{t}"));
        v.push(format!("trigger|trigger_delete_feature_count_{t}"));
    }
    v.push("table|gpkg_metadata".to_owned());
    v.push("table|gpkg_metadata_reference".to_owned());
    v
}

/// 새 조사(5186·5187)가 GDAL 이 만든 C++ 새 조사 파일과 같은 구조인지.
#[test]
fn new_survey_file_matches_cpp_gdal_survey_for_5186_and_5187() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let demo = Connection::open_with_flags(demo_copy(dir.path())?, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(db)?;
    let demo_def = |id: i32| one(&demo, &format!("SELECT definition FROM gpkg_spatial_ref_sys WHERE srs_id = {id}"));
    let demo_triggers = rows(
        &demo,
        "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'feature_poly' ORDER BY rowid",
    )?;

    for (crs, epsg, srs_name) in [
        (WorkCrs::Epsg5186, 5186, "KGD2002 / Central Belt 2010"),
        (WorkCrs::Epsg5187, 5187, "KGD2002 / East Belt 2010"),
    ] {
        let path = dir.path().join(format!("조사{epsg}.gpkg"));
        GpkgRepository::create_new_survey(&path, crs)?;
        let c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(db)?;

        assert_eq!(one(&c, "SELECT CAST(application_id AS TEXT) FROM pragma_application_id")?, "1196444487");
        assert_eq!(one(&c, "SELECT CAST(user_version AS TEXT) FROM pragma_user_version")?, "10400");
        assert_eq!(one(&c, "PRAGMA integrity_check")?, "ok");
        assert_eq!(rows(&c, "SELECT type || '|' || name FROM sqlite_master ORDER BY rowid")?, expected_master());

        for (t, ddl) in LAYER_DDL {
            let q = format!("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = '{t}'");
            assert_eq!(one(&c, &q)?, ddl);
            let q = format!("SELECT sql FROM sqlite_master WHERE name = 'rtree_{t}_geom'");
            assert_eq!(one(&c, &q)?, format!(r#"CREATE VIRTUAL TABLE "rtree_{t}_geom" USING rtree(id, minx, maxx, miny, maxy)"#));
            let q = format!("SELECT sql FROM sqlite_master WHERE type = 'trigger' AND tbl_name = '{t}' ORDER BY rowid");
            let want: Vec<String> = demo_triggers.iter().map(|s| s.replace("feature_poly", t)).collect();
            assert_eq!(rows(&c, &q)?, want, "triggers of {t}");
        }

        assert_eq!(
            rows(&c, "SELECT srs_id || '|' || quote(srs_name) || '|' || organization || '|' || organization_coordsys_id || '|' || quote(description) FROM gpkg_spatial_ref_sys ORDER BY srs_id")?,
            vec![
                "-1|'Undefined Cartesian SRS'|NONE|-1|'undefined Cartesian coordinate reference system'".to_owned(),
                "0|'Undefined geographic SRS'|NONE|0|'undefined geographic coordinate reference system'".to_owned(),
                "4326|'WGS 84 geodetic'|EPSG|4326|'longitude/latitude coordinates in decimal degrees on the WGS 84 spheroid'".to_owned(),
                format!("{epsg}|'{srs_name}'|EPSG|{epsg}|NULL"),
            ]
        );
        let def = |id: i32| one(&c, &format!("SELECT definition FROM gpkg_spatial_ref_sys WHERE srs_id = {id}"));
        assert_eq!(def(-1)?, "undefined");
        assert_eq!(def(0)?, "undefined");
        assert_eq!(def(4326)?, demo_def(4326)?);
        assert_eq!(def(epsg)?, if epsg == 5186 { demo_def(5186)? } else { WKT_5187.to_owned() });

        let names: Vec<&str> = LAYER_DDL.iter().map(|(t, _)| *t).collect();
        assert_eq!(
            rows(&c, "SELECT table_name || '|' || data_type || '|' || identifier || '|' || quote(description) || '|' || quote(min_x) || quote(min_y) || quote(max_x) || quote(max_y) || '|' || srs_id FROM gpkg_contents ORDER BY rowid")?,
            names.iter().map(|t| format!("{t}|features|{t}|''|NULLNULLNULLNULL|{epsg}")).collect::<Vec<_>>()
        );
        for stamp in rows(&c, "SELECT last_change FROM gpkg_contents")? {
            assert!(stamp.len() == 24 && stamp.as_bytes()[10] == b'T' && stamp.ends_with('Z'), "{stamp}");
        }
        let geom = |t: &str| if t.contains("line") { "LINESTRING" } else if t.contains("point") { "POINT" } else { "POLYGON" };
        assert_eq!(
            rows(&c, "SELECT table_name || '|' || column_name || '|' || geometry_type_name || '|' || srs_id || '|' || z || '|' || m FROM gpkg_geometry_columns ORDER BY rowid")?,
            names.iter().map(|t| format!("{t}|geom|{}|{epsg}|0|0", geom(t))).collect::<Vec<_>>()
        );
        assert_eq!(
            rows(&c, "SELECT table_name || '|' || feature_count FROM gpkg_ogr_contents ORDER BY rowid")?,
            names.iter().map(|t| format!("{t}|0")).collect::<Vec<_>>()
        );
        let mut ext: Vec<String> = names
            .iter()
            .map(|t| format!("'{t}'|'geom'|'gpkg_rtree_index'|'http://www.geopackage.org/spec120/#extension_rtree'|'write-only'"))
            .collect();
        for t in ["gpkg_metadata", "gpkg_metadata_reference"] {
            ext.push(format!("'{t}'|NULL|'gpkg_metadata'|'http://www.geopackage.org/spec120/#extension_metadata'|'read-write'"));
        }
        assert_eq!(
            rows(&c, "SELECT quote(table_name) || '|' || quote(column_name) || '|' || quote(extension_name) || '|' || quote(definition) || '|' || quote(scope) FROM gpkg_extensions ORDER BY rowid")?,
            ext
        );
        assert_eq!(
            rows(&c, "SELECT id || '|' || md_scope || '|' || md_standard_uri || '|' || mime_type FROM gpkg_metadata")?,
            vec!["1|dataset|http://gdal.org|text/xml".to_owned()]
        );
        assert_eq!(one(&c, "SELECT metadata FROM gpkg_metadata WHERE id = 1")?, STAMP_XML);
        assert_eq!(
            rows(&c, "SELECT reference_scope || '|' || quote(table_name) || quote(column_name) || quote(row_id_value) || '|' || md_file_id || '|' || quote(md_parent_id) FROM gpkg_metadata_reference")?,
            vec!["geopackage|NULLNULLNULL|1|NULL".to_owned()]
        );
        let ts = one(&c, "SELECT timestamp FROM gpkg_metadata_reference")?;
        assert!(ts.len() == 24 && ts.as_bytes()[10] == b'T' && ts.ends_with('Z'), "{ts}");
        assert_eq!(one(&c, "SELECT CAST(count(*) AS TEXT) FROM sqlite_sequence")?, "0");
    }
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(".ka-new-survey-"))
        .collect();
    assert!(leftovers.is_empty(), "staging folders must be removed: {leftovers:?}");
    Ok(())
}

/// `data/schemas/ka_hgis_layers.yaml`(C++ 과 같은 파일)의 레이어·도형·항목·형식·순서가
/// 새 조사 표 DDL 과 같은지.
#[test]
fn layer_ddl_matches_the_schema_yaml_names_types_and_order() -> StrataResult<()> {
    let yaml = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schemas/ka_hgis_layers.yaml"),
    )?;
    let mut built: Vec<(String, String)> = Vec::new();
    for line in yaml.lines().map(str::trim) {
        if let Some(id) = line.strip_prefix("- id: ") {
            built.push((id.to_owned(), String::new()));
        } else if let (Some(g), Some((t, ddl))) = (line.strip_prefix("geometry: "), built.last_mut()) {
            *ddl = format!(r#"CREATE TABLE "{t}" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" {}"#, g.to_uppercase());
        } else if let (Some(field), Some((_, ddl))) = (line.strip_prefix("- {name: "), built.last_mut()) {
            let (name, rest) = field.split_once(", type: ").expect("field line has a type");
            let ty = if rest.starts_with("double") { "REAL" } else { "TEXT" };
            ddl.push_str(&format!(r#", "{name}" {ty}"#));
        }
    }
    for (_, ddl) in &mut built {
        ddl.push(')');
    }
    let want: Vec<(String, String)> =
        LAYER_DDL.iter().map(|(t, d)| ((*t).to_owned(), (*d).to_owned())).collect();
    assert_eq!(built, want);
    Ok(())
}

/// GDAL rtree 트리거는 ST_MinX·ST_IsEmpty 같은 함수를 부른다. Rust 연결에 등록하지 않으면
/// 도형을 넣지 못하고, 등록하면 rtree 에 바른 범위가 들어간다(새 조사·demo.gpkg 사본 둘 다).
#[test]
fn polygon_insert_fills_rtree_bbox_in_new_survey_and_demo_copy() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let fresh = dir.path().join("새조사.gpkg");
    GpkgRepository::create_new_survey(&fresh, WorkCrs::Epsg5187)?;
    let demo = demo_copy(dir.path())?;
    let ring = vec![[200_010.5, 600_020.25], [200_030.75, 600_020.25], [200_030.75, 600_045.5], [200_010.5, 600_020.25]];
    for (path, srs) in [(&fresh, 5187), (&demo, 5186)] {
        let conn = Connection::open(path).map_err(db)?;
        let blob = encode_gpkg_wkb(srs, &WkbGeometry::Polygon(vec![ring.clone()]));
        let insert = "INSERT INTO feature_poly (geom, kind) VALUES (?1, '주거지')";
        let err = conn.execute(insert, [&blob]).expect_err("GDAL triggers need ST_* functions");
        assert!(err.to_string().contains("no such function"), "{err}");

        register_gpkg_sql_functions(&conn).map_err(db)?;
        let count = || one(&conn, "SELECT CAST(feature_count AS TEXT) FROM gpkg_ogr_contents WHERE table_name = 'feature_poly'");
        let before: i64 = count()?.parse().map_err(|_| StrataError::Storage("count".into()))?;
        conn.execute(insert, [&blob]).map_err(db)?;
        let id = conn.last_insert_rowid();
        let bbox: [f64; 4] = conn
            .query_row("SELECT minx, maxx, miny, maxy FROM rtree_feature_poly_geom WHERE id = ?1", [id], |r| {
                Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?])
            })
            .map_err(db)?;
        // rtree 는 32비트 실수라 바깥쪽으로 둥근다.
        assert!(bbox[0] <= 200_010.5 && bbox[1] >= 200_030.75 && bbox[2] <= 600_020.25 && bbox[3] >= 600_045.5, "{bbox:?}");
        for (got, want) in bbox.iter().zip([200_010.5, 200_030.75, 600_020.25, 600_045.5]) {
            assert!((got - want).abs() < 0.05, "{bbox:?}");
        }
        assert_eq!(count()?, (before + 1).to_string());
        assert_eq!(one(&conn, "PRAGMA integrity_check")?, "ok");
    }
    Ok(())
}
