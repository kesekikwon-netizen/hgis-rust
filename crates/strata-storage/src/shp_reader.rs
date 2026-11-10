//! Shapefile 리더 — `.shp` + `.dbf` (+ `.prj`, `.cpg`) 를 읽어 L0 타입으로 옮긴다.
//!
//! ## 왜 직접 파싱하지 않는가
//!
//! SHP/DBF 규격 파싱은 GeoRust 생태계의 [`shapefile`](https://georust.org) 크레이트(0.9)가
//! 이미 한다. 이 모듈은 그 결과를 `strata_types::geom` 의 L0 타입과 [`StrataError`] 로
//! 옮기는 일만 한다. 직접 파싱을 새로 만들면 8바이트 정렬·BCD 일자·삭제 플래그 같은
//! 자잘한 규칙에서 C++(OGR)과 판정이 갈라진다.
//!
//! ## C++ 대응
//!
//! | 여기 | C++ |
//! |---|---|
//! | [`read_shapefile`] | `D:\hgis\src\core\HeritageImport.cpp` `HeritageImport::utf8WorkingLayer` (OGR 로 SHP 를 열어 UTF-8 작업 자료를 만드는 자리) |
//! | [`read_shapefile`] 의 인코딩 | `D:\hgis\src\core\LayerOps.cpp` `LayerOps::prepareShapefileEncoding` |
//! | `shp_read_dbf` | `D:\hgis\src\core\CadastralEncoding.cpp` `CadastralEncoding::fallbackFor`·`detectDbfEncoding` |
//! | `shp_read_prj` | `D:\hgis\src\core\CadCrsHints.cpp` `CadCrsHints::fromPrj` |
//!
//! ## 알려진 한계
//!
//! L0 [`Geometry`] 에는 MultiLineString·MultiPoint 가 없다. 그래서 여러 조각 폴리라인은
//! **첫 조각만** `LineString` 으로 담고, 2점 이상 MultiPoint·Multipatch·NullShape 는
//! `geometry = None` 이 된다. 다만 레이어 `extent` 는 도형 종류와 상관없이 모든 꼭짓점으로
//! 계산하므로 범위는 이 경우에도 정확하다.

use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use strata_types::error::{StrataError, StrataResult};
use strata_types::geom::{Extent, Geometry};

use crate::shp_read_dbf;
use crate::shp_read_geom;
use crate::shp_read_prj;

/// DBF 필드 종류. C++ `QgsField::Type` 을 읽기용으로 줄인 것.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShpFieldKind {
    /// `C`(문자) · `M`(메모)
    Text,
    /// 소수 자릿수 0 인 `N`·`F`, FoxPro `I`
    Integer,
    /// 그 밖의 숫자(`N`·`F` 소수, `Y`·`B`)
    Real,
    /// `D`·`T` — 값은 `YYYY-MM-DD` 문자열
    Date,
    /// `L` — 값은 `T`/`F`
    Logical,
}

/// DBF 필드 하나. C++ `QgsField` 자리.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShpField {
    pub name: String,
    pub kind: ShpFieldKind,
}

/// SHP 점 좌표. C++ `QgsPointXY` 자리.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShpPoint {
    pub xy: [f64; 2],
}

impl ShpPoint {
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { xy: [x, y] }
    }

    /// SHP 점 타입(`Point`·`PointZ`·`PointM`)에서 곧바로 만든다.
    ///
    /// C++ `QgsPointXY(point.x(), point.y())` — `D:\hgis\src\core\CadDrawingStore.cpp`
    /// 가 CAD 점을 옮길 때 쓰는 것과 같은 자리.
    #[must_use]
    pub fn of<P: shapefile::record::traits::HasXY>(p: &P) -> Self {
        Self::new(p.x(), p.y())
    }
}

impl From<[f64; 2]> for ShpPoint {
    fn from(xy: [f64; 2]) -> Self {
        Self { xy }
    }
}

