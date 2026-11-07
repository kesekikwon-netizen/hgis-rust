use geo::{Area, Geometry, Polygon};

/// C++ `HGIS_MAX_PIECE_COUNT` (26) — 기본 단일 분할 최대 조각 수
pub const HGIS_MAX_PIECE_COUNT: usize = 26;

/// C++ `HGIS_KOREAN_PIECE_SUFFIXES` — 한글 자모 접미사 표준 배열 상수
pub const HGIS_KOREAN_PIECE_SUFFIXES: [&str; 14] = [
    "가", "나", "다", "라", "마", "바", "사", "아", "자", "차", "카", "타", "파", "하",
];

/// C++ `PieceNamingStyle` — 분할 조각 번호 부여 스타일
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceNamingStyle {
    HyphenNumeric,
    KoreanAlphabet,
}

/// C++ `PolygonPieceResult` — 면적 내림차순 정렬 및 번호가 부여된 분할 폴리곤 조각
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonPieceResult {
    pub polygon: Polygon<f64>,
    pub piece_number: String,
    pub area_m2: f64,
}

/// C++ `KaPolygonPieceNamer::assignPieceIdentifiers` 1:1 대응:
/// 분할된 폴리곤 조각들의 면적을 산출하여 **면적 내림차순(`areaA > areaB`)**으로 정렬한 뒤,
/// 가장 큰 본체 조각부터 `"1호-1"`, `"1호-2"` (또는 `"1호-가"`, `"1호-나"`)를 부여한다.
#[must_use]
pub fn assign_piece_identifiers(
    original_feat_no: &str,
    split_pieces: &[Polygon<f64>],
    style: PieceNamingStyle,
) -> Vec<PolygonPieceResult> {
    let valid_pieces: Vec<(Polygon<f64>, f64)> = split_pieces
        .iter()
        .map(|p| (p.clone(), p.unsigned_area()))
        .filter(|(_, area)| area.is_finite() && *area > 1e-9)
        .collect();

    if valid_pieces.is_empty() {
        return Vec::new();
    }
    if valid_pieces.len() == 1 {
        let (polygon, area_m2) = valid_pieces.into_iter().next().expect("single piece");
        return vec![PolygonPieceResult {
            polygon,
            piece_number: original_feat_no.to_owned(),
            area_m2,
        }];
    }

    let mut sorted = valid_pieces;
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    sorted
        .into_iter()
        .take(HGIS_MAX_PIECE_COUNT)
        .enumerate()
        .map(|(idx, (polygon, area_m2))| {
            let suffix = match style {
                PieceNamingStyle::HyphenNumeric => (idx + 1).to_string(),
                PieceNamingStyle::KoreanAlphabet => HGIS_KOREAN_PIECE_SUFFIXES
                    .get(idx)
                    .copied()
                    .map_or_else(|| (idx + 1).to_string(), str::to_owned),
            };
            PolygonPieceResult {
                polygon,
                piece_number: format!("{original_feat_no}-{suffix}"),
                area_m2,
            }
        })
        .collect()
}

pub fn split_polygon_pieces(geom: &Geometry<f64>) -> Vec<Polygon<f64>> {
    let mut out = Vec::new();
    collect_valid_polygons(geom, &mut out);
    out
}

fn collect_valid_polygons(geom: &Geometry<f64>, out: &mut Vec<Polygon<f64>>) {
    match geom {
        Geometry::Polygon(p) => {
            if p.unsigned_area() > 1e-9 {
                out.push(p.clone());
            }
        }
        Geometry::MultiPolygon(mp) => {
            for p in &mp.0 {
                if p.unsigned_area() > 1e-9 {
                    out.push(p.clone());
                }
            }
        }
        Geometry::GeometryCollection(gc) => {
            for item in &gc.0 {
                collect_valid_polygons(item, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn multipolygon_splits_into_non_zero_single_polygons_on_gpkg_write() {
        let mp = wkt!(MULTIPOLYGON(
            ((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)),
            ((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0)),
            ((5.0 5.0, 5.0 5.0, 5.0 5.0, 5.0 5.0))
        ));
        let pieces = split_polygon_pieces(&Geometry::MultiPolygon(mp));
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn test_multipolygon_decompose_3_islands_preserves_exact_count_and_total_area() {
        let mp = wkt!(MULTIPOLYGON(
            ((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)),
            ((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0)),
            ((40.0 40.0, 50.0 40.0, 50.0 45.0, 40.0 45.0, 40.0 40.0))
        ));
        let orig_geom = Geometry::MultiPolygon(mp);
        let orig_area = orig_geom.unsigned_area();
        let pieces = split_polygon_pieces(&orig_geom);
        assert_eq!(pieces.len(), 3);
        let sum_area: f64 = pieces.iter().map(|p| p.unsigned_area()).sum();
        assert!((sum_area - orig_area).abs() < 1e-6);

        // TestKaPolygonPieceNamer::testAreaSortingAndNaming:
        // Piece A = 100.0 m2, Piece B = 300.0 m2 -> sorted [300m2 ("1호-1"/"1호-가"), 100m2 ("1호-2"/"1호-나")]
        let piece_100 = wkt!(POLYGON((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)));
        let piece_300 = wkt!(POLYGON((0.0 0.0, 30.0 0.0, 30.0 10.0, 0.0 10.0, 0.0 0.0)));
        let named_num = assign_piece_identifiers(
            "1호",
            &[piece_100.clone(), piece_300.clone()],
            PieceNamingStyle::HyphenNumeric,
        );
        assert_eq!(named_num.len(), 2);
        assert!((named_num[0].area_m2 - 300.0).abs() < 1e-6);
        assert_eq!(named_num[0].piece_number, "1호-1");
        assert!((named_num[1].area_m2 - 100.0).abs() < 1e-6);
        assert_eq!(named_num[1].piece_number, "1호-2");

        let named_ko = assign_piece_identifiers(
            "1호",
            &[piece_100, piece_300],
            PieceNamingStyle::KoreanAlphabet,
        );
        assert_eq!(named_ko[0].piece_number, "1호-가");
        assert_eq!(named_ko[1].piece_number, "1호-나");
    }
}
