//! 참조 레이어 — 받은 SHP(지적도·수치지형도·주변유적 등)를 읽어 지도에 올린다.
//!
//! C++ 은 `QgsVectorLayer(path, name, "ogr")` 로 올리고 QGIS 가 좌표계를 맞춰 그린다.
//! 여기서는 SHP 를 읽고 `.prj` 에서 좌표계를 알아낸 뒤 작업 좌표계로 바꿔 메모리에 든다.
//! 참조 레이어는 조사 데이터가 아니다: 조사 파일에 저장하지 않고, 고쳐 쓸 수 없다.

use std::path::{Path, PathBuf};

use strata_geo::crs_transform::CrsTransformer;
use strata_storage::shp_file_reader::read_shapefile;
use strata_types::crs::{CrsCode, WorkCrs};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;

/// 참조 도형 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceShape {
    /// 작업 좌표계로 바꾼 부분(고리·선·점)들.
    pub parts: Vec<Vec<[f64; 2]>>,
    /// 이름 속성(지번·유적명 등). 없으면 빈 글자.
    pub label: String,
}

/// 참조 레이어 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceLayer {
    pub name: String,
    pub source: PathBuf,
    /// 1 점, 3 선, 5 면.
    pub shape_type: i32,
    pub shapes: Vec<ReferenceShape>,
}

/// `.prj` 글자에서 좌표계를 알아낸다. 한국에서 쓰는 것만 본다. 모르면 `None`.
#[must_use]
pub fn detect_prj_crs(prj: &str) -> Option<CrsCode> {
    let upper = prj.to_ascii_uppercase();
    let param = |name: &str| -> Option<f64> {
        let at = upper.find(&format!("\"{name}\""))?;
        let rest = &upper[at + name.len() + 2..];
        let end = rest.find(']')?;
        rest[..end].trim_start_matches(',').trim().parse().ok()
    };
    if !upper.contains("PROJCS") {
        return upper.contains("GEOGCS").then_some(CrsCode::Epsg4326);
    }
    if upper.contains("MERCATOR_AUXILIARY_SPHERE") || upper.contains("PSEUDO-MERCATOR") {
        return Some(CrsCode::Epsg3857);
    }
    let meridian = param("CENTRAL_MERIDIAN")?;
    let false_n = param("FALSE_NORTHING")?;
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    if upper.contains("BESSEL") {
        return (near(meridian, 127.002_890_28) && near(false_n, 500_000.0)).then_some(CrsCode::Epsg5174);
    }
    if near(meridian, 127.0) && near(false_n, 600_000.0) {
        Some(CrsCode::Epsg5186)
    } else if near(meridian, 129.0) && near(false_n, 600_000.0) {
        Some(CrsCode::Epsg5187)
    } else if near(meridian, 127.5) && near(false_n, 2_000_000.0) {
        Some(CrsCode::Epsg5179)
    } else {
        None
    }
}

/// 이름으로 쓸 속성 열을 고른다(C++ `LayerOps::detectNameField` 의 유적·지적 후보 순서).
fn name_field(fields: &[String]) -> Option<&String> {
    const CANDIDATES: [&str; 12] = [
        "사업명", "유적명", "유적명칭", "명칭", "JIBUN", "지번", "A5", "NAME", "이름", "NAM", "name", "jibun",
    ];
    CANDIDATES
        .iter()
        .find_map(|c| fields.iter().find(|f| f.eq_ignore_ascii_case(c)))
}

