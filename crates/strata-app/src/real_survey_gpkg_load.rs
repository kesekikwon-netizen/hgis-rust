//! 지적도·유산 GPKG 읽기 (좌표계 5179 → 5187 변환 포함).
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·시그니처·동작은 그대로다.
//! `load_from_dir`(다른 파일의 같은 타입 impl)에서 부르므로 `pub(crate)` 다 — 크레이트 밖으로는
//! 나가지 않으므로 공개 API 는 그대로다.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use strata_geo::crs_transform::CrsTransformer;
use strata_storage::wkb_codec::{decode_gpkg_wkb, WkbGeometry};
use strata_types::crs::CrsCode;

use super::real_survey_types::{ring_centroid, RealCadastralParcel, RealHeritagePoly, RealSurveyData};

impl RealSurveyData {
    pub(crate) fn load_cadastral_gpkg(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir.join("지적도").join("표시")) else {
            return;
        };
        let Some(gpkg) = entries
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().and_then(|s| s.to_str()) == Some("gpkg"))
        else {
            return;
        };
        let Ok(conn) = Connection::open_with_flags(
            &gpkg,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            return;
        };
        let [cx, cy] = self.survey_center;
        let sql = "SELECT c.JIBUN, c.geom FROM cadastral c JOIN rtree_cadastral_geom r ON c.fid = r.id \
                   WHERE r.minx <= ?1 AND r.maxx >= ?2 AND r.miny <= ?3 AND r.maxy >= ?4 LIMIT 1800";
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(mut rows) = stmt.query([cx + 1500.0, cx - 1500.0, cy + 1300.0, cy - 1300.0]) {
                while let Ok(Some(r)) = rows.next() {
                    let jibun = r
                        .get::<_, Option<String>>(0)
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    if let Ok(blob) = r.get::<_, Vec<u8>>(1) {
                        if let Ok((_, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                            if let Some(ext) = rings.first() {
                                self.cadastral_parcels.push(RealCadastralParcel {
                                    jibun,
                                    ring: ext.clone(),
                                    center: ring_centroid(ext),
                                });
                            }
                        }
                    }
                }
            }
        };
    }

    pub(crate) fn load_heritage_gpkgs(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir.join("가져온자료")) else {
            return;
        };
        let xf = CrsTransformer::new(CrsCode::Epsg5179, CrsCode::Epsg5187);
        let [cx, cy] = self.survey_center;
        for entry in entries.flatten() {
            let layer_name = entry.file_name().to_string_lossy().to_string();
            let gpkg = entry.path().join("heritage.gpkg");
            let is_green = layer_name.contains("매장")
                || layer_name.contains("분포")
                || layer_name.contains("유존")
                || layer_name.contains("보호");
            let Ok(conn) = Connection::open_with_flags(
                &gpkg,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) else {
                continue;
            };
            if let Ok(mut stmt) = conn.prepare("SELECT __hgis_geom FROM heritage LIMIT 250") {
                if let Ok(mut rows) = stmt.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(blob) = r.get::<_, Vec<u8>>(0) {
                            if let Ok((srs, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                                for ring in rings {
                                    let ring_5187: Vec<[f64; 2]> = if srs == 5179 {
                                        ring.into_iter()
                                            .map(|p| xf.transform_xy(p[0], p[1]))
                                            .collect()
                                    } else {
                                        ring
                                    };
                                    let center = ring_centroid(&ring_5187);
                                    if (center[0] - cx).abs() <= 2600.0
                                        && (center[1] - cy).abs() <= 2200.0
                                    {
                                        self.heritage_polys.push(RealHeritagePoly {
                                            layer_name: layer_name.clone(),
                                            site_name: layer_name.clone(),
                                            is_green,
                                            ring: ring_5187,
                                            center,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            };
        }
    }
}
