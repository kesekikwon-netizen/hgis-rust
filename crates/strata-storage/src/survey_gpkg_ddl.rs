//! 새 조사 파일(GPKG)의 표 정의·좌표계 문자열 — C++ 이 `QgsVectorFileWriter`(GDAL GPKG
//! 드라이버)로 쓰는 SQL 그대로(`SurveyProjectFactory.cpp:72-111`). GDAL 3.14 로 만든 같은
//! 조사 파일과 `samples/demo_survey/demo.gpkg` 에 맞췄다(시험: `survey_gpkg_create_tests`).

/// 조사 레이어 한 항목. `real` 이면 REAL, 아니면 TEXT(C++ `kReal`/`kText`).
#[derive(Debug, Clone, Copy)]
pub struct SurveyFieldDef {
    pub name: &'static str,
    pub real: bool,
}

/// 조사 레이어 하나 — 표 이름(layer_key), GPKG 도형 형식, 항목(순서 그대로).
#[derive(Debug, Clone, Copy)]
pub struct SurveyLayerDef {
    pub id: &'static str,
    pub geometry: &'static str,
    pub fields: &'static [SurveyFieldDef],
}

const fn t(name: &'static str) -> SurveyFieldDef {
    SurveyFieldDef { name, real: false }
}

const fn r(name: &'static str) -> SurveyFieldDef {
    SurveyFieldDef { name, real: true }
}

/// C++ `SurveySchema.cpp:28-93` buildLayers — 일곱 조사 레이어(감사 항목 uid·created_at·
/// updated_at 은 시굴격자만 없다).
pub const SURVEY_LAYERS: [SurveyLayerDef; 7] = [
    SurveyLayerDef { id: "survey_area", geometry: "POLYGON", fields: &[
        t("survey_name"), t("site_name"), t("note"), t("uid"), t("created_at"), t("updated_at"),
    ] },
    SurveyLayerDef { id: "feature_poly", geometry: "POLYGON", fields: &[
        t("kind"), t("period"), t("feature_no"), t("note"), t("layer_no"), r("depth_m"), r("top_el"),
        r("bottom_el"), t("relation"), t("status"), t("photo"), t("surveyor"), t("surv_date"),
        t("uid"), t("created_at"), t("updated_at"),
    ] },
    SurveyLayerDef { id: "feature_line", geometry: "LINESTRING", fields: &[
        t("kind"), t("period"), t("note"), t("feature_no"), t("layer_no"), t("relation"), t("status"),
        t("photo"), t("surveyor"), t("surv_date"), t("uid"), t("created_at"), t("updated_at"),
    ] },
    SurveyLayerDef { id: "section_line", geometry: "LINESTRING", fields: &[
        t("section_id"), t("note"), t("surveyor"), t("surv_date"), t("uid"), t("created_at"),
        t("updated_at"),
    ] },
    SurveyLayerDef { id: "control_points", geometry: "POINT", fields: &[
        t("point_id"), r("x"), r("y"), r("z"), t("datum"), t("ellipsoid"), t("projection"),
        t("origin"), t("accuracy"), r("accuracy_m"), r("pdop"), t("fix_type"), r("pixel_x"),
        r("pixel_y"), t("uid"), t("created_at"), t("updated_at"),
    ] },
    SurveyLayerDef { id: "artifact_point", geometry: "POINT", fields: &[
        t("kind"), t("period"), t("artifact_no"), t("note"), t("in_feature"), r("z"), t("layer_no"),
        t("photo"), t("surveyor"), t("surv_date"), t("uid"), t("created_at"), t("updated_at"),
    ] },
    SurveyLayerDef { id: "trial_trench", geometry: "POLYGON", fields: &[
        t("name"), r("width"), r("length"),
    ] },
];

/// C++ `SurveySchema.h:22-23` — GDAL 데이터셋 메타데이터로 쓰는 판 번호.
pub const SURVEY_SCHEMA_VERSION: u32 = 3;

