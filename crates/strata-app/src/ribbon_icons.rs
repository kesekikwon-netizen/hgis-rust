use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// Strata 브랜드 로고 (테라코타 둥근 사각형 + 고고학 꽃삽 + 등고선 파동)
pub fn paint_strata_logo(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 7.0, Color32::from_rgb(189, 98, 66));
    let c = rect.center();
    let r_max = rect.width() * 0.38;
    // 등고선 동심원 호
    for i in 1..=3 {
        let r = r_max * (i as f32 / 3.0);
        painter.circle_stroke(
            Pos2::new(c.x, c.y + 3.0),
            r,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 240, 230, 150)),
        );
    }
    // 꽃삽(Trowel) 마름모 날 + 손잡이
    let blade = vec![
        Pos2::new(c.x, c.y - 2.0),
        Pos2::new(c.x + 5.5, c.y + 3.5),
        Pos2::new(c.x, c.y + 9.5),
        Pos2::new(c.x - 5.5, c.y + 3.5),
    ];
    painter.add(egui::Shape::convex_polygon(
        blade,
        Color32::from_rgb(245, 243, 238),
        Stroke::new(1.2, Color32::from_rgb(60, 45, 38)),
    ));
    painter.line_segment(
        [Pos2::new(c.x, c.y - 2.0), Pos2::new(c.x, c.y - 9.5)],
        Stroke::new(2.4, Color32::from_rgb(60, 45, 38)),
    );
}

