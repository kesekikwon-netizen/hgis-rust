//! 레이어 테이블에 **실제 피처(도형+속성)** 를 쓴다.
//!
//! ## 왜 필요한가
//!
//! 2026-11-10 사용자 흐름 시험에서 드러났다: 조사구역을 그리고 저장한 뒤 다시 열면
//! **도형이 0개**였다. `write_workspace_file` 은 `gpkg_contents`·
//! `gpkg_geometry_columns` 에 레이어를 **등록만** 하고, 레이어 테이블을 만들지도
//! 피처를 넣지도 않았다. 저장이 "성공"했다고 말하면서 도형은 사라진 것이다.
//!
//! C++ 은 `QgsVectorFileWriter::writeAsVectorFormatV3`(`SurveyStorage.cpp:88-95`)와
//! `provider->addFeatures(added)`(`:174`)로 실제 피처를 쓴다. 이 모듈이 그 자리다.
//!
//! ## 테이블 모양
//!
//! GeoPackage 규격대로 `fid INTEGER PRIMARY KEY AUTOINCREMENT`, `geom <TYPE>`,
//! 그리고 속성 열(여기서는 `name` 하나)을 둔다. WKB 는 기존
//! [`crate::wkb_codec`] 로 인코딩한다(직접 만들지 않는다).

use std::path::Path;

use rusqlite::{params, Connection};

use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::wkb_codec::{encode_gpkg_wkb, WkbGeometry};

/// 한 레이어에 넣을 피처.
#[derive(Debug, Clone, PartialEq)]
pub struct GpkgFeatureWrite {
    /// 면·선의 바깥 링(지도 좌표).
    pub ring: Vec<[f64; 2]>,
    /// `name` 열에 들어갈 이름(없으면 빈 문자열).
    pub name: String,
}

/// 레이어 테이블을 만들고 피처를 넣는다.
///
/// 이미 있으면 **비우고 다시 쓴다**(같은 조사를 두 번 저장해도 중복되지 않게).
/// C++ `CreateOrOverwriteLayer` 와 같은 동작이다.
///
/// # Errors
///
/// 파일을 열 수 없거나 SQL 이 실패하면 `StrataError::Storage`.
pub fn write_layer_features(
    path: &Path,
    key: DomainLayerKey,
    srs_epsg: u32,
    features: &[GpkgFeatureWrite],
) -> StrataResult<()> {
    let conn = Connection::open(path)
        .map_err(|e| StrataError::Storage(format!("GPKG 열기 실패: {e}")))?;

    let table = key.id();
    let geom_type = match key {
        DomainLayerKey::FeatureLine | DomainLayerKey::SectionLine => "MULTILINESTRING",
        DomainLayerKey::ControlPoints | DomainLayerKey::ArtifactPoint => "POINT",
        _ => "MULTIPOLYGON",
    };

    // 규격: fid + geom + 속성 열.
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS \"{table}\" (
             fid INTEGER PRIMARY KEY AUTOINCREMENT,
             geom {geom_type},
             name TEXT NOT NULL DEFAULT ''
         );"
    ))
    .map_err(|e| StrataError::Storage(format!("레이어 표 만들기 실패({table}): {e}")))?;

    // 다시 쓸 때 중복되지 않게 비운다.
    conn.execute(&format!("DELETE FROM \"{table}\""), [])
        .map_err(|e| StrataError::Storage(format!("레이어 비우기 실패({table}): {e}")))?;

    if features.is_empty() {
        return Ok(());
    }

    let sql = format!("INSERT INTO \"{table}\" (geom, name) VALUES (?1, ?2)");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StrataError::Storage(format!("INSERT 준비 실패({table}): {e}")))?;

    for f in features {
        if f.ring.len() < 2 {
            continue; // 그릴 수 없는 도형은 건너뛴다(무음 손실 아님 — 개수는 호출부가 안다)
        }
        let geom = match key {
            DomainLayerKey::ControlPoints | DomainLayerKey::ArtifactPoint => {
                WkbGeometry::Point(f.ring[0])
            }
            DomainLayerKey::FeatureLine | DomainLayerKey::SectionLine => {
                WkbGeometry::LineString(f.ring.clone())
            }
            _ => WkbGeometry::Polygon(vec![f.ring.clone()]),
        };
        // 닫힌 링이 아니면 닫아 준다 — GeoPackage 는 닫힌 링을 요구한다.
        let geom = close_ring_if_needed(geom);
        let blob = encode_gpkg_wkb(i32::try_from(srs_epsg).unwrap_or(5187), &geom);
        stmt.execute(params![blob, f.name])
            .map_err(|e| StrataError::Storage(format!("피처 쓰기 실패({table}): {e}")))?;
    }
    Ok(())
}

