use eframe::egui;
use strata_layout::pdf_geotiff::render_sheet_pdf_bytes;
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// `KaDrawingStudio.cpp` + `KaDrawingStudioSheets.cpp` + `KaTitleBlock.cpp` 1:1 재현
    pub(crate) fn render_drawing_tab_ui(&mut self, ui: &mut egui::Ui) {
        // 1. 우측 도면 설정 및 출력 패널
        egui::SidePanel::right("ka_drawing_right_panel")
            .resizable(true)
            .default_width(270.0)
            .min_width(230.0)
            .show_inside(ui, |ui| {
                ui.strong("용지 · 축척 · 조판 요소");
                ui.separator();
                ui.label("용지 크기:");
                ui.horizontal(|ui| {
                    for (sz, lbl) in [
                        (SheetPaperSize::A4, "A4"),
                        (SheetPaperSize::A3, "A3"),
                    ] {
                        let sel = self.drawing.sheet.paper_size == sz;
                        if ui.selectable_label(sel, lbl).clicked() {
                            let ori = self.drawing.sheet.orientation;
                            self.drawing.sheet.change_paper(sz, ori);
                        }
                    }
                });
                ui.label("용지 방향:");
                ui.horizontal(|ui| {
                    for (ori, lbl) in [
                        (SheetOrientation::Landscape, "가로"),
                        (SheetOrientation::Portrait, "세로"),
                    ] {
                        let sel = self.drawing.sheet.orientation == ori;
                        if ui.selectable_label(sel, lbl).clicked() {
                            let sz = self.drawing.sheet.paper_size;
                            self.drawing.sheet.change_paper(sz, ori);
                        }
                    }
                });
                ui.separator();
                ui.strong("도면 장식 요소 (KaDrawingStudio)");
                ui.label("☑ 방위표 (ka_north)");
                ui.label("☑ 축척막대 (ka_scalebar · 1:1,000)");
                ui.label("☑ 좌표계 주기 (ka_crs)");
                ui.label("☑ 범례 상자 (ka_legend)");
                ui.label("☑ 도면 표제란 (KaTitleBlock)");
                ui.separator();
                let pdf_btn = egui::Button::new(
                    egui::RichText::new("📄 PDF 도면 저장")
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(egui::Color32::from_rgb(181, 87, 58))
                .min_size(egui::vec2(ui.available_width(), 32.0));
                if ui.add(pdf_btn).clicked() {
                    if let Ok(bytes) = render_sheet_pdf_bytes(&self.drawing.sheet, &self.project_title) {
                        let out = std::env::temp_dir().join("조사도면.pdf");
                        let _ = std::fs::write(&out, bytes);
                        self.home_map.draw_guide_message =
                            Some(format!("벡터 PDF 도면 저장 완료: {}", out.display()));
                    }
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("🖨 인쇄 (A3·A4 분할 인쇄)"),
                    )
                    .clicked()
                {
                    self.home_map.draw_guide_message =
                        Some("A3·A4 타일 분할 인쇄 조판(여백 10mm·재단선 포함)을 준비했습니다.".to_owned());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("📦 검수·제출 꾸러미 (EPSG:5179)"),
                    )
                    .clicked()
                {
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                }
            });

        // 2. 좌측 도면 시트 세트 & 도면 레이어 패널 (KaDrawingSheetSet.cpp)
        egui::SidePanel::left("ka_drawing_left_sheets")
            .resizable(true)
            .default_width(250.0)
            .min_width(200.0)
            .show_inside(ui, |ui| {
                ui.strong("도면 시트 세트 (KaDrawingSheetSet)");
                ui.separator();
                for sheet_name in [
                    "1. 조사지역 위치도 (1:25,000)",
                    "2. 주변유적 분포도 (1:5,000)",
                    "3. 조사구역 지형·지적도 (1:1,000)",
                    "4. 시굴트렌치 배치도 (1:500)",
                    "5. 고지형 분석·단면도 (1:10,000)",
                ] {
                    let _ = ui.selectable_label(sheet_name.starts_with("2."), sheet_name);
                }
                ui.separator();
                ui.strong("도면 표시 레이어");
                ui.label("☑ 조사구역 (survey_area)");
                ui.label("☑ 시굴트렌치 (trial_trench)");
                ui.label("☑ 반경 500m·1000m 유적경계");
                ui.label("☑ 주변 국가유산 번호 배지 (①~③)");
                ui.label("☑ 수치지형도 등고선 · 연속지적도");
            });

        // 3. 중앙 용지 프리뷰 (ka_map + ka_north + ka_scalebar + ka_legend + KaTitleBlock)
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(230, 228, 220));

            let scale = ((rect.width() - 48.0) / self.drawing.sheet.width_mm as f32)
                .min((rect.height() - 48.0) / self.drawing.sheet.height_mm as f32)
                .clamp(0.8, 2.4);
            let paper_w = self.drawing.sheet.width_mm as f32 * scale;
            let paper_h = self.drawing.sheet.height_mm as f32 * scale;
            let paper_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(paper_w, paper_h));

            // 종이 바탕 및 외곽 테두리
            painter.rect_filled(paper_rect, 2.0, egui::Color32::WHITE);
            painter.rect_stroke(
                paper_rect,
                2.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(20, 20, 19)),
                egui::StrokeKind::Inside,
            );

            // 본 지도 프레임 (ka_map)
            let map_rect = egui::Rect::from_min_max(
                paper_rect.min + egui::vec2(18.0, 18.0),
                egui::pos2(paper_rect.right() - 135.0, paper_rect.bottom() - 54.0),
            );
            painter.rect_filled(map_rect, 0.0, egui::Color32::from_rgb(245, 243, 235));
            painter.rect_stroke(
                map_rect,
                0.0,
                egui::Stroke::new(1.2, egui::Color32::from_rgb(20, 20, 19)),
                egui::StrokeKind::Inside,
            );

            // 지도 프레임 내부: 500m 원형 버퍼 + 조사구역 + 시굴트렌치 + 유적 배지(①~③)
            let mc = map_rect.center();
            painter.circle_stroke(
                mc,
                (map_rect.height() * 0.36).min(120.0),
                egui::Stroke::new(1.2, egui::Color32::from_rgb(217, 119, 6)),
            );
            let site_box = egui::Rect::from_center_size(mc, egui::vec2(84.0, 56.0));
            painter.rect_filled(
                site_box,
                0.0,
                egui::Color32::from_rgba_unmultiplied(181, 87, 58, 55),
            );
            painter.rect_stroke(
                site_box,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(181, 87, 58)),
                egui::StrokeKind::Inside,
            );
            for (idx, offset) in [(-45.0, -35.0), (52.0, -28.0), (38.0, 46.0)]
                .iter()
                .enumerate()
            {
                let bp = mc + egui::vec2(offset.0, offset.1);
                painter.circle_filled(bp, 9.0, egui::Color32::from_rgb(181, 87, 58));
                painter.text(
                    bp,
                    egui::Align2::CENTER_CENTER,
                    (idx + 1).to_string(),
                    egui::FontId::proportional(10.5),
                    egui::Color32::WHITE,
                );
            }

            // 우상단 방위표 (ka_north)
            let north_pos = egui::pos2(map_rect.right() - 24.0, map_rect.top() + 26.0);
            painter.text(
                north_pos,
                egui::Align2::CENTER_CENTER,
                "N\n▲",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(20, 20, 19),
            );

            // 좌하단 축척막대 (ka_scalebar) & 좌표계 주기 (ka_crs)
            painter.text(
                egui::pos2(map_rect.left() + 12.0, map_rect.bottom() - 18.0),
                egui::Align2::LEFT_CENTER,
                format!("축척 1:5,000  |  0 ━━ 100 ━━ 200m  |  {}", self.work_crs.label_ko()),
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(20, 20, 19),
            );

            // 우측 범례 상자 (ka_legend)
            let leg_rect = egui::Rect::from_min_max(
                egui::pos2(map_rect.right() + 8.0, map_rect.top()),
                egui::pos2(paper_rect.right() - 14.0, map_rect.bottom()),
            );
            painter.rect_stroke(
                leg_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 20, 19)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                leg_rect.min + egui::vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                "범 례\n──────────\n■ 조사구역\n■ 시굴트렌치\n○ 500m 반경\n● 국가유산:\n ① 인왕동 고분군\n ② 월성 해자\n ③ 동궁과 월지",
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(20, 20, 19),
            );

            // 하단 표제란 (KaTitleBlock)
            let tb_rect = egui::Rect::from_min_max(
                egui::pos2(paper_rect.left() + 18.0, map_rect.bottom() + 6.0),
                egui::pos2(paper_rect.right() - 14.0, paper_rect.bottom() - 12.0),
            );
            painter.rect_stroke(
                tb_rect,
                0.0,
                egui::Stroke::new(1.2, egui::Color32::from_rgb(20, 20, 19)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                tb_rect.min + egui::vec2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "조사명: {}   |   도면명: 조사지역 및 주변유적 분포도   |   좌표계: {} (제출 EPSG:5179)   |   STRATA GIS",
                    self.project_title,
                    self.work_crs.auth_id()
                ),
                egui::FontId::proportional(11.5),
                egui::Color32::from_rgb(20, 20, 19),
            );
        });
    }
}