/// C++ `KaIcons.cpp` 23개 리본 벡터 라인 아이콘 1:1 드로잉
pub fn paint_ribbon_icon(painter: &egui::Painter, rect: Rect, label: &str, stroke_col: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.6, stroke_col);
    let s_thin = Stroke::new(1.2, stroke_col);
    match label {
        "새 조사" => {
            let r = Rect::from_center_size(c, Vec2::new(14.0, 16.0));
            painter.rect_stroke(r, 2.0, s, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x + 4.0, c.y)], s);
            painter.line_segment([Pos2::new(c.x, c.y - 4.0), Pos2::new(c.x, c.y + 4.0)], s);
        }
        "열기" => {
            let r = Rect::from_center_size(Pos2::new(c.x, c.y + 1.5), Vec2::new(16.0, 12.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.top() - 2.5),
                    Pos2::new(r.left() + 7.0, r.top() - 2.5),
                ],
                s,
            );
        }
        "저장" | "다른 이름" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 2.0, s, egui::StrokeKind::Middle);
            let top_box = Rect::from_min_max(
                Pos2::new(c.x - 4.5, r.top()),
                Pos2::new(c.x + 3.5, c.y - 1.5),
            );
            painter.rect_stroke(top_box, 1.0, s_thin, egui::StrokeKind::Middle);
            if label == "다른 이름" {
                painter.line_segment(
                    [
                        Pos2::new(c.x + 2.0, c.y + 5.0),
                        Pos2::new(c.x + 7.5, c.y - 0.5),
                    ],
                    s,
                );
            }
        }
        "선택" => {
            let pts = vec![
                Pos2::new(c.x - 5.0, c.y - 7.0),
                Pos2::new(c.x + 6.5, c.y - 1.0),
                Pos2::new(c.x + 1.0, c.y + 1.5),
                Pos2::new(c.x - 1.5, c.y + 7.0),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
        }
        "측거" => {
            let r = Rect::from_center_size(c, Vec2::new(17.0, 7.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            for dx in [-5.0, -1.5, 2.0, 5.5] {
                painter.line_segment(
                    [
                        Pos2::new(c.x + dx, r.top()),
                        Pos2::new(c.x + dx, r.top() + 3.2),
                    ],
                    s_thin,
                );
            }
        }
        "그리기" => {
            let pts = [
                Pos2::new(c.x - 5.5, c.y - 4.5),
                Pos2::new(c.x + 5.0, c.y - 6.0),
                Pos2::new(c.x + 6.0, c.y + 4.5),
                Pos2::new(c.x - 4.0, c.y + 6.0),
            ];
            painter.add(egui::Shape::closed_line(pts.to_vec(), s_thin));
            for p in pts {
                painter.circle_filled(p, 2.2, stroke_col);
            }
        }
        "시굴격자" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            for d in [-2.5, 2.5] {
                painter.line_segment(
                    [Pos2::new(c.x + d, r.top()), Pos2::new(c.x + d, r.bottom())],
                    s_thin,
                );
                painter.line_segment(
                    [Pos2::new(r.left(), c.y + d), Pos2::new(r.right(), c.y + d)],
                    s_thin,
                );
            }
        }
        "버퍼" => {
            painter.circle_stroke(c, 7.5, s);
            painter.circle_filled(c, 2.0, stroke_col);
        }
        "지적" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(c.x - 1.0, r.top()),
                    Pos2::new(c.x - 1.0, r.bottom()),
                ],
                s_thin,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left(), c.y + 1.0),
                    Pos2::new(c.x - 1.0, c.y + 1.0),
                ],
                s_thin,
            );
        }
        "수치지형" => {
            painter.line_segment([Pos2::new(c.x, c.y - 7.0), Pos2::new(c.x, c.y + 3.0)], s);
            painter.line_segment(
                [Pos2::new(c.x - 4.0, c.y - 1.0), Pos2::new(c.x, c.y + 3.0)],
                s,
            );
            painter.line_segment(
                [Pos2::new(c.x + 4.0, c.y - 1.0), Pos2::new(c.x, c.y + 3.0)],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 7.0, c.y + 6.5),
                    Pos2::new(c.x + 7.0, c.y + 6.5),
                ],
                s,
            );
        }
        "유산" => {
            let roof = vec![
                Pos2::new(c.x - 7.5, c.y - 0.5),
                Pos2::new(c.x, c.y - 7.5),
                Pos2::new(c.x + 7.5, c.y - 0.5),
            ];
            painter.add(egui::Shape::line(roof, s));
            let body = Rect::from_min_max(
                Pos2::new(c.x - 5.5, c.y - 0.5),
                Pos2::new(c.x + 5.5, c.y + 7.0),
            );
            painter.rect_stroke(body, 1.0, s, egui::StrokeKind::Middle);
        }
        "지형" | "DEM" => {
            let pts = vec![
                Pos2::new(c.x - 8.0, c.y + 6.0),
                Pos2::new(c.x - 2.5, c.y - 5.5),
                Pos2::new(c.x + 1.5, c.y + 1.5),
                Pos2::new(c.x + 5.0, c.y - 3.5),
                Pos2::new(c.x + 8.5, c.y + 6.0),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
        }
        "등고선" => {
            painter.circle_stroke(c, 7.5, s);
            painter.circle_stroke(c, 4.2, s_thin);
            painter.circle_filled(c, 1.5, stroke_col);
        }
        "토양" => {
            for dy in [-3.5, 0.5, 4.5] {
                let pts = vec![
                    Pos2::new(c.x, c.y + dy - 3.5),
                    Pos2::new(c.x + 7.5, c.y + dy),
                    Pos2::new(c.x, c.y + dy + 3.5),
                    Pos2::new(c.x - 7.5, c.y + dy),
                ];
                painter.add(egui::Shape::closed_line(pts, s_thin));
            }
        }
        "고지형" => {
            painter.circle_stroke(c, 7.2, s);
            painter.line_segment([c, Pos2::new(c.x, c.y - 4.2)], s);
            painter.line_segment([c, Pos2::new(c.x + 3.5, c.y + 1.5)], s);
        }
        "지질" => {
            for (dy, w) in [(-4.0, 5.5), (0.5, 8.0), (5.0, 6.5)] {
                painter.line_segment(
                    [Pos2::new(c.x - w, c.y + dy), Pos2::new(c.x + w, c.y + dy)],
                    s,
                );
            }
            painter.line_segment([Pos2::new(c.x, c.y - 6.5), Pos2::new(c.x, c.y + 1.0)], s);
        }
        "수계" => {
            for dy in [-4.5, 0.0, 4.5] {
                let pts = vec![
                    Pos2::new(c.x - 7.0, c.y + dy),
                    Pos2::new(c.x - 2.5, c.y + dy - 2.0),
                    Pos2::new(c.x + 2.5, c.y + dy + 2.0),
                    Pos2::new(c.x + 7.0, c.y + dy),
                ];
                painter.add(egui::Shape::line(pts, s));
            }
        }
        "옛 지도" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 13.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, r.top()),
                    Pos2::new(c.x - 2.5, r.bottom()),
                ],
                s_thin,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 2.5, r.top()),
                    Pos2::new(c.x + 2.5, r.bottom()),
                ],
                s_thin,
            );
        }
        "정합" => {
            painter.circle_stroke(c, 5.5, s);
            painter.line_segment(
                [Pos2::new(c.x - 8.5, c.y), Pos2::new(c.x + 8.5, c.y)],
                s_thin,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y - 8.5), Pos2::new(c.x, c.y + 8.5)],
                s_thin,
            );
        }
        "도면" => {
            let r = Rect::from_center_size(c, Vec2::new(13.0, 16.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            for dy in [-3.0, 0.5, 4.0] {
                painter.line_segment(
                    [
                        Pos2::new(c.x - 3.5, c.y + dy),
                        Pos2::new(c.x + 3.5, c.y + dy),
                    ],
                    s_thin,
                );
            }
        }
        "인쇄" => {
            let body = Rect::from_center_size(Pos2::new(c.x, c.y + 1.0), Vec2::new(16.0, 9.0));
            painter.rect_stroke(body, 1.5, s, egui::StrokeKind::Middle);
            let top = Rect::from_min_max(
                Pos2::new(c.x - 4.5, c.y - 6.5),
                Pos2::new(c.x + 4.5, c.y - 3.5),
            );
            painter.rect_stroke(top, 0.5, s_thin, egui::StrokeKind::Middle);
        }
        "GeoTIFF" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 13.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.circle_filled(Pos2::new(c.x - 3.5, c.y - 2.5), 1.6, stroke_col);
            painter.line_segment(
                [
                    Pos2::new(r.left() + 2.0, r.bottom() - 2.0),
                    Pos2::new(c.x + 1.5, c.y),
                ],
                s_thin,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 1.5, c.y),
                    Pos2::new(r.right() - 2.0, r.bottom() - 2.0),
                ],
                s_thin,
            );
        }
        "검수·제출" => {
            let pts = vec![
                Pos2::new(c.x, c.y - 7.5),
                Pos2::new(c.x + 7.0, c.y - 3.5),
                Pos2::new(c.x + 7.0, c.y + 4.0),
                Pos2::new(c.x, c.y + 7.5),
                Pos2::new(c.x - 7.0, c.y + 4.0),
                Pos2::new(c.x - 7.0, c.y - 3.5),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
            painter.line_segment([Pos2::new(c.x - 7.0, c.y - 3.5), c], s_thin);
            painter.line_segment([Pos2::new(c.x + 7.0, c.y - 3.5), c], s_thin);
            painter.line_segment([c, Pos2::new(c.x, c.y + 7.5)], s_thin);
        }
        _ => {
            for dx in [-4.5, 0.0, 4.5] {
                painter.circle_filled(Pos2::new(c.x + dx, c.y), 1.6, stroke_col);
            }
        }
    }
}

