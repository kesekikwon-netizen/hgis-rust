//! `dialogs_modal` 에서 옮긴 대화상자 묶음: ReferenceStatus.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_modal::ActiveModalDialog;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Stroke};


pub(crate) fn render_reference_status(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaReferenceStatusDialog.cpp:31-112 & ReferenceInventory.cpp:140-189 — "자료 준비 상태"
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 880.0_f32.min(vis_w - 40.0);
    let modal_h = 460.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_reference_status")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 24.0);
            ui.set_max_width(modal_w - 24.0);
            render_strata_modal_header(ui, "자료 준비 상태", open);

            let cad_ready = !app.home_map.real_survey.cadastral_parcels.is_empty();
            let her_ready = !app.home_map.real_survey.heritage_polys.is_empty();
            let dem_ready = app.home_map.real_survey.is_layer_visible("지형 음영")
                || app
                    .home_map
                    .real_survey
                    .is_layer_visible("지표모델(DSM) 30m · Coperni...");

            // ReferenceInventory::Item 대응 7종 참조 자료 (지적도·수치지형도·DEM·주변유적·지질도·토양도·수계도)
            let rows = [
                (
                    "지적도 · 조사 주변 5km",
                    "지적도",
                    "약 10.0 × 10.0 km",
                    if cad_ready { "2026-11-08 02:15" } else { "받지 않음" },
                    if cad_ready { "4.8 MB" } else { "—" },
                    if cad_ready {
                        "현장 사용 가능 (준비됨)"
                    } else {
                        "파일 없음 (없음)"
                    },
                    cad_ready,
                    ActiveModalDialog::CadastralDownload,
                    ActiveModalDialog::AccountSettings,
                ),
                (
                    "국토지리정보원 수치지형도 1:5,000",
                    "수치지형도",
                    "약 6.5 × 5.2 km",
                    "2026-11-08 02:18",
                    "12.4 MB",
                    "현장 사용 가능 (준비됨)",
                    true,
                    ActiveModalDialog::NgiiTopoDownload,
                    ActiveModalDialog::AccountSettings,
                ),
                (
                    "지표모델(DSM) 30m · 국토지리원 DEM",
                    "DEM",
                    "약 10.0 × 10.0 km",
                    if dem_ready { "2026-11-08 02:20" } else { "받지 않음" },
                    if dem_ready { "6.2 MB" } else { "—" },
                    if dem_ready {
                        "현장 사용 가능 (준비됨)"
                    } else {
                        "파일 없음 (없음)"
                    },
                    dem_ready,
                    ActiveModalDialog::DemDownload,
                    ActiveModalDialog::DemClass,
                ),
                (
                    "국가유산청 지정·비지정 주변유적 (경주시)",
                    "주변유적",
                    "약 12.0 × 12.0 km",
                    if her_ready { "2026-11-08 02:22" } else { "받지 않음" },
                    if her_ready { "9.1 MB" } else { "—" },
                    if her_ready {
                        "현장 사용 가능 (준비됨)"
                    } else {
                        "파일 없음 (없음)"
                    },
                    her_ready,
                    ActiveModalDialog::HeritageRegionSelect,
                    ActiveModalDialog::AccountSettings,
                ),
                (
                    "한국지질자원연구원 1:50,000 지질도 (KIGAM)",
                    "지질도",
                    "전국 이상",
                    "받지 않음",
                    "—",
                    "인터넷 필요 · 오프라인 저장 가능 (설정 필요)",
                    false,
                    ActiveModalDialog::PaleoSourceDownload,
                    ActiveModalDialog::AccountSettings,
                ),
                (
                    "농촌진흥청 흙토람 정밀토양도 (RDA)",
                    "토양도",
                    "전국 이상",
                    "받지 않음",
                    "—",
                    "인터넷 필요 · 오프라인 저장 전 제공처 조건 확인 (설정 필요)",
                    false,
                    ActiveModalDialog::PaleoSourceDownload,
                    ActiveModalDialog::AccountSettings,
                ),
                (
                    "국가수자원관리종합정보 하천망 수계도 (WAMIS)",
                    "수계도",
                    "전국 이상",
                    "파일 없음",
                    "—",
                    "파일 없음 · 오프라인 저장 불가 (없음)",
                    false,
                    ActiveModalDialog::PaleoSourceDownload,
                    ActiveModalDialog::AccountSettings,
                ),
            ];

            let offline_cnt = rows.iter().filter(|r| r.6).count();
            let online_cnt = 2usize;
            let missing_cnt = rows.len().saturating_sub(offline_cnt + online_cnt);
            // ReferenceInventory::summary(items) — "현장 사용 가능 N개 · 인터넷 필요 M개 · 파일 없음 K개"
            let summary_txt = format!(
                "현장 사용 가능 {offline_cnt}개 · 인터넷 필요 {online_cnt}개 · 파일 없음 {missing_cnt}개"
            );
            ui.label(
                egui::RichText::new(&summary_txt)
                    .size(15.0)
                    .strong()
                    .color(Color32::from_rgb(0x16, 0x16, 0x16)),
            );
            // KaReferenceStatusDialog.cpp:39-42 — what + how 문구
            ui.label(
                egui::RichText::new(
                    "조사에 올린 참조 자료가 현장에서 인터넷 없이 열리는지 확인하는 표입니다. 이 창은 자료를 새로 받거나 지우지 않습니다.\n인터넷이 필요한 배경은 레이어를 오른쪽 클릭해 「오프라인 저장」으로 직접 받아 두세요.",
                )
                .size(11.5)
                .color(Color32::from_rgb(0x47, 0x55, 0x69)),
            );
            ui.add_space(8.0);

            // KaReferenceStatusDialog.cpp:56-60 — 표 헤더 ("자료", "종류", "범위", "받은 날짜", "크기", "현장(오프라인)", "받기 / 설정")
            egui::Frame::NONE
                .fill(Color32::from_rgb(232, 229, 220))
                .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    egui::Grid::new("ka_ref_status_hdr")
                        .num_columns(7)
                        .min_col_width(48.0)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.set_min_width(modal_w - 44.0);
                            for (col_lbl, w) in [
                                ("자료", 176.0),
                                ("종류", 68.0),
                                ("범위", 96.0),
                                ("받은 날짜", 96.0),
                                ("크기", 56.0),
                                ("현장(오프라인)", 150.0),
                                ("받기 / 설정", 86.0),
                            ] {
                                ui.add_sized(
                                    [w, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(col_lbl).size(11.5).strong(),
                                    ),
                                );
                            }
                            ui.end_row();
                        });
                });

            // 7개 자료 행 렌더링
            let mut next_modal: Option<ActiveModalDialog> = None;
            egui::ScrollArea::vertical()
                .max_height(245.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (
                        idx,
                        (
                            name,
                            kind_lbl,
                            extent_txt,
                            received_txt,
                            size_txt,
                            offline_txt,
                            is_offline,
                            dl_modal,
                            cfg_modal,
                        ),
                    ) in rows.into_iter().enumerate()
                    {
                        let bg = if idx % 2 == 0 {
                            Color32::from_rgb(252, 251, 247)
                        } else {
                            Color32::WHITE
                        };
                        egui::Frame::NONE
                            .fill(bg)
                            .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                egui::Grid::new(("ka_ref_status_row", idx))
                                    .num_columns(7)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.add_sized(
                                            [176.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(name)
                                                    .size(11.5)
                                                    .strong(),
                                            )
                                            .truncate(),
                                        );
                                        ui.add_sized(
                                            [68.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(kind_lbl).size(11.5),
                                            ),
                                        );
                                        ui.add_sized(
                                            [96.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(extent_txt).size(11.0),
                                            ),
                                        );
                                        ui.add_sized(
                                            [96.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(received_txt)
                                                    .monospace()
                                                    .size(11.0),
                                            ),
                                        );
                                        // KaReferenceStatusDialog.cpp:106-109 — 크기 열 우측 정렬
                                        ui.with_layout(
                                            egui::Layout::right_to_left(
                                                egui::Align::Center,
                                            ),
                                            |ui| {
                                                ui.add_sized(
                                                    [56.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(size_txt)
                                                            .monospace()
                                                            .size(11.5),
                                                    ),
                                                );
                                            },
                                        );
                                        // KaReferenceStatusDialog.cpp:110 — Offline 아니면 warnMark(#7A5A00) 표시
                                        let (status_prefix, status_col) = if is_offline {
                                            ("● ", Color32::from_rgb(21, 128, 61))
                                        } else {
                                            ("▲ ", Color32::from_rgb(0x7A, 0x5A, 0x00))
                                        };
                                        ui.add_sized(
                                            [150.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(format!(
                                                    "{status_prefix}{offline_txt}"
                                                ))
                                                .size(11.0)
                                                .strong()
                                                .color(status_col),
                                            )
                                            .truncate(),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui.small_button("받기").clicked() {
                                                next_modal = Some(dl_modal);
                                            }
                                            if ui.small_button("설정").clicked() {
                                                next_modal = Some(cfg_modal);
                                            }
                                        });
                                        ui.end_row();
                                    });
                            });
                    }
                });

            if let Some(target_modal) = next_modal {
                app.modals.active = Some(target_modal);
            }

            ui.add_space(6.0);
            ui.separator();
            // KaReferenceStatusDialog.cpp:80-88 — 하단 [다시 확인] / [닫기] 버튼
            ui.horizontal(|ui| {
                if ui.button("다시 확인").clicked() {
                    app.home_map.draw_guide_message =
                        Some(format!("자료 준비 상태 다시 확인 완료: {summary_txt}"));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        app.modals.active = None;
                    }
                });
            });
        });
}
