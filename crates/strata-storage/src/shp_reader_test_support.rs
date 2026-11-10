//! `shp_reader` 시험용 — 임시 폴더와 **실제 규격에 맞는** shapefile 작성기.
//!
//! ## 왜 `shp_writer::write_shapefile_utf8` 를 쓰지 않는가
//!
//! 그 함수는 `.shx`/`.dbf` 를 4바이트 스텁으로 쓴다(규격 위반). 왕복 시험은 읽는 쪽
//! 버그를 가릴 수 있으므로, 여기서는 GeoRust `shapefile` 크레이트의 writer 로 `.shp`+`.shx`+
//! `.dbf` 를 규격대로 만든다. 즉 시험 자료는 "우리가 읽는 코드"와 독립적이다.
//!
//! ## 임시 폴더
//!
//! Qt `QTemporaryDir` 대신 `std::env::temp_dir()` + 고유 이름을 쓰고,
//! [`TempShpDir`] 가 `Drop` 에서 지운다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use shapefile::dbase::{
    self, FieldName, FieldValue, TableWriterBuilder,
};

/// 고유 이름의 임시 폴더. 끝나면(스코프를 벗어나면) 통째로 지운다.
pub struct TempShpDir {
    path: PathBuf,
}

impl TempShpDir {
    pub fn new(tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "strata-shp-{tag}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("임시 폴더를 만들 수 있어야 한다");
        Self { path }
    }

    /// 폴더 안 파일 이름 충돌을 피하려고 시험마다 다른 줄기(`stem`)를 쓴다.
    pub fn base(&self, stem: &str) -> PathBuf {
        self.path.join(stem)
    }
}

impl Drop for TempShpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// DBF 필드 정의 하나.
pub struct FieldSpec {
    pub name: &'static str,
    pub kind: dbase::FieldType,
    pub length: u8,
    pub decimals: u8,
}

impl FieldSpec {
    pub const fn text(name: &'static str, length: u8) -> Self {
        Self {
            name,
            kind: dbase::FieldType::Character,
            length,
            decimals: 0,
        }
    }

    pub const fn number(name: &'static str, length: u8, decimals: u8) -> Self {
        Self {
            name,
            kind: dbase::FieldType::Numeric,
            length,
            decimals,
        }
    }

    pub const fn date(name: &'static str) -> Self {
        Self {
            name,
            kind: dbase::FieldType::Date,
            length: 8,
            decimals: 0,
        }
    }

    pub const fn logical(name: &'static str) -> Self {
        Self {
            name,
            kind: dbase::FieldType::Logical,
            length: 1,
            decimals: 0,
        }
    }
}

/// UTF-8(기본) DBF 쓰기 도구. `UnicodeLossy` 라 값이 UTF-8 바이트로 들어간다.
pub fn utf8_table(specs: &[FieldSpec]) -> TableWriterBuilder {
    apply_fields(TableWriterBuilder::new(), specs)
}

/// CP949(EUC-KR) DBF 쓰기 도구.
///
/// 한국 공공 SHP 처럼 `.cpg` 없이 CP949 바이트가 든 파일을 시험하려면 이걸 쓴다.
pub fn cp949_table(specs: &[FieldSpec]) -> TableWriterBuilder {
    let encoding =
        dbase::encoding::EncodingRs::from(dbase::encoding_rs::EUC_KR);
    apply_fields(TableWriterBuilder::with_encoding(encoding), specs)
}

fn apply_fields(mut builder: TableWriterBuilder, specs: &[FieldSpec]) -> TableWriterBuilder {
    for spec in specs {
        let name = FieldName::try_from(spec.name).expect("필드 이름은 11바이트 이하");
        builder = match spec.kind {
            dbase::FieldType::Date => builder.add_date_field(name),
            dbase::FieldType::Logical => builder.add_logical_field(name),
            dbase::FieldType::Numeric => {
                builder.add_numeric_field(name, spec.length, spec.decimals)
            }
            _ => builder.add_character_field(name, spec.length),
        };
    }
    builder
}

/// 레코드 하나를 만든다. 값 순서는 `specs` 순서와 같아야 한다.
pub fn record(specs: &[FieldSpec], values: &[FieldValue]) -> dbase::Record {
    let mut rec = dbase::Record::default();
    for (spec, value) in specs.iter().zip(values.iter()) {
        rec.insert(spec.name.to_owned(), value.clone());
    }
    rec
}

/// 규격에 맞는 `.shp`+`.shx`+`.dbf` 를 실제로 쓴다.
///
/// `shapes` 와 `records` 길이가 같아야 한다(짧은 쪽만큼 쓴다).
///
/// 주의: 크레이트의 `Writer::from_path` 는 준 경로를 **그대로** `.shp` 로 쓴다
/// (`.shp` 를 붙여 주지 않는다). 그래서 여기서 확장자를 먼저 맞춘다.
pub fn write_shapefile<S: shapefile::record::EsriShape>(
    base: &Path,
    shapes: &[S],
    builder: TableWriterBuilder,
    records: &[dbase::Record],
) -> Result<(), String> {
    let shp = if base
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("shp"))
    {
        base.to_path_buf()
    } else {
        base.with_extension("shp")
    };
    // writer 는 Drop 에서 헤더 bbox·레코드 수를 확정한다. 스코프를 닫아야 파일이 온전해진다.
    {
        let mut writer = shapefile::Writer::from_path(&shp, builder).map_err(|e| e.to_string())?;
        for (shape, rec) in shapes.iter().zip(records.iter()) {
            writer
                .write_shape_and_record(shape, rec)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 텍스트 속성 하나만 담는 편의 레코드.
pub fn text_record(name: &'static str, value: &str) -> (Vec<FieldSpec>, dbase::Record) {
    let specs = vec![FieldSpec::text(name, 80)];
    let rec = record(&specs, &[FieldValue::Character(Some(value.to_owned()))]);
    (specs, rec)
}

/// 점 shapefile 한 벌을 만든다(속성 `NAME` 하나).
pub fn point_layer(
    base: &Path,
    points: &[[f64; 2]],
    names: &[&str],
    table: impl Fn(&[FieldSpec]) -> TableWriterBuilder,
) -> Result<(), String> {
    let specs = vec![FieldSpec::text("NAME", 80)];
    let shapes: Vec<shapefile::Point> = points
        .iter()
        .map(|p| shapefile::Point::new(p[0], p[1]))
        .collect();
    let records: Vec<dbase::Record> = names
        .iter()
        .map(|n| record(&specs, &[FieldValue::Character(Some((*n).to_owned()))]))
        .collect();
    write_shapefile(base, &shapes, table(&specs), &records)
}