/// 링의 첫 점과 끝 점이 다르면 닫아 준다.
fn close_ring_if_needed(g: WkbGeometry) -> WkbGeometry {
    match g {
        WkbGeometry::Polygon(mut rings) => {
            for r in &mut rings {
                if r.len() >= 3 && r.first() != r.last() {
                    let first = r[0];
                    r.push(first);
                }
            }
            WkbGeometry::Polygon(rings)
        }
        WkbGeometry::LineString(mut pts) => {
            // 선은 닫지 않는다(닫으면 모양이 달라진다).
            if pts.len() == 1 {
                pts.push(pts[0]);
            }
            WkbGeometry::LineString(pts)
        }
        other => other,
    }
}

/// 저장된 피처 개수를 센다(검증용). 표가 없으면 0.
#[must_use]
pub fn count_layer_features(path: &Path, key: DomainLayerKey) -> usize {
    let Ok(conn) = Connection::open(path) else {
        return 0;
    };
    let table = key.id();
    let Ok(n) = conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |r| {
        r.get::<_, i64>(0)
    }) else {
        return 0;
    };
    usize::try_from(n).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_types::crs::WorkCrs;

    fn temp_gpkg(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ka_feat_{tag}_{}", std::process::id()));
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

    /// **핵심 회귀 시험**: 쓴 피처가 파일에 실제로 남는다.
    ///
    /// 고치기 전에는 저장이 "성공"했지만 다시 열면 0개였다.
    #[test]
    fn written_features_are_really_in_the_file() {
        let p = temp_gpkg("write");
        let feats = vec![
            GpkgFeatureWrite {
                ring: square(),
                name: "조사구역 1".to_owned(),
            },
            GpkgFeatureWrite {
                ring: vec![[201_000.0, 501_000.0], [201_050.0, 501_050.0], [201_100.0, 501_000.0]],
                name: "조사구역 2".to_owned(),
            },
        ];
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &feats).expect("쓰기");
        assert_eq!(
            count_layer_features(&p, DomainLayerKey::SurveyArea),
            2,
            "쓴 피처 2개가 파일에 있어야 한다"
        );
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 두 번 저장해도 중복되지 않는다(비우고 다시 쓴다).
    #[test]
    fn rewriting_replaces_not_appends() {
        let p = temp_gpkg("rewrite");
        let one = vec![GpkgFeatureWrite {
            ring: square(),
            name: "A".to_owned(),
        }];
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &one).expect("1차");
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &one).expect("2차");
        assert_eq!(
            count_layer_features(&p, DomainLayerKey::SurveyArea),
            1,
            "다시 저장하면 1개여야 한다(중복 금지)"
        );
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 빈 목록을 쓰면 0개가 된다(지운 것을 저장할 수 있다).
    #[test]
    fn empty_list_clears_the_layer() {
        let p = temp_gpkg("empty");
        let one = vec![GpkgFeatureWrite {
            ring: square(),
            name: "A".to_owned(),
        }];
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &one).expect("1차");
        write_layer_features(&p, DomainLayerKey::SurveyArea, 5187, &[]).expect("빈 쓰기");
        assert_eq!(count_layer_features(&p, DomainLayerKey::SurveyArea), 0);
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }

    /// 폴리곤 링은 닫혀서 저장된다(GeoPackage 규격).
    #[test]
    fn polygon_rings_are_closed() {
        let g = close_ring_if_needed(WkbGeometry::Polygon(vec![square()]));
        let WkbGeometry::Polygon(rings) = g else {
            panic!("폴리곤이어야");
        };
        let r = &rings[0];
        assert_eq!(r.first(), r.last(), "첫 점과 끝 점이 같아야 닫힌 링");
    }

    /// 표가 없는 레이어를 세면 0(패닉 없음).
    #[test]
    fn counting_missing_layer_is_zero() {
        let p = temp_gpkg("missing");
        assert_eq!(count_layer_features(&p, DomainLayerKey::TrialTrench), 0);
        let _ = std::fs::remove_dir_all(p.parent().expect("부모"));
    }
}