pub(crate) const WKT_4326: &str = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4326"]]"#;
pub(crate) const WKT_5186: &str = r#"PROJCS["KGD2002 / Central Belt 2010",GEOGCS["KGD2002",DATUM["Korean_Geodetic_Datum_2002",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","6737"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4737"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",38],PARAMETER["central_meridian",127],PARAMETER["scale_factor",1],PARAMETER["false_easting",200000],PARAMETER["false_northing",600000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Northing",NORTH],AXIS["Easting",EAST],AUTHORITY["EPSG","5186"]]"#;
pub(crate) const WKT_5187: &str = r#"PROJCS["KGD2002 / East Belt 2010",GEOGCS["KGD2002",DATUM["Korean_Geodetic_Datum_2002",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","6737"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4737"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",38],PARAMETER["central_meridian",129],PARAMETER["scale_factor",1],PARAMETER["false_easting",200000],PARAMETER["false_northing",600000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Northing",NORTH],AXIS["Easting",EAST],AUTHORITY["EPSG","5187"]]"#;

/// GDAL 이 새 GPKG 를 열 때 쓰는 기본 표·좌표계 세 줄·타일 표 트리거(순서 그대로).
pub(crate) fn gpkg_base_sql() -> String {
    let mut s = String::from(
        "CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL,srs_id INTEGER NOT NULL PRIMARY KEY,organization TEXT NOT NULL,organization_coordsys_id INTEGER NOT NULL,definition  TEXT NOT NULL,description TEXT);
INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined Cartesian SRS', -1, 'NONE', -1, 'undefined', 'undefined Cartesian coordinate reference system');
INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', 'undefined geographic coordinate reference system');
",
    );
    s.push_str(&format!(
        "INSERT INTO gpkg_spatial_ref_sys VALUES ('WGS 84 geodetic', 4326, 'EPSG', 4326, '{WKT_4326}', 'longitude/latitude coordinates in decimal degrees on the WGS 84 spheroid');\n"
    ));
    s.push_str(
        "CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY,data_type TEXT NOT NULL,identifier TEXT UNIQUE,description TEXT DEFAULT '',last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),min_x DOUBLE, min_y DOUBLE,max_x DOUBLE, max_y DOUBLE,srs_id INTEGER,CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id));
CREATE TABLE gpkg_ogr_contents(table_name TEXT NOT NULL PRIMARY KEY,feature_count INTEGER DEFAULT NULL);
CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL,column_name TEXT NOT NULL,geometry_type_name TEXT NOT NULL,srs_id INTEGER NOT NULL,z TINYINT NOT NULL,m TINYINT NOT NULL,CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name),CONSTRAINT uk_gc_table_name UNIQUE (table_name),CONSTRAINT fk_gc_tn FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),CONSTRAINT fk_gc_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
CREATE TABLE gpkg_tile_matrix_set (table_name TEXT NOT NULL PRIMARY KEY,srs_id INTEGER NOT NULL,min_x DOUBLE NOT NULL,min_y DOUBLE NOT NULL,max_x DOUBLE NOT NULL,max_y DOUBLE NOT NULL,CONSTRAINT fk_gtms_table_name FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),CONSTRAINT fk_gtms_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
CREATE TABLE gpkg_tile_matrix (table_name TEXT NOT NULL,zoom_level INTEGER NOT NULL,matrix_width INTEGER NOT NULL,matrix_height INTEGER NOT NULL,tile_width INTEGER NOT NULL,tile_height INTEGER NOT NULL,pixel_x_size DOUBLE NOT NULL,pixel_y_size DOUBLE NOT NULL,CONSTRAINT pk_ttm PRIMARY KEY (table_name, zoom_level),CONSTRAINT fk_tmm_table_name FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name));
CREATE TRIGGER 'gpkg_tile_matrix_zoom_level_insert' BEFORE INSERT ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'insert on table ''gpkg_tile_matrix'' violates constraint: zoom_level cannot be less than 0') WHERE (NEW.zoom_level < 0); END;
CREATE TRIGGER 'gpkg_tile_matrix_zoom_level_update' BEFORE UPDATE of zoom_level ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'update on table ''gpkg_tile_matrix'' violates constraint: zoom_level cannot be less than 0') WHERE (NEW.zoom_level < 0); END;
CREATE TRIGGER 'gpkg_tile_matrix_matrix_width_insert' BEFORE INSERT ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'insert on table ''gpkg_tile_matrix'' violates constraint: matrix_width cannot be less than 1') WHERE (NEW.matrix_width < 1); END;
CREATE TRIGGER 'gpkg_tile_matrix_matrix_width_update' BEFORE UPDATE OF matrix_width ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'update on table ''gpkg_tile_matrix'' violates constraint: matrix_width cannot be less than 1') WHERE (NEW.matrix_width < 1); END;
CREATE TRIGGER 'gpkg_tile_matrix_matrix_height_insert' BEFORE INSERT ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'insert on table ''gpkg_tile_matrix'' violates constraint: matrix_height cannot be less than 1') WHERE (NEW.matrix_height < 1); END;
CREATE TRIGGER 'gpkg_tile_matrix_matrix_height_update' BEFORE UPDATE OF matrix_height ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'update on table ''gpkg_tile_matrix'' violates constraint: matrix_height cannot be less than 1') WHERE (NEW.matrix_height < 1); END;
CREATE TRIGGER 'gpkg_tile_matrix_pixel_x_size_insert' BEFORE INSERT ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'insert on table ''gpkg_tile_matrix'' violates constraint: pixel_x_size must be greater than 0') WHERE NOT (NEW.pixel_x_size > 0); END;
CREATE TRIGGER 'gpkg_tile_matrix_pixel_x_size_update' BEFORE UPDATE OF pixel_x_size ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'update on table ''gpkg_tile_matrix'' violates constraint: pixel_x_size must be greater than 0') WHERE NOT (NEW.pixel_x_size > 0); END;
CREATE TRIGGER 'gpkg_tile_matrix_pixel_y_size_insert' BEFORE INSERT ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'insert on table ''gpkg_tile_matrix'' violates constraint: pixel_y_size must be greater than 0') WHERE NOT (NEW.pixel_y_size > 0); END;
CREATE TRIGGER 'gpkg_tile_matrix_pixel_y_size_update' BEFORE UPDATE OF pixel_y_size ON 'gpkg_tile_matrix' FOR EACH ROW BEGIN SELECT RAISE(ABORT, 'update on table ''gpkg_tile_matrix'' violates constraint: pixel_y_size must be greater than 0') WHERE NOT (NEW.pixel_y_size > 0); END;
",
    );
    s
}

