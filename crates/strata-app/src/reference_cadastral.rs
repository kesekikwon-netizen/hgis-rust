//! 받은 연속지적도 ZIP 을 「지적도 · 조사 주변 5km」 레이어 하나로 만든다.
//!
//! C++ `CadastralImport.cpp:66-294`: 시·군·구 ZIP 을 풀고, 조사구역 5 km 범위에 걸친 필지만
//! 남겨(자르지는 않는다) 한 레이어로 합친다. 같은 필지는 한 번만 넣는다.
//! 모양은 채움 없는 검은 0.2 mm 선, 지번(`JIBUN`) 글자는 1:10,000 보다 확대했을 때만 보인다.
//! 레이어는 「참조 지도」가 아니라 「지적도」 묶음에 놓인다.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use strata_types::crs::WorkCrs;
use strata_types::schema::LegendGroup;

use crate::reference_layers::{load_reference_shp, ReferenceLayer, ReferenceStyle};

/// C++ `CadastralImport.cpp:269` 의 레이어 이름.
pub const CADASTRAL_LAYER_NAME: &str = "지적도 · 조사 주변 5km";
/// 1:10,000 에서 화면 1 픽셀의 길이(m) — 96 DPI 기준 10000 ÷ 3780.
const LABEL_MAX_MPP: f64 = 10_000.0 / 3_780.0;

/// ZIP 들을 풀어 `clip`(작업 좌표계 `[min_x, min_y, max_x, max_y]`)에 걸친 필지를 한 레이어로.
pub fn load_cadastral_layer(
    zips: &[PathBuf],
    work_crs: WorkCrs,
    clip: [f64; 4],
) -> Result<ReferenceLayer, String> {
    let mut shapes = Vec::new();
    let mut seen: HashSet<(String, [i64; 4])> = HashSet::new();
    let mut shp_count = 0;
    for zip in zips {
        let stem = zip.file_stem().and_then(|s| s.to_str()).unwrap_or("지적도");
        let dest = zip.with_file_name(format!("{stem}_풀기"));
        let files = strata_storage::zip_extract::extract_zip(zip, &dest)
            .map_err(|e| format!("받은 지적도 ZIP 을 풀지 못했습니다: {e}"))?;
        for shp in strata_storage::zip_extract::shapefiles_in(&files) {
            shp_count += 1;
            let (layer, _) = load_reference_shp(&shp, work_crs, Some(clip))?;
            for shape in layer.shapes {
                // 같은 지번·같은 자리의 필지는 한 번만(시·군 경계에서 겹쳐 올 수 있다).
                let key = (shape.label.clone(), shape.bbox.map(|v| (v * 100.0).round() as i64));
                if seen.insert(key) {
                    shapes.push(shape);
                }
            }
        }
    }
    if shp_count == 0 {
        return Err("받은 시·군·구 ZIP에 SHP 도형이 없습니다.".to_owned());
    }
    if shapes.is_empty() {
        return Err("조사 주변 범위와 겹치는 필지가 없습니다.".to_owned());
    }
    Ok(ReferenceLayer {
        name: CADASTRAL_LAYER_NAME.to_owned(),
        source: zips.first().map_or_else(PathBuf::new, |z| z.parent().unwrap_or(Path::new("")).to_path_buf()),
        shape_type: 5,
        shapes,
        style: ReferenceStyle {
            line_rgb: [0, 0, 0],
            line_mm: 0.2,
            label_max_mpp: LABEL_MAX_MPP,
            group: LegendGroup::Cadastral,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제로 받은 지적도 ZIP 으로 확인한다. 파일이 없으면(다른 PC) 건너뛴다.
    /// 받는 법: `cargo run -p strata-net --example cadastral_probe -- login <경위도 범위> D:\GIS1\target\probe\지적도`
    #[test]
    fn real_downloaded_zips_become_one_layer_of_nearby_parcels_with_jibun_labels() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/probe/지적도");
        let zips: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|d| {
                d.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "zip"))
                    .collect()
            })
            .unwrap_or_default();
        if zips.is_empty() {
            eprintln!("받은 지적도 ZIP 이 없어 건너뜁니다: {}", dir.display());
            return;
        }
        // demo 조사구역(EPSG:5186, 199990~200110 × 449990~450090) 주변 5 km.
        let clip = [194_990.0, 444_990.0, 205_110.0, 455_090.0];
        let started = std::time::Instant::now();
        let layer = load_cadastral_layer(&zips, WorkCrs::Epsg5186, clip).expect("레이어");
        eprintln!("필지 {}개, {:.1}초", layer.shapes.len(), started.elapsed().as_secs_f32());
        assert_eq!(layer.name, CADASTRAL_LAYER_NAME);
        assert!(layer.shapes.len() > 1_000, "5 km 안에 필지가 수천 개는 있어야 한다: {}", layer.shapes.len());
        assert!(layer.shapes.len() < 200_000, "시·군 전체가 아니라 주변만: {}", layer.shapes.len());
        // 모든 필지가 범위에 걸쳐 있고, 지번이 한글로 읽힌다.
        assert!(layer.shapes.iter().all(|s| crate::reference_layers::boxes_overlap(s.bbox, clip)));
        let with_label = layer.shapes.iter().filter(|s| !s.label.is_empty()).count();
        assert!(with_label * 10 > layer.shapes.len() * 9, "지번이 있는 필지 {with_label}");
        let korean = layer.shapes.iter().filter(|s| s.label.chars().any(|c| ('가'..='힣').contains(&c))).count();
        assert!(korean * 2 > layer.shapes.len(), "지번에 지목(대·전·답 …) 한글이 보여야 한다: {korean}");
    }
}
