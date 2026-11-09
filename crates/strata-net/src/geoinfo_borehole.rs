use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strata_types::error::{StrataError, StrataResult};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratumLayer {
    pub from_m: f64,
    pub to_m: f64,
    pub name: String,
    pub uscs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoreholeRecord {
    pub hole_code: String,
    pub tm_x: f64,
    pub tm_y: f64,
    pub altitude_m: f64,
    pub fill_depth_m: f64,
    pub alluvial_from_m: f64,
    pub alluvial_to_m: f64,
    pub alluvial_thickness_m: f64,
    pub weathered_depth_m: f64,
    /// 지하수위 깊이(GL-m). C++ `BoreholeRecord::waterLevelM` 대응. 미출현은 0.0 이고
    /// 소비 측에서 `abs(v) > 0.01` 로 거른다 (`GeoInfoBoreholeMaps.cpp:218, 222`).
    #[serde(default)]
    pub water_level_m: f64,
    pub dominant_uscs: String,
    pub paleo_hint: String,
    pub layers: Vec<StratumLayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleCandidate {
    pub code: String,
    pub pt_5186: [f64; 2],
}

#[derive(Debug, Deserialize)]
struct OuterWrapper {
    #[serde(rename = "jsonView")]
    json_view: String,
}

#[derive(Debug, Deserialize)]
struct RawLayerItem {
    #[serde(rename = "FROM_DEPTH", default)]
    from_depth: f64,
    #[serde(rename = "TO_DEPTH", default)]
    to_depth: f64,
    #[serde(rename = "LAYER_NAME", default)]
    layer_name: String,
    #[serde(rename = "USCS", default)]
    uscs: String,
    /// C++ `GeoInfoBoreholeParser.cpp:82` 의 `"WATER"` 속성 — 숫자·문자열 모두 허용.
    #[serde(rename = "WATER", default)]
    water: Option<serde_json::Value>,
}

/// C++ `toVariant().toString(); toDouble(&ok)` 대응 — 유한 숫자만 인정한다.
fn json_value_to_f64(v: &serde_json::Value) -> Option<f64> {
    let parsed = match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    parsed.filter(|x| x.is_finite())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoreholeLayerError {
    EmptyStrata,
}

#[derive(Debug, Clone, Copy)]
pub struct GeoInfoBoreholeService;

impl GeoInfoBoreholeService {
    pub const HGIS_DEPTH_EPSILON: f64 = 1.0e-4;

    /// C++ `GeoinfoBoreholeClient::parseBoreholeLayers` — 심도 역전(`fromDepth >= toDepth`) 및 0두께 스킵, 총심도(`totalHoleDepth`) 초과 클램핑 1:1
    pub fn validate_and_clamp_strata_layers(
        raw_layers: &[StratumLayer],
        total_hole_depth: f64,
    ) -> Result<Vec<StratumLayer>, BoreholeLayerError> {
        let mut valid = Vec::with_capacity(raw_layers.len());
        for layer in raw_layers {
            if !layer.from_m.is_finite()
                || !layer.to_m.is_finite()
                || layer.from_m < 0.0
                || layer.to_m < 0.0
            {
                continue;
            }
            let clamped_to = if total_hole_depth > 0.0 && total_hole_depth.is_finite() {
                layer.to_m.min(total_hole_depth)
            } else {
                layer.to_m
            };
            if layer.from_m >= clamped_to
                || (clamped_to - layer.from_m).abs() < Self::HGIS_DEPTH_EPSILON
            {
                continue;
            }
            valid.push(StratumLayer {
                from_m: layer.from_m,
                to_m: clamped_to,
                name: layer.name.clone(),
                uscs: layer.uscs.clone(),
            });
        }
        if valid.is_empty() {
            Err(BoreholeLayerError::EmptyStrata)
        } else {
            Ok(valid)
        }
    }

    pub fn parse_sichudan_json(raw_bytes: &[u8]) -> StrataResult<BoreholeRecord> {
        let text = std::str::from_utf8(raw_bytes)
            .map_err(|e| StrataError::Network(format!("시추주상도 UTF-8 오류: {e}")))?;
        let inner_json = if let Ok(wrapper) = serde_json::from_str::<OuterWrapper>(text) {
            wrapper.json_view
        } else {
            text.to_string()
        };
        let items: Vec<RawLayerItem> = serde_json::from_str(&inner_json)
            .map_err(|e| StrataError::Network(format!("시추주상도 JSON 파싱 오류: {e}")))?;

        let mut fill_depth_m = 0.0_f64;
        let mut alluvial_from_m: Option<f64> = None;
        let mut alluvial_to_m = 0.0_f64;
        let mut weathered_depth_m = -1.0_f64;
        let mut dominant_uscs = String::new();
        let mut water_level_m = 0.0_f64;
        let mut layers = Vec::with_capacity(items.len());

        for item in items {
            // C++ `parseSichudanJson` 는 `WATER` 를 바깥 객체에서 한 번만 읽는다
            // (`GeoInfoBoreholeParser.cpp:82-88`). `jsonView` 안쪽 JSON 이 배열이면
            // 항목마다 실려 올 수 있으므로 첫 유효값을 채택한다.
            if water_level_m == 0.0 {
                if let Some(w) = item.water.as_ref().and_then(json_value_to_f64) {
                    water_level_m = w;
                }
            }
            let name = item.layer_name.trim();
            if name.contains("매립") || name.contains("성토") || name.contains("표토") {
                fill_depth_m = fill_depth_m.max(item.to_depth);
            } else if name.contains("충적") || name.contains("퇴적") || name.contains("붕적")
            {
                let start = alluvial_from_m.unwrap_or(item.from_depth);
                alluvial_from_m = Some(start.min(item.from_depth));
                alluvial_to_m = alluvial_to_m.max(item.to_depth);
                if dominant_uscs.is_empty() && !item.uscs.trim().is_empty() {
                    dominant_uscs = item.uscs.trim().to_string();
                }
            } else if weathered_depth_m < 0.0
                && (name.contains("풍화")
                    || name.contains("연암")
                    || name.contains("경암")
                    || name.contains("기반암"))
            {
                weathered_depth_m = item.from_depth;
            }
            layers.push(StratumLayer {
                from_m: item.from_depth,
                to_m: item.to_depth,
                name: name.to_string(),
                uscs: item.uscs.trim().to_string(),
            });
        }
        let alluv_from = alluvial_from_m.unwrap_or(fill_depth_m);
        let alluv_thick = (alluvial_to_m - alluv_from).max(0.0);
        let paleo_hint = if alluv_thick > 0.0 {
            format!("충적층 두께 {alluv_thick:.1}m ({dominant_uscs})")
        } else {
            String::from("충적층 미확인")
        };

        Ok(BoreholeRecord {
            hole_code: String::new(),
            tm_x: 0.0,
            tm_y: 0.0,
            altitude_m: 0.0,
            fill_depth_m,
            alluvial_from_m: alluv_from,
            alluvial_to_m,
            alluvial_thickness_m: alluv_thick,
            weathered_depth_m,
            water_level_m,
            dominant_uscs,
            paleo_hint,
            layers,
        })
    }

    pub fn select_spatially_balanced_candidates(
        raw: &[HoleCandidate],
        center_5186: [f64; 2],
        max_count: usize,
        cell_meters: f64,
    ) -> Vec<HoleCandidate> {
        let step = cell_meters.max(10.0);
        let mut sorted = raw.to_vec();
        sorted.sort_by(|a, b| {
            let da =
                (a.pt_5186[0] - center_5186[0]).powi(2) + (a.pt_5186[1] - center_5186[1]).powi(2);
            let db =
                (b.pt_5186[0] - center_5186[0]).powi(2) + (b.pt_5186[1] - center_5186[1]).powi(2);
            da.total_cmp(&db)
        });
        let mut occupied: BTreeSet<(i64, i64)> = BTreeSet::new();
        let mut out = Vec::new();
        for cand in sorted {
            if out.len() >= max_count {
                break;
            }
            let gx = (cand.pt_5186[0] / step).floor() as i64;
            let gy = (cand.pt_5186[1] / step).floor() as i64;
            if occupied.insert((gx, gy)) {
                out.push(cand);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geoinfo_sichudan_json_parser_extracts_fill_alluvial_and_weathered_depths() -> StrataResult<()>
    {
        let sample = r#"{"jsonView":"[{\"FROM_DEPTH\":0.0,\"TO_DEPTH\":1.5,\"LAYER_NAME\":\"매립층\",\"USCS\":\"SM\"},{\"FROM_DEPTH\":1.5,\"TO_DEPTH\":6.0,\"LAYER_NAME\":\"충적층\",\"USCS\":\"CL\"},{\"FROM_DEPTH\":6.0,\"TO_DEPTH\":10.0,\"LAYER_NAME\":\"풍화암\",\"USCS\":\"WR\"}]"}"#.as_bytes();
        let rec = GeoInfoBoreholeService::parse_sichudan_json(sample)?;
        assert!((rec.fill_depth_m - 1.5).abs() < 1e-6);
        assert!((rec.alluvial_thickness_m - 4.5).abs() < 1e-6);
        assert!((rec.weathered_depth_m - 6.0).abs() < 1e-6);
        assert_eq!(rec.dominant_uscs, "CL");
        Ok(())
    }

    #[test]
    fn test_strata_depth_inversion_5_to_2_skipped_and_total_depth_clamped() {
        let raw = vec![
            StratumLayer {
                from_m: 0.0,
                to_m: 2.0,
                name: "매립층".to_owned(),
                uscs: "SM".to_owned(),
            },
            StratumLayer {
                from_m: 5.0,
                to_m: 2.0,
                name: "역전지층".to_owned(),
                uscs: "SM".to_owned(),
            },
            StratumLayer {
                from_m: 2.0,
                to_m: 15.0,
                name: "충적층".to_owned(),
                uscs: "CL".to_owned(),
            },
        ];
        let valid = GeoInfoBoreholeService::validate_and_clamp_strata_layers(&raw, 10.0)
            .expect("valid strata");
        assert_eq!(valid.len(), 2);
        assert_eq!(valid[0].name, "매립층");
        assert_eq!(valid[1].name, "충적층");
        assert!((valid[1].to_m - 10.0).abs() < 1e-6);

        let all_invalid = vec![StratumLayer {
            from_m: 5.0,
            to_m: 2.0,
            name: "역전지층".to_owned(),
            uscs: "SM".to_owned(),
        }];
        assert_eq!(
            GeoInfoBoreholeService::validate_and_clamp_strata_layers(&all_invalid, 10.0),
            Err(BoreholeLayerError::EmptyStrata)
        );
    }
}