/// GDAL 이 첫 공간 색인을 만들 때 여는 확장 표.
pub(crate) const GPKG_EXTENSIONS_DDL: &str = "CREATE TABLE gpkg_extensions (table_name TEXT,column_name TEXT,extension_name TEXT NOT NULL,definition TEXT NOT NULL,scope TEXT NOT NULL,CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));\n";

/// GDAL 이 데이터셋 메타데이터를 쓸 때 만드는 두 표.
pub(crate) const GPKG_METADATA_DDL: &str = "CREATE TABLE gpkg_metadata (id INTEGER CONSTRAINT m_pk PRIMARY KEY ASC NOT NULL,md_scope TEXT NOT NULL DEFAULT 'dataset',md_standard_uri TEXT NOT NULL,mime_type TEXT NOT NULL DEFAULT 'text/xml',metadata TEXT NOT NULL DEFAULT '');
CREATE TABLE gpkg_metadata_reference (reference_scope TEXT NOT NULL,table_name TEXT,column_name TEXT,row_id_value INTEGER,timestamp DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),md_file_id INTEGER NOT NULL,md_parent_id INTEGER,CONSTRAINT crmr_mfi_fk FOREIGN KEY (md_file_id) REFERENCES gpkg_metadata(id),CONSTRAINT crmr_mpi_fk FOREIGN KEY (md_parent_id) REFERENCES gpkg_metadata(id));
";