/// 피처 하나 — 도형 + 속성.
#[derive(Debug, Clone, PartialEq)]
pub struct ShpFeature {
    /// `.shp` 레코드 순번(0부터). C++ `QgsFeature::id` 자리.
    pub index: usize,
    /// 도형. NullShape·Multipatch·2점 이상 MultiPoint 는 `None`.
    pub geometry: Option<Geometry>,
    /// 필드 순서대로의 값. 빈 값은 `None`.
    pub values: Vec<Option<String>>,
    /// 값이 있는 필드만 담는다(빈 값은 빠진다).
    pub attributes: BTreeMap<String, String>,
}

/// 레이어 하나(파일 하나) — 필드 정의 + 피처 + 범위 + 좌표계.
#[derive(Debug, Clone, PartialEq)]
pub struct ShpLayer {
    pub fields: Vec<ShpField>,
    pub features: Vec<ShpFeature>,
    /// 모든 꼭짓점을 감싸는 범위. 피처가 없으면 `None`.
    pub extent: Option<Extent>,
    /// `.prj` 에서 읽어 추정한 EPSG 번호. 못 알아보면 `None`.
    pub epsg: Option<u32>,
}

/// `.shp` + `.dbf`(+`.prj`·`.cpg`)를 읽는다. 한글이 깨지지 않아야 한다.
///
/// `base` 는 `.shp` 자체여도 되고 확장자 없는 이름이어도 된다.
///
/// C++ `HeritageImport::utf8WorkingLayer`(`D:\hgis\src\core\HeritageImport.cpp`) 와
/// `LayerOps::prepareShapefileEncoding`(`D:\hgis\src\core\LayerOps.cpp`) 이 OGR 로 하던
/// "곁 파일 찾기 → 인코딩 결정 → 피처 읽기" 를 한 번에 한다.
pub fn read_shapefile(base: &Path) -> StrataResult<ShpLayer> {
    let shp_path = shp_file_of(base);
    if !shp_path.is_file() {
        return Err(StrataError::InvalidInput(format!(
            "SHP 파일이 없습니다: {}",
            shp_path.display()
        )));
    }
    let dbf_path = find_sibling(&shp_path, "dbf").ok_or_else(|| {
        StrataError::InvalidInput(format!(
            "SHP 옆에 .dbf 가 없습니다(속성 없이 도형만 읽는 길은 아직 없다): {}",
            shp_path.display()
        ))
    })?;

    let dbf_bytes = std::fs::read(&dbf_path)?;
    let encoding = shp_read_dbf::resolve_encoding(&shp_path, &dbf_bytes);
    let decimals = shp_read_dbf::field_decimals(&dbf_bytes);
    let dbase_reader = shp_read_dbf::open_dbf(dbf_bytes, encoding)?;
    let fields: Vec<ShpField> = dbase_reader
        .fields()
        .iter()
        .enumerate()
        .map(|(i, info)| ShpField {
            name: info.name().to_owned(),
            kind: shp_read_dbf::field_kind(info.field_type(), decimals_at(&decimals, i)),
        })
        .collect();

    let shape_reader = open_shapes(&shp_path)?;
    let mut reader = shapefile::Reader::new(shape_reader, dbase_reader);
    let mut features = Vec::new();
    let mut extent: Option<Extent> = None;
    for (index, item) in reader.iter_shapes_and_records().enumerate() {
        let (shape, record) = item.map_err(|e| {
            StrataError::Storage(format!(
                "{} 의 {}번째 피처를 읽지 못했습니다: {e}",
                shp_path.display(),
                index + 1
            ))
        })?;
        features.push(feature_of(index, &shape, &record, &fields, &decimals));
        if let Some(e) = shp_read_geom::shape_extent(&shape) {
            extent = Some(match extent {
                Some(acc) => acc.union(&e),
                None => e,
            });
        }
    }

    Ok(ShpLayer {
        fields,
        features,
        extent,
        epsg: shp_read_prj::read_shp_epsg(&shp_path),
    })
}