/// C++ `MainWindowRibbon.cpp` 25개 리본 단추 툴팁 및 단축키 안내 1:1
pub fn cpp_ribbon_tooltip(label: &str) -> &'static str {
    match label {
        "새 조사" => "새 조사 — 현장 조사 프로젝트를 새로 만듭니다 (Ctrl+N)",
        "열기" => "저장한 조사를 엽니다 (Ctrl+O)",
        "저장" => "현재 조사를 저장합니다 (Ctrl+S)",
        "다른 이름" => "작업 중인 모든 레이어를 다른 이름으로 저장합니다 (Ctrl+Shift+S)",
        "선택" => "그린 도형을 선택합니다. 다시 누르면 이동으로 돌아갑니다 (Ctrl+1, A)",
        "측거" => "지도에서 거리와 면적을 측정합니다. 다시 누르면 종료합니다",
        "그리기" => {
            "그리기 — 조사구역·유구 면과 선을 그립니다. 다시 누르면 도구를 닫고 이동합니다 (Ctrl+D)"
        }
        "시굴격자" => {
            "조사구역이 있으면 바로 시굴격자를 놓고, 없으면 지도에서 놓을 자리를 누릅니다"
        }
        "버퍼" => "주변유적 500M·1000M 표시하기 — 선택한 면 둘레에 거리 경계를 그립니다",
        "지적" => "조사구역 주변 5km 지적도를 받아 경계선과 지번을 표시합니다",
        "수치지형" => "HGIS 안에서 국토지리정보원에 로그인하고 수치지형도를 받습니다",
        "유산" => {
            "조사구역이 속한 시·군의 국가유산 자료를 인트라넷에서 받아 참조 지도로 올립니다"
        }
        "지형" => "등고·음영 지형도를 켭니다. 다시 누르면 숨깁니다",
        "DEM" => "고도를 색으로 보여 줍니다 (지표모델 DSM 30m · Copernicus)",
        "등고선" => "측량 DXF·엑셀 좌표로 등고선과 해발 색 구간을 만듭니다",
        "토양" => "흙토람 토양도를 겹칩니다. 다시 누르면 숨깁니다",
        "고지형" => "흙토람 입지 후보를 강조하고 구하도·자연제방 등 가설을 그립니다",
        "지질" => "KIGAM 1:5만 지질 색 위에 지형 음영을 겹칩니다",
        "수계" => "하천망을 겹칩니다. 다시 누르면 숨깁니다",
        "옛 지도" => "대동여지도와 1919년 조선지형도를 참조 지도로 올립니다",
        "정합" => "좌표없는 사진·CAD를 도면에 합치기",
        "도면" => "도면 만들기 — 종이에 지도를 올려 도면을 만듭니다 (Ctrl+L)",
        "인쇄" => "도면을 프린터로 찍거나, 큰 도면을 A3·A4 여러 장으로 나눠 찍습니다 (Ctrl+P)",
        "GeoTIFF" => "현재 지도에 보이는 범위와 레이어를 지도 좌표계 그대로 GeoTIFF로 저장합니다",
        "검수·제출" => {
            "도면을 검수하고 제출 꾸러미(EPSG:5179 SHP·조사도면.pdf·MANIFEST)를 만듭니다 (Ctrl+E)"
        }
        _ => "가끔 쓰는 기능 (F9 왼쪽 패널, F10 오른쪽 패널, Ctrl+F11 지도 넓게 보기)",
    }
}