/// 레이어 표 하나와 GDAL 공간 색인(rtree)·트리거 아홉 개의 SQL(GDAL 이 만드는 순서 그대로).
pub(crate) fn layer_sql(layer: &SurveyLayerDef, srs_id: u32, first: bool) -> String {
    let id = layer.id;
    let mut cols = String::new();
    for f in layer.fields {
        cols.push_str(&format!(r#", "{}" {}"#, f.name, if f.real { "REAL" } else { "TEXT" }));
    }
    let mut s = format!(
        r#"CREATE TABLE "{id}" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" {}{cols});
INSERT INTO gpkg_contents (table_name, data_type, identifier, description, srs_id) VALUES ('{id}', 'features', '{id}', '', {srs_id});
INSERT INTO gpkg_geometry_columns VALUES ('{id}', 'geom', '{}', {srs_id}, 0, 0);
INSERT INTO gpkg_ogr_contents VALUES ('{id}', 0);
"#,
        layer.geometry, layer.geometry
    );
    if first {
        s.push_str(GPKG_EXTENSIONS_DDL);
    }
    let g = format!("rtree_{id}_geom");
    let ins = r#"VALUES (NEW."fid",ST_MinX(NEW."geom"), ST_MaxX(NEW."geom"),ST_MinY(NEW."geom"), ST_MaxY(NEW."geom"));"#;
    s.push_str(&format!(
        r#"INSERT INTO gpkg_extensions VALUES ('{id}', 'geom', 'gpkg_rtree_index', 'http://www.geopackage.org/spec120/#extension_rtree', 'write-only');
CREATE VIRTUAL TABLE "{g}" USING rtree(id, minx, maxx, miny, maxy);
CREATE TRIGGER "{g}_insert" AFTER INSERT ON "{id}" WHEN (new."geom" NOT NULL AND NOT ST_IsEmpty(NEW."geom")) BEGIN INSERT OR REPLACE INTO "{g}" {ins} END;
CREATE TRIGGER "{g}_update6" AFTER UPDATE OF "geom" ON "{id}" WHEN OLD."fid" = NEW."fid" AND (NEW."geom" NOTNULL AND NOT ST_IsEmpty(NEW."geom")) AND (OLD."geom" NOTNULL AND NOT ST_IsEmpty(OLD."geom")) BEGIN UPDATE "{g}" SET minx = ST_MinX(NEW."geom"), maxx = ST_MaxX(NEW."geom"),miny = ST_MinY(NEW."geom"), maxy = ST_MaxY(NEW."geom") WHERE id = NEW."fid";END;
CREATE TRIGGER "{g}_update7" AFTER UPDATE OF "geom" ON "{id}" WHEN OLD."fid" = NEW."fid" AND (NEW."geom" NOTNULL AND NOT ST_IsEmpty(NEW."geom")) AND (OLD."geom" ISNULL OR ST_IsEmpty(OLD."geom")) BEGIN INSERT INTO "{g}" {ins} END;
CREATE TRIGGER "{g}_update2" AFTER UPDATE OF "geom" ON "{id}" WHEN OLD."fid" = NEW."fid" AND (NEW."geom" ISNULL OR ST_IsEmpty(NEW."geom")) BEGIN DELETE FROM "{g}" WHERE id = OLD."fid"; END;
CREATE TRIGGER "{g}_update5" AFTER UPDATE ON "{id}" WHEN OLD."fid" != NEW."fid" AND (NEW."geom" NOTNULL AND NOT ST_IsEmpty(NEW."geom")) BEGIN DELETE FROM "{g}" WHERE id = OLD."fid"; INSERT OR REPLACE INTO "{g}" {ins} END;
CREATE TRIGGER "{g}_update4" AFTER UPDATE ON "{id}" WHEN OLD."fid" != NEW."fid" AND (NEW."geom" ISNULL OR ST_IsEmpty(NEW."geom")) BEGIN DELETE FROM "{g}" WHERE id IN (OLD."fid", NEW."fid"); END;
CREATE TRIGGER "{g}_delete" AFTER DELETE ON "{id}" WHEN old."geom" NOT NULL BEGIN DELETE FROM "{g}" WHERE id = OLD."fid"; END;
CREATE TRIGGER "trigger_insert_feature_count_{id}" AFTER INSERT ON "{id}" BEGIN UPDATE gpkg_ogr_contents SET feature_count = feature_count + 1 WHERE lower(table_name) = lower('{id}'); END;
CREATE TRIGGER "trigger_delete_feature_count_{id}" AFTER DELETE ON "{id}" BEGIN UPDATE gpkg_ogr_contents SET feature_count = feature_count - 1 WHERE lower(table_name) = lower('{id}'); END;
"#
    ));
    s
}
