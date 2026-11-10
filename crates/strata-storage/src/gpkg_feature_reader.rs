//! 저장된 피처를 **다시 읽어 온다**. 저장의 반대쪽 짝.
//!
//! ## 왜 필요한가
//!
//! 2026-11-10 흐름 시험: 조사구역을 그리고 저장한 뒤 다시 열면 도형이 0개였다.
//! 쓰기(`gpkg_feature_writer`)만 있고 읽기가 없어서, 저장은 되어도 앱이
//! 되살리지 못했다. 사용자 눈에는 "저장해도 사라진다"로 보인다.
//!
//! C++ 은 QGIS 프로바이더가 레이어를 읽어 오지만(`loadSurveyLayers`),
//! Rust 는 레이어마다 직접 읽는다.
//!
//! WKB 해석은 기존 [`crate::wkb_codec::decode_gpkg_wkb`] 를 쓴다(직접 만들지 않는다).

use std::path::Path;

use rusqlite::Connection;

use strata_types::schema::DomainLayerKey;

use crate::wkb_codec::{decode_gpkg_wkb, WkbGeometry};

/// 읽어 온 피처 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct GpkgFeatureRead {
    /// 면·선의 바깥 링(지도 좌표).
    pub ring: Vec<[f64; 2]>,
    /// `name` 열 값(없으면 빈 문자열).
    pub name: String,
}

/// 레이어 표에서 피처를 읽는다. 표가 없거나 비었으면 빈 벡터.
///
/// 실패해도 패닉하지 않는다 — 파일이 예전 형식이거나 표가 없을 수 있다
/// (그 경우 "도형 0개"가 맞는 답이다).
#[must_use]
pub fn read_layer_features(path: &Path, key: DomainLayerKey) -> Vec<GpkgFeatureRead> {
    let Ok(conn) = Connection::open(path) else {
        return Vec::new();
    };
    let table = key.id();
    let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT geom, name FROM \"{table}\" ORDER BY fid"
    )) else {
        return Vec::new();
    };
    let Ok(mut rows) = stmt.query([]) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let Ok(blob) = r.get::<_, Vec<u8>>(0) else {
            continue;
        };
        let name = r.get::<_, String>(1).unwrap_or_default();
        let Ok((_srs, geom)) = decode_gpkg_wkb(&blob) else {
            continue;
        };
        let ring = geometry_to_ring(&geom);
        if ring.len() >= 2 {
            out.push(GpkgFeatureRead { ring, name });
        }
    }
    out
}

/// WKB 지오메트리에서 **바깥 링**만 꺼낸다.
///
/// GeoPackage 는 폴리곤을 닫힌 링으로 저장하므로, 마지막 점이 첫 점과 같으면 뺀다
/// (앱 내부 표현은 닫지 않은 링이다).
fn geometry_to_ring(g: &WkbGeometry) -> Vec<[f64; 2]> {
    match g {
        WkbGeometry::Point(p) => vec![*p],
        WkbGeometry::LineString(pts) => pts.clone(),
        WkbGeometry::Polygon(rings) => {
            let Some(first) = rings.first() else {
                return Vec::new();
            };
            let mut r = first.clone();
            if r.len() > 1 && r.first() == r.last() {
                r.pop();
            }
            r
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpkg_feature_writer::{write_layer_features, GpkgFeatureWrite};
    use strata_types::crs::WorkCrs;

    fn temp_gpkg(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ka_read_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("임시 폴더");
        let p = dir.join("t.gpkg");
        crate::gpkg::GpkgRepository::create_new_survey(&p, WorkCrs::Epsg5187).expect("생성");
        p
    }

    fn square() -> Vec<[f64; 2]> {
        vec![
            [200_000.0, 500_000.0],
            [200_100.0, 500_000.0],
            [200_100.0, 500_100.0],
            [200_000.0, 500_100.0],
        ]
    }

    /// **핵심 회귀 시험**: 쓴 것이 그대로 읽힌다(좌표까지).
    ///
    /// 고치기 전에는 저장은 되는데 읽기 경로가 없어 "다시 열면 0개"였다.
    #[test]
    fn written_features_round_trip_exactly() {
        let p = temp_gpkg("roundtrip");
        let original = square();
        write_layer_features(
            &p,
            DomainLayerKey::SurveyArea,
            5187,
            &[GpkgFeatureWrite {
                ring: original.clone(),
                name: "조사구역 1".to_owned(),
            }],
        )
        .expect("쓰기");

        let back = read_layer_features(&p, DomainLayerKey::SurveyArea);
        assert_eq!(back.len(), 1, "1개를 썼으면 1개가 읽혀야");
        assert_eq!(back[0].name, "조사구역 1", "이름이 살아야");
        // 좌표가 그대로 (닫는 점은 빠져 있을 수 있으므로 앞부분만 비교)
        assert_eq!(back[0].ring.len(), original.len(), "점 수가 같아야");
        for (a, b) in back[0].ring.iter().zip(original.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6, "{a:?} vs {b:?}");
        }
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 여러 개도 순서대로 읽힌다.
    #[test]
    fn multiple_features_keep_order() {
        let p = temp_gpkg("order");
        let feats: Vec<GpkgFeatureWrite> = (1..=3)
            .map(|i| GpkgFeatureWrite {
                ring: vec![
                    [200_000.0 + f64::from(i) * 10.0, 500_000.0],
                    [200_050.0 + f64::from(i) * 10.0, 500_000.0],
                    [200_050.0 + f64::from(i) * 10.0, 500_050.0],
                ],
                name: format!("F{i}"),
            })
            .collect();
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &feats).expect("쓰기");
        let back = read_layer_features(&p, DomainLayerKey::SurveyArea);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].name, "F1");
        assert_eq!(back[2].name, "F3");
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 표가 없으면 빈 벡터(패닉 없음) — 예전에 저장한 파일을 열 때.
    #[test]
    fn missing_table_reads_empty() {
        let p = temp_gpkg("missing");
        assert!(read_layer_features(&p, DomainLayerKey::TrialTrench).is_empty());
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 닫힌 링은 닫는 점을 빼고 돌려준다(앱 내부 표현).
    #[test]
    fn closed_ring_drops_the_duplicate_end() {
        let closed = WkbGeometry::Polygon(vec![vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 0.0],
        ]]);
        let r = geometry_to_ring(&closed);
        assert_eq!(r.len(), 3, "닫는 점을 빼면 3점");
    }
}
