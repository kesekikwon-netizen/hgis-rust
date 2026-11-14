//! 「새 조사」 대화상자의 경도 자 — C++ `KaLongitudeRuler.cpp:40-107` 1:1.
//!
//! `dlg_new_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_new_survey` 안의 자 그리기 블록을 **그대로** 옮겼다.
//! 이 블록은 `app` 을 쓰지 않고 자기 지역변수만 쓰므로 함수 하나로 떼어낼 수 있었다.
//! 눈금·색·문구·도시 표시는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use strata_storage::gpkg::KA_LONGITUDE_RULER_CITIES;

/// 126°~130° 경도 자 (128° 중부|동부 경계 + 0.1° 눈금 + 8개 도시 + 지도 중심 표시).
pub(crate) fn paint_longitude_ruler(ui: &mut egui::Ui, map_lon: Option<f64>) {
    // C++ KaLongitudeRuler (126°~130°, 128° 중부|동부 경계 + 8개 도시 눈금)
    let (ruler_r, _) =
        ui.allocate_exact_size(Vec2::new(440.0, 62.0), egui::Sense::hover());
    let rp = ui.painter_at(ruler_r);
    rp.rect_filled(ruler_r, 4.0, Color32::from_rgb(245, 241, 232));
    rp.rect_stroke(
        ruler_r,
        4.0,
        Stroke::new(1.0, Color32::from_rgb(212, 204, 190)),
        egui::StrokeKind::Middle,
    );
    let x_of = |lon: f64| -> f32 {
        let t = ((lon - 126.0) / 4.0).clamp(0.0, 1.0) as f32;
        ruler_r.left() + 18.0 + t * (ruler_r.width() - 36.0)
    };
    let bar_y = ruler_r.top() + 34.0;
    let mid_x = x_of(128.0);
    rp.line_segment(
        [Pos2::new(x_of(126.0), bar_y), Pos2::new(x_of(130.0), bar_y)],
        Stroke::new(1.2, Color32::from_rgb(168, 160, 147)),
    );
    rp.line_segment(
        [
            Pos2::new(mid_x, ruler_r.top() + 4.0),
            Pos2::new(mid_x, bar_y + 8.0),
        ],
        Stroke::new(1.5, Color32::from_rgb(180, 83, 9)),
    );
    rp.text(
        Pos2::new(mid_x, ruler_r.top() + 11.0),
        egui::Align2::CENTER_CENTER,
        "중부원점 | 동부원점",
        egui::FontId::proportional(10.0),
        Color32::from_rgb(180, 83, 9),
    );
    for deg in 126..=130 {
        let dx = x_of(f64::from(deg));
        rp.line_segment(
            [Pos2::new(dx, bar_y - 4.0), Pos2::new(dx, bar_y + 4.0)],
            Stroke::new(1.2, Color32::from_rgb(90, 85, 76)),
        );
        rp.text(
            Pos2::new(dx, bar_y + 13.0),
            egui::Align2::CENTER_CENTER,
            format!("{deg}°"),
            egui::FontId::proportional(10.0),
            Color32::from_rgb(90, 85, 76),
        );
    }
    // KaLongitudeRuler.cpp:40-107 — 0.1° 눈금. 정수(7px, 진함),
    // 반(5px, 옅음), 나머지(3px, 옅음)로 높이와 색을 나눈다.
    for d10 in 1260..=1300 {
        let deg = f64::from(d10) / 10.0;
        let is_whole = d10 % 10 == 0;
        let is_half = d10 % 5 == 0;
        let tick_h = if is_whole {
            7.0
        } else if is_half {
            5.0
        } else {
            3.0
        };
        let tick_c = if is_whole {
            Color32::from_rgb(31, 35, 40)
        } else {
            Color32::from_rgb(209, 205, 197)
        };
        let tx = x_of(deg);
        rp.line_segment(
            [Pos2::new(tx, bar_y - tick_h), Pos2::new(tx, bar_y)],
            Stroke::new(if is_whole { 1.2 } else { 1.0 }, tick_c),
        );
    }
    // target/cpp-ref/src/app/KaLongitudeRuler.cpp:14-16,99-107 — centre mark (vertical line and
    // small triangle, no degree text) only when longitude is finite and within 126..=130.
    if let Some(center_lon) =
        map_lon.filter(|lon| lon.is_finite() && (126.0..=130.0).contains(lon))
    {
        let cx = x_of(center_lon);
        let clay = Color32::from_rgb(181, 87, 58);
        rp.line_segment(
            [
                Pos2::new(cx, ruler_r.top() + 2.0),
                Pos2::new(cx, bar_y + 2.0),
            ],
            Stroke::new(1.5, clay),
        );
        rp.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(cx, bar_y + 1.0),
                Pos2::new(cx - 4.0, bar_y - 6.0),
                Pos2::new(cx + 4.0, bar_y - 6.0),
            ],
            clay,
            Stroke::NONE,
        ));
    }
    for (city, lon) in KA_LONGITUDE_RULER_CITIES {
        let cx = x_of(lon);
        let col = if lon < 128.0 {
            Color32::from_rgb(29, 78, 216)
        } else {
            Color32::from_rgb(185, 28, 28)
        };
        rp.circle_filled(Pos2::new(cx, bar_y), 3.0, col);
        rp.text(
            Pos2::new(cx, bar_y - 11.0),
            egui::Align2::CENTER_CENTER,
            city,
            egui::FontId::proportional(10.0),
            Color32::from_rgb(45, 43, 39),
        );
    }
}