fn decimals_at(decimals: &[u8], index: usize) -> u8 {
    decimals.get(index).copied().unwrap_or(0)
}

/// 피처 하나를 만든다 — 도형 + 필드 순서대로의 값 + 이름→값 지도.
///
/// C++ `QgsFeature` 를 만드는 자리(`D:\hgis\src\core\HeritageImport.cpp`
/// `HeritageImport::utf8WorkingLayer` 가 OGR 로 피처를 읽어 옮기는 부분)에 해당한다.
fn feature_of(
    index: usize,
    shape: &shapefile::Shape,
    record: &shapefile::dbase::Record,
    fields: &[ShpField],
    decimals: &[u8],
) -> ShpFeature {
    let values: Vec<Option<String>> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            record
                .get(&f.name)
                .and_then(|v| shp_read_dbf::value_text(v, decimals_at(decimals, i)))
        })
        .collect();
    let mut attributes = BTreeMap::new();
    for (f, value) in fields.iter().zip(values.iter()) {
        if let Some(v) = value {
            attributes.insert(f.name.clone(), v.clone());
        }
    }
    ShpFeature {
        index,
        geometry: shp_read_geom::shape_geometry(shape),
        values,
        attributes,
    }
}

/// `.shp` 를 연다. `.shx` 색인이 깨져 있으면 색인 없이 순차 읽기로 돌아간다
/// (공개 배포 자료에 잘린 `.shx` 가 섞여 있다 — C++ OGR 도 없으면 순차로 읽는다).
///
/// C++ `HeritageImport::utf8WorkingLayer` 가 만드는 `QgsVectorLayer(shp, …, "ogr")` 와
/// 같은 단계다.
fn open_shapes(shp_path: &Path) -> StrataResult<shapefile::ShapeReader<BufReader<std::fs::File>>> {
    let open_shp = || std::fs::File::open(shp_path).map(BufReader::new);
    if let Some(shx) = find_sibling(shp_path, "shx") {
        if let Ok(index) = std::fs::File::open(&shx).map(BufReader::new) {
            if let Ok(reader) = shapefile::ShapeReader::with_shx(open_shp()?, index) {
                return Ok(reader);
            }
        }
    }
    let reader = shapefile::ShapeReader::new(open_shp()?).map_err(|e| {
        StrataError::Storage(format!("{} 를 SHP 로 열지 못했습니다: {e}", shp_path.display()))
    })?;
    Ok(reader)
}

/// `base` 가 `.shp` 를 가리키게 만든다. C++ `QFileInfo::completeBaseName` + `.shp` 와 같다.
fn shp_file_of(base: &Path) -> PathBuf {
    match base.extension() {
        Some(ext) if ext.eq_ignore_ascii_case("shp") => base.to_path_buf(),
        _ => base.with_extension("shp"),
    }
}

/// 같은 이름 + 확장자(대소문자 무관)인 곁 파일을 찾는다.
///
/// C++ `CadastralEncoding.cpp` 의 `sibling()` 과 같은 규칙이다. 한국 공공 자료는
/// `.SHP`/`.DBF` 대문자도 흔하고, 확장자만 다른 이름이 섞여 있어 이름으로만 짝을 짓는다.
pub(crate) fn find_sibling(shp: &Path, ext: &str) -> Option<PathBuf> {
    let stem = shp.file_stem()?;
    let parent = shp.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = parent.unwrap_or_else(|| Path::new("."));
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            let stem_matches = path
                .file_stem()
                .is_some_and(|s| s.eq_ignore_ascii_case(stem));
            let ext_matches = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(ext));
            stem_matches && ext_matches
        })
}

#[cfg(test)]
#[path = "shp_reader_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "shp_reader_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shp_reader_encoding_tests.rs"]
mod encoding_tests;

#[cfg(test)]
#[path = "shp_reader_error_tests.rs"]
mod error_tests;
