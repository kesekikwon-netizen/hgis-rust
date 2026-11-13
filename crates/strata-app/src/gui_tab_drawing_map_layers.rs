//! 용지 지도 칸에 실제 조사 레이어를 축척대로 그린다 — 원문 (a)~(f) 구간.
//!
//! 본문은 원문 그대로이고 바깥 지역변수는 머리의 alias 줄로 되살린다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_layout::badge_placer::LayoutBadgePlacer;

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// (a) 지적도 ~ (f) 작도 유구.
    pub(crate) fn paint_sheet_map_layers(&mut self, f: &SheetFrame) {
        let map_rect = f.map_rect;
        let map_p = f.map_p.clone();
        let rs = &self.home_map.real_survey;
        // (a) 실제 경주시 지적도 레이어
        if rs.is_layer_visible("지적도 · 조사 주변 5km") {
            let cad_stroke =
                Stroke::new(0.55, Color32::from_rgba_unmultiplied(68, 66, 60, 140));
            for parcel in &rs.cadastral_parcels {
                if !f.in_view_bbox(parcel.center) {
                    continue;
                }
                let pts: Vec<Pos2> = parcel.ring.iter().map(|&pt| f.w2m(pt)).collect();
                map_p.add(egui::Shape::line(pts, cad_stroke));
                if self.drawing.sheet.scale_denominator <= 2_500
                    && rs.is_label_visible("지적도 · 조사 주변 5km")
                    && !parcel.jibun.is_empty()
                {
                    map_p.text(
                        f.w2m(parcel.center),
                        egui::Align2::CENTER_CENTER,
                        &parcel.jibun,
                        egui::FontId::proportional(7.5),
                        Color32::from_rgba_unmultiplied(55, 54, 50, 180),
                    );
                }
            }
        }

        // (b) 실제 경주시 지정유산·매장유산유존지역·문화유적분포지도 레이어
        if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
            let green_fill = Color32::from_rgba_unmultiplied(39, 174, 96, 42);
            let green_stroke = Stroke::new(1.15, Color32::from_rgb(34, 148, 52));
            let mag_fill = Color32::from_rgba_unmultiplied(142, 68, 173, 42);
            let mag_stroke = Stroke::new(1.15, Color32::from_rgb(162, 32, 118));
            let mut visible_heritage: Vec<(usize, &crate::real_survey::RealHeritagePoly)> =
                Vec::new();

            for h in &rs.heritage_polys {
                if !rs.is_layer_visible(&h.layer_name) || !f.in_view_bbox(h.center) {
                    continue;
                }
                let pts: Vec<Pos2> = h.ring.iter().map(|&pt| f.w2m(pt)).collect();
                if pts.len() >= 3 {
                    map_p.add(egui::Shape::convex_polygon(
                        pts.clone(),
                        if h.is_green { green_fill } else { mag_fill },
                        Stroke::NONE,
                    ));
                }
                map_p.add(egui::Shape::line(
                    pts,
                    if h.is_green { green_stroke } else { mag_stroke },
                ));
                if visible_heritage.len() < 16 {
                    let next_no = visible_heritage.len() + 1;
                    visible_heritage.push((next_no, h));
                }
            }

            if LayoutBadgePlacer::should_render_site_names(
                self.drawing.sheet.scale_denominator,
                self.drawing.sheet.show_site_names_under_5000,
            ) {
                let sample_sites: Vec<(u32, String, f64, f64, f64, f64)> = visible_heritage
                    .iter()
                    .take(10)
                    .map(|&(no, hp)| {
                        let sp = f.w2m(hp.center);
                        (
                            no as u32,
                            hp.site_name.clone(),
                            f64::from(sp.x),
                            f64::from(sp.y),
                            68.0,
                            14.0,
                        )
                    })
                    .collect();
                let placed = LayoutBadgePlacer::place_all_boxes(
                    &sample_sites,
                    Some((
                        f64::from(map_rect.left() + 4.0),
                        f64::from(map_rect.top() + 4.0),
                        f64::from(map_rect.right() - 4.0),
                        f64::from(map_rect.bottom() - 4.0),
                    )),
                );
                for nb in placed {
                    let br = Rect::from_center_size(
                        Pos2::new(nb.center_x_mm as f32, nb.center_y_mm as f32),
                        Vec2::new(nb.width_mm as f32, nb.height_mm as f32),
                    );
                    map_p.rect_filled(
                        br,
                        2.5,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 232),
                    );
                    map_p.rect_stroke(
                        br,
                        2.5,
                        Stroke::new(0.8, Color32::from_rgb(35, 34, 30)),
                        egui::StrokeKind::Middle,
                    );
                    map_p.text(
                        br.center(),
                        egui::Align2::CENTER_CENTER,
                        &nb.name,
                        egui::FontId::proportional(8.0),
                        Color32::from_rgb(17, 24, 39),
                    );
                }
            } else {
                for (no, hp) in visible_heritage.into_iter().take(12) {
                    let sp = f.w2m(hp.center);
                    if !map_rect.shrink(8.0).contains(sp) {
                        continue;
                    }
                    let badge_col = if hp.is_green {
                        Color32::from_rgb(39, 174, 96)
                    } else {
                        Color32::from_rgb(142, 68, 173)
                    };
                    map_p.circle_filled(sp, 6.5, badge_col);
                    map_p.circle_stroke(sp, 6.5, Stroke::new(1.0, Color32::WHITE));
                    map_p.text(
                        sp,
                        egui::Align2::CENTER_CENTER,
                        no.to_string(),
                        egui::FontId::proportional(8.0),
                        Color32::WHITE,
                    );
                }
            }
        }

        // (c) 「주변 Nm」 경계선과 「Nm」 거리 글자(범례에서 켠 것만)
        for (meters, ring) in &rs.distance_rings {
            let title = format!("주변 {meters}m");
            if rs.is_layer_visible(&title) {
                for line in &ring.lines {
                    let buf_pts: Vec<Pos2> = line.iter().map(|&pt| f.w2m(pt)).collect();
                    map_p.add(egui::Shape::line(buf_pts, Stroke::new(1.6, Color32::from_rgb(184, 58, 24))));
                }
            }
            let label_layer = format!("{title} 거리");
            if rs.is_layer_visible(&label_layer) && rs.is_label_visible(&label_layer) {
                for pt in &ring.labels {
                    let bp = f.w2m(*pt);
                    if map_rect.shrink(12.0).contains(bp) {
                        let b_rect = Rect::from_center_size(bp, Vec2::new(32.0, 13.0));
                        map_p.rect_filled(b_rect, 2.5, Color32::from_rgba_unmultiplied(255, 255, 255, 230));
                        let text = format!("{meters}m");
                        map_p.text(b_rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(8.0), Color32::from_rgb(184, 58, 24));
                    }
                }
            }
        }

        // (d) 실제 조사구역 (survey_rings)
        if rs.is_layer_visible("조사구역") {
            let fill_col = Color32::from_rgba_unmultiplied(214, 92, 32, 82);
            let stroke = Stroke::new(1.9, Color32::from_rgb(214, 78, 22));
            for ring in &rs.survey_rings {
                let pts: Vec<Pos2> = ring.iter().map(|&pt| f.w2m(pt)).collect();
                if pts.len() >= 3 {
                    map_p.add(egui::Shape::convex_polygon(
                        pts.clone(),
                        fill_col,
                        Stroke::NONE,
                    ));
                }
                map_p.add(egui::Shape::line(pts, stroke));
            }
            // F1: 면적 글은 그린 조사구역의 바깥 상자 가운데에만(예전엔 조사구역이 없어도 경주 기본점에 「0 m²」).
            let pts = rs.survey_rings.iter().flatten();
            let bbox = pts.fold(None::<[f64; 4]>, |b, p| {
                Some(b.map_or([p[0], p[1], p[0], p[1]], |b| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]))
            });
            if let (true, Some(b)) = (rs.is_label_visible("조사구역"), bbox) {
                let sp = f.w2m([(b[0] + b[2]) * 0.5, (b[1] + b[3]) * 0.5]);
                let lbl_r = Rect::from_center_size(sp, Vec2::new(64.0, 15.0));
                map_p.rect_filled(
                    lbl_r,
                    2.5,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 225),
                );
                map_p.text(
                    lbl_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0} m²", rs.survey_area_m2),
                    egui::FontId::proportional(8.5),
                    Color32::from_rgb(17, 24, 39),
                );
            }
        }

        // (e) 실제 시굴격자 (trenches 232개 전수)
        if rs.is_layer_visible("시굴격자") {
            let tr_fill = Color32::from_rgba_unmultiplied(156, 62, 32, 65);
            let tr_stroke = Stroke::new(1.0, Color32::from_rgb(156, 62, 32));
            for tr in &rs.trenches {
                if !f.in_view_bbox(tr.center) {
                    continue;
                }
                let pts: Vec<Pos2> = tr.ring.iter().map(|&pt| f.w2m(pt)).collect();
                if pts.len() >= 3 {
                    map_p.add(egui::Shape::convex_polygon(
                        pts.clone(),
                        tr_fill,
                        Stroke::NONE,
                    ));
                }
                map_p.add(egui::Shape::line(pts, tr_stroke));
            }
        }

        // (f) 작도된 유구 다각형 (scene.polygons)
        for poly in &self.home_map.scene.polygons {
            let pts: Vec<Pos2> = poly.world_pts.iter().map(|&pt| f.w2m(pt)).collect();
            if pts.len() >= 3 {
                map_p.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(181, 87, 58, 70),
                    Stroke::new(1.4, Color32::from_rgb(142, 56, 32)),
                ));
            }
        }
    }
}
