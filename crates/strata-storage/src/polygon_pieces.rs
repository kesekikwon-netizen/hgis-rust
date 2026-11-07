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

/// 원본 폴리곤(`target_polygon`)에서 지우개 폴리곤(`eraser_polygon`)을 기하 차집합(`difference`)으로 도려내고 면적 내림차순으로 정렬된 조각 목록 반환 (`<= 1e-9` 미세 잔여물 자동 제거)
/// (C++ `PolygonErase::erased` & `partsFor` 1:1 대응)
#[must_use]
pub fn erase_polygon_area_difference(
    target_polygon: &Polygon<f64>,
    eraser_polygon: &Polygon<f64>,
) -> Vec<Polygon<f64>> {
    use geo::BooleanOps;

    let from_area = target_polygon.unsigned_area();
    if !from_area.is_finite() || from_area <= 1e-9 {
        return Vec::new();
    }
    if eraser_polygon.unsigned_area() <= 1e-9 {
        return vec![target_polygon.clone()];
    }
    let diff = target_polygon.difference(eraser_polygon);
    let mut parts: Vec<Polygon<f64>> = diff
        .0
        .into_iter()
        .filter(|p| p.unsigned_area() > from_area * 1e-9)
        .collect();
    parts.sort_by(|a, b| {
        b.unsigned_area()
            .partial_cmp(&a.unsigned_area())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    parts
}

/// 다중 레이어 원자적 편집 트랜잭션 레코드 (C++ `KaEditUndoOrder` 매크로 항목)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiLayerEditRecord {
    pub macro_label: String,
    pub layer_fids: Vec<(String, i64)>,
}

/// 다중 레이어 편집 시계열 동기화 Undo/Redo 스택 (C++ `KaEditUndoOrder` 1:1 대응)
#[derive(Debug, Clone, Default)]
pub struct MultiLayerUndoStack {
    undo_stack: Vec<MultiLayerEditRecord>,
    redo_stack: Vec<MultiLayerEditRecord>,
}

impl MultiLayerUndoStack {
    pub fn record_multi_layer_edit(&mut self, macro_label: &str, layer_fids: Vec<(String, i64)>) {
        self.redo_stack.clear();
        self.undo_stack.push(MultiLayerEditRecord {
            macro_label: macro_label.to_owned(),
            layer_fids,
        });
    }

    pub fn undo(&mut self) -> Option<MultiLayerEditRecord> {
        let rec = self.undo_stack.pop()?;
        self.redo_stack.push(rec.clone());
        Some(rec)
    }

    pub fn redo(&mut self) -> Option<MultiLayerEditRecord> {
        let rec = self.redo_stack.pop()?;
        self.undo_stack.push(rec.clone());
        Some(rec)
    }

    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
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

        // TestPolygonErase::testSquareEraseCenter:
        // 10m x 10m square (100 m²) minus 2m x 2m center eraser (4 m²) -> 96.0 m² and 1 interior ring
        let target_sq = wkt!(POLYGON((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)));
        let eraser_sq = wkt!(POLYGON((4.0 4.0, 6.0 4.0, 6.0 6.0, 4.0 6.0, 4.0 4.0)));
        let erased_parts = erase_polygon_area_difference(&target_sq, &eraser_sq);
        assert_eq!(erased_parts.len(), 1);
        assert!((erased_parts[0].unsigned_area() - 96.0).abs() <= 1e-4);
        assert_eq!(erased_parts[0].interiors().len(), 1);

        // Multi-layer composite Undo/Redo stack (KaEditUndoOrder)
        let mut undo_stack = MultiLayerUndoStack::default();
        undo_stack.record_multi_layer_edit(
            "겹친 곳 지우기",
            vec![
                ("feature_poly".to_owned(), 1),
                ("artifact_point".to_owned(), 2),
            ],
        );
        assert_eq!(undo_stack.undo_depth(), 1);
        let undone = undo_stack.undo().expect("macro undo");
        assert_eq!(undone.macro_label, "겹친 곳 지우기");
        assert_eq!(undone.layer_fids.len(), 2);
        assert_eq!(undo_stack.redo_depth(), 1);
        let redone = undo_stack.redo().expect("macro redo");
        assert_eq!(redone.macro_label, "겹친 곳 지우기");
    }
}