/// SHP 를 읽어 작업 좌표계의 참조 레이어로 만든다. 돌려주는 글자는 사용자에게 보일 좌표계 설명.
pub fn load_reference_shp(path: &Path, work_crs: WorkCrs) -> Result<(ReferenceLayer, String), String> {
    let ds = read_shapefile(path).map_err(|e| format!("SHP 를 읽지 못했습니다: {e}"))?;
    let work = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    let source = detect_prj_crs(&ds.prj_wkt);
    let crs_note = match source {
        Some(crs) if crs == work => format!("{} (작업 좌표계와 같음)", crs.auth_id()),
        Some(crs) => format!("{} → {}", crs.auth_id(), work.auth_id()),
        None => format!("좌표계 정보 없음 — {} 로 보고 올렸습니다", work.auth_id()),
    };
    let transformer = CrsTransformer::new(source.unwrap_or(work), work);
    let label_field = name_field(&ds.fields).cloned();
    let shapes = ds
        .shapes
        .into_iter()
        .filter(|s| !s.parts.is_empty())
        .map(|s| ReferenceShape {
            parts: s
                .parts
                .iter()
                .map(|part| part.iter().map(|p| transformer.transform_xy(p[0], p[1])).collect())
                .collect(),
            label: label_field
                .as_ref()
                .and_then(|f| s.attrs.get(f))
                .cloned()
                .unwrap_or_default(),
        })
        .collect();
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("참조")
        .to_owned();
    Ok((
        ReferenceLayer {
            name,
            source: path.to_path_buf(),
            shape_type: ds.shape_type,
            shapes,
        },
        crs_note,
    ))
}

impl ReferenceLayer {
    /// 모든 도형을 감싸는 범위 `[min_x, min_y, max_x, max_y]`.
    #[must_use]
    pub fn extent(&self) -> Option<[f64; 4]> {
        let mut pts = self.shapes.iter().flat_map(|s| s.parts.iter().flatten());
        let first = pts.next()?;
        Some(pts.fold([first[0], first[1], first[0], first[1]], |b, p| {
            [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]
        }))
    }
}

impl StrataAppState {
    /// SHP 파일을 참조 레이어로 올린다. 올렸으면 `true`. 결과(또는 못 올린 이유)를 안내 줄에 남긴다.
    pub(crate) fn import_reference_shp(&mut self, path: &Path) -> bool {
        let (mut layer, crs_note) = match load_reference_shp(path, self.work_crs) {
            Ok(loaded) => loaded,
            Err(why) => {
                self.home_map.draw_guide_message = Some(why);
                return false;
            }
        };
        if layer.shapes.is_empty() {
            self.home_map.draw_guide_message =
                Some(format!("「{}」에 도형이 없어 올리지 않았습니다.", layer.name));
            return false;
        }
        // 같은 이름이 이미 있으면 번호를 붙인다.
        let base = layer.name.clone();
        let mut n = 2;
        while self.home_map.reference_layers.iter().any(|l| l.name == layer.name) {
            layer.name = format!("{base} ({n})");
            n += 1;
        }
        self.left_layers.entries.push(LayerTreeEntry {
            layer_key: format!("ref:{}", layer.name),
            label_ko: layer.name.clone(),
            group: LegendGroup::ReferenceMap,
            visible: true,
            opacity: 1.0,
            read_only: true,
            snap_enabled: true,
            feature_count: layer.shapes.len(),
        });
        self.home_map
            .real_survey
            .layer_visible
            .insert(layer.name.clone(), true);
        // 조사가 비어 있으면 올린 자료가 보이도록 그 범위로 간다.
        if self.survey_doc.total_features() == 0 {
            if let Some([min_x, min_y, max_x, max_y]) = layer.extent() {
                self.home_map.viewport.center_xy = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
                let span = (max_x - min_x).max(max_y - min_y).max(10.0);
                self.home_map.viewport.meters_per_pixel = span / 600.0;
            }
        }
        self.home_map.draw_guide_message = Some(format!(
            "「{}」 참조 레이어를 올렸습니다 (도형 {}개 · {crs_note}). 조사 파일에는 저장되지 않습니다.",
            layer.name,
            layer.shapes.len()
        ));
        self.home_map.reference_layers.push(layer);
        self.active_tab = crate::app_state::MainTab::Map;
        true
    }
}

#[cfg(test)]
#[path = "reference_layers_tests.rs"]
mod tests;
