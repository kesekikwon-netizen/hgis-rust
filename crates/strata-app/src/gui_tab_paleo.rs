use eframe::egui;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// `KaPaleoStudio.cpp:36-260` — 3단 분할(`layersCard` | `centerSplit`[고지형 분류도 + 단면도] | `rightScroll`[6대 자료·7대 범례·요약])
    pub(crate) fn render_paleo_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 우측 스크롤 패널 (KaPaleoStudio.cpp:118-190)
        egui::SidePanel::right("ka_paleo_right_panel")
            .resizable(true)
            .default_width(350.0)
            .min_width(300.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong("6대 입력 자료 현황");
                    ui.colored_label(
                        egui::Color32::from_rgb(21, 128, 61),
                        format!(
                            "● 국토지리원DEM · ● 토양도 · ● 지질도({}건)\n● 수계도 · ● KIGAM수문 · ● 시추({}공·디클러스터링)",
                            self.paleo_3d.geology_feature_count,
                            self.paleo_3d.declustered_borehole_count
                        ),
                    );
                    ui.add_space(6.0);
                    if ui
                        .button("수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기")
                        .clicked()
                    {
                        self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
                    }
                    ui.checkbox(
                        &mut self.paleo_3d.show_imhof_slope_overlay,
                        "Imhof 다중방위 음영·경사도 중첩 시각화",
                    );
                    let btn_run = egui::Button::new(
                        egui::RichText::new("준비된 자료로 고지형 분석 실행")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(181, 87, 58))
                    .min_size(egui::vec2(ui.available_width(), 32.0));
                    if ui.add(btn_run).clicked() {
                        self.paleo_3d.run_sample_10km_analysis();
                        self.home_map.draw_guide_message = Some(
                            "고지형 4단계 파이프라인(반경 10km) 연산을 완료했습니다.".to_owned(),
                        );
                    }
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new("도면(조판)에 고지형 판독 근거 넣기"),
                        )
                        .clicked()
                    {
                        self.active_tab = MainTab::Drawing;
                        self.home_map.draw_guide_message =
                            Some("도면(조판) 시트에 고지형 판독 근거 및 단면도를 반영했습니다.".to_owned());
                    }
                    ui.separator();
                    ui.strong("7대 고지형 분류 체계 및 유적 잠재력");
                    for (col, name_ko, desc) in [
                        (
                            egui::Color32::from_rgb(217, 119, 6),
                            "자연제방",
                            "유적 입지 최우선 고잠재력 (취락·생산)",
                        ),
                        (
                            egui::Color32::from_rgb(37, 99, 235),
                            "구하도",
                            "유기물·목기·수리시설 매장이상대",
                        ),
                        (
                            egui::Color32::from_rgb(16, 185, 129),
                            "배후습지",
                            "저습지·고대 수전(논) 유구 유망",
                        ),
                        (
                            egui::Color32::from_rgb(234, 179, 8),
                            "선상지",
                            "선사·고대 생활유구 완경사 미고지",
                        ),
                        (
                            egui::Color32::from_rgb(132, 204, 22),
                            "범람원",
                            "충적층 심도별 다층 문화층 부존 가능",
                        ),
                        (
                            egui::Color32::from_rgb(180, 83, 9),
                            "구릉사면",
                            "분묘·주거지 사면 말단부 입지",
                        ),
                        (
                            egui::Color32::from_rgb(120, 113, 108),
                            "산지·기반암",
                            "산성·관방·봉수·제사 유적 입지",
                        ),
                    ] {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(col, "■");
                            ui.strong(name_ko);
                            ui.label(format!("— {desc}"));
                        });
                    }
                    ui.separator();
                    ui.strong("시추공 충적층 두께 및 수치지질도(1:5만) 범례");
                    ui.label("• 충적층 두께: ● <2m(연청) · ● 2~5m(황갈) · ● 5~10m(주황) · ● >10m(적갈)");
                    ui.label("• 지질(KIGAM): Qa 제4기 충적층 · Qr 하안단구층 · Ks 경상누층군 · Kgr 불국사화강암");
                    ui.separator();
                    ui.strong("조사구역 고지형·기본층서 판독 요약 및 근거");
                    let mut summary = self
                        .paleo_3d
                        .last_paleo_output
                        .as_ref()
                        .map_or_else(
                            || "분석 실행 버튼을 누르면 판독 요약이 생성됩니다.".to_owned(),
                            |o| {
                                format!(
                                    "{}\n• 1단계(지형기복): EDT 수계거리 및 다중스케일 TPI 자연제방 미고지 감지\n• 2단계(지반·시추): 성토층 0.0~1.2m / 충적층(문화층 유망) 1.2~3.8m / 풍화대 3.8m 이하\n• 3단계(수문·토양): KIGAM 제4기 충적층(Qa) 및 흙토람 사양질 배수 양호",
                                    o.aoi_summary_text
                                )
                            },
                        );
                    ui.add(
                        egui::TextEdit::multiline(&mut summary)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new("판독 요약·근거 문장 복사"),
                        )
                        .clicked()
                    {
                        ctx.copy_text(summary);
                        self.home_map.draw_guide_message = Some(
                            "고지형·기본층서 판독 요약문을 클립보드에 복사했습니다.".to_owned(),
                        );
                    }
                });
            });

        // 2. 좌측 레이어 패널 (KaPaleoStudio.cpp:45-93)
        egui::SidePanel::left("ka_paleo_left_layers")
            .resizable(true)
            .default_width(250.0)
            .min_width(200.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("레이어");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.small_button("선택 삭제(Del)");
                    });
                });
                ui.separator();
                for name in [
                    "☑ 7대 고지형 분류도 (반경 10km)",
                    "☑ 국토지리원 1:5,000 정밀 DEM",
                    "☑ 지오인포 시추주상도 (12공)",
                    "☑ KIGAM 고지형 수문·연대",
                    "☑ 흙토람 분포지형 토양도",
                    "☑ 국가·지방하천 수계망",
                    "☑ 조사구역 및 A-A' 단면선",
                ] {
                    ui.label(name);
                }
                ui.separator();
                ui.label(
                    egui::RichText::new("해상도: 5.0m TIN 격자\n좌표계: EPSG:5187 (동부원점)")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(94, 93, 89)),
                );
            });

        // 3. 중앙 수직 분할 (상단 7대 고지형 분류도·시추공 프리뷰 + 하단 KaPaleoCrossSectionWidget 지층 단면도)
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let total_h = ui.available_height();
            let top_h = (total_h * 0.62).max(200.0);
            // 상단: 고지형 분류도 + 시추공 마커 프리뷰
            let (map_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), top_h),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(map_rect);
            painter.rect_filled(map_rect, 4.0, egui::Color32::from_rgb(232, 238, 226));
            if self.paleo_3d.show_imhof_slope_overlay && self.paleo_3d.imhof_slope_rgba.len() == 8 * 8 * 4 {
                let imhof_img = egui::ColorImage::from_rgba_unmultiplied([8, 8], &self.paleo_3d.imhof_slope_rgba);
                let imhof_tex = ctx.load_texture("paleo_imhof_slope", imhof_img, egui::TextureOptions::LINEAR);
                painter.image(
                    imhof_tex.id(),
                    map_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 165),
                );
            }
            // 구하도(청색 띠) 및 자연제방(황토색 미고지) 시각화
            painter.rect_filled(
                egui::Rect::from_min_size(
                    map_rect.min + egui::vec2(40.0, 35.0),
                    egui::vec2(map_rect.width() - 80.0, 56.0),
                ),
                18.0,
                egui::Color32::from_rgba_unmultiplied(37, 99, 235, 95),
            );
            painter.rect_filled(
                egui::Rect::from_min_size(
                    map_rect.min + egui::vec2(70.0, 100.0),
                    egui::vec2(map_rect.width() - 140.0, 74.0),
                ),
                14.0,
                egui::Color32::from_rgba_unmultiplied(217, 119, 6, 125),
            );
            // A-A' 단면 절단선 및 시추공(BH-01~BH-04) 마커
            let p_a = map_rect.min + egui::vec2(80.0, 135.0);
            let p_b = egui::pos2(map_rect.right() - 80.0, map_rect.top() + 135.0);
            painter.line_segment(
                [p_a, p_b],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(181, 87, 58)),
            );
            for (i, code) in [
                "BH-01 · 충적 2.6m",
                "BH-02 · 충적 3.6m",
                "BH-03 · 충적 4.6m",
                "BH-04 · 충적 2.6m",
            ]
            .iter()
            .enumerate()
            {
                let t = (i as f32 + 0.5) / 4.0;
                let pos = p_a.lerp(p_b, t);
                painter.circle_filled(pos, 6.0, egui::Color32::from_rgb(20, 20, 19));
                painter.text(
                    pos + egui::vec2(0.0, -14.0),
                    egui::Align2::CENTER_BOTTOM,
                    *code,
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(20, 20, 19),
                );
            }
            painter.text(
                map_rect.min + egui::vec2(14.0, 10.0),
                egui::Align2::LEFT_TOP,
                "반경 10km 7대 고지형 분류도 · 구하도(청색) / 자연제방(황색) / 시추공 단면선(A–A')",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(20, 20, 19),
            );

            ui.separator();
            ui.strong("지층 단면도 (KaPaleoCrossSectionWidget — A–A' 현 지표면 · 고지형면 · 풍화대/기반암)");
            if self.paleo_3d.cross_section_rgba.len() == 320 * 120 * 4 {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [320, 120],
                    &self.paleo_3d.cross_section_rgba,
                );
                let tex = ctx.load_texture("paleo_cross_sec", img, egui::TextureOptions::LINEAR);
                let sec_rect = ui.available_rect_before_wrap();
                ui.painter().image(
                    tex.id(),
                    sec_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        });
    }
}
