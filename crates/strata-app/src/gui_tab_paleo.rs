//! `gui_tab_paleo` 는 「고지형」 탭 렌더링의 재수출 창구다.
//!
//! 원래 741줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::gui_tab_paleo::...` 경로와 `create`/`render_paleo_tab_ui` 동작은 그대로다.
//! 세 절(우측 패널 → 좌측 패널 → 중앙 분류도 → 중앙 단면도)의 순서는 원문 그대로다.

use eframe::egui;

use crate::app_state::StrataAppState;

#[path = "gui_tab_paleo_map.rs"]
mod gui_tab_paleo_map;
#[path = "gui_tab_paleo_panels.rs"]
mod gui_tab_paleo_panels;
#[path = "gui_tab_paleo_section.rs"]
mod gui_tab_paleo_section;

impl StrataAppState {
    /// `KaPaleoStudio.cpp:36-291` & `KaPaleoCrossSectionWidget.cpp:27-97` — 3단 분할(`layersCard` | `centerSplit`[7대 고지형 분류도 + 시추공 주상도 단면도] | `rightScroll`[6대 자료·7대 범례·요약])
    pub(crate) fn render_paleo_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 우측 스크롤 패널 (KaPaleoStudio.cpp:123-204)
        self.render_paleo_right_panel(ctx, ui);

        // 2. 좌측 레이어 패널 (KaPaleoStudio.cpp:51-99)
        self.render_paleo_layer_panel(ui);

        // 3. 중앙 수직 분할 (상단: KaPaleoStudio::renderMapPreview 7색 분류도 + 시추공·단면선 / 하단: KaPaleoCrossSectionWidget 지층 단면도 + 시추공 주상도)
        self.render_paleo_center(ctx, ui);
    }

    /// 중앙 패널 — 위에서 아래로 (A) 분류도 → (B) 단면도 순서를 지킨다.
    fn render_paleo_center(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_paleo_map_preview(ctx, ui);
            self.render_paleo_cross_section(ui);
        });
    }
}
