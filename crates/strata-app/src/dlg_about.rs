//! 「정보」 — `QMessageBox::about` (target/cpp-ref/src/app/MainWindow.cpp:3048-3061).
//!
//! Sentences that name libraries this build does not link are left out: the QGIS link line
//! (`MainWindow.cpp:3053`), the Qt lines of `KaStartupSplash::attributionText`
//! (`KaStartupSplash.cpp:81-83`), the LibreDWG line (`MainWindow.cpp:3058`) and the crash-log
//! hint (`KaSessionLog.cpp:102`). This build does not link QGIS or Qt, does not ship LibreDWG,
//! and does not write `session.log` (`THIRD_PARTY_NOTICES.md`).

use eframe::egui::{self, Color32};

use crate::dialogs_modal::render_strata_modal_header;

/// The notice body. Version is this crate's version (`KaHgisVersion.h` is `2.0.0` in the C++ tree).
/// `ka-hgis <version> · 커밋 unknown` is `KaSessionLog::buildLabel` (`KaSessionLog.cpp:147-149`)
/// without the ` · QGIS %3` tail, and with the hash fallback from `KaGitHash.h:5`.
#[must_use]
pub(crate) fn about_notice_text() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "Strata · 필드고고학 GIS  v{version}\n\
         ka-hgis {version} · 커밋 unknown\n\
         동국문화재연구원 · 만든이: 권영인 · 조유량 · 박종환\n\n\
         작업 좌표계: EPSG:5186/5187 · 제출: EPSG:5179\n\n\
         저작권·라이선스\n\
         GDAL/OGR  © OSGeo 및 기여자 · MIT 계열\n\
         PROJ  © PROJ contributors · MIT\n\
         GEOS  © GEOS contributors · LGPLv2.1\n\
         SQLite · Public domain\n\
         지도·자료: VWorld · 국토정보플랫폼 · 국가유산 공간정보 · 흙토람 · KIGAM · 국사편찬위원회 등\n\
         지도·자료 및 PROJ 데이터는 제공처의 저작권·이용조건을 따릅니다.\n\n\
         본 소프트웨어는 GNU GPL v2 이상으로 배포됩니다.\n\
         자세한 의존 고지는 앱 폴더의 THIRD_PARTY_NOTICES.md를 봅니다.\n"
    )
}

pub(crate) fn render_about(ctx: &egui::Context, open: &mut bool) {
    // QMessageBox::about is application-modal: a click outside does not reach the map and does not
    // dismiss the box. Escape closes it (the OK button is the default). No shade: the C++ box does not dim the app.
    let mut dismiss = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    egui::Modal::new(egui::Id::new("ka_modal_about"))
        .backdrop_color(Color32::TRANSPARENT)
        .frame(egui::Frame::window(&ctx.style()))
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            render_strata_modal_header(ui, "정보", open);
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                ui.label(egui::RichText::new(about_notice_text()).size(13.0).color(Color32::from_rgb(30, 30, 28)));
            });
            ui.add_space(8.0);
            if ui.button("확인").clicked() {
                dismiss = true;
            }
        });
    if dismiss {
        *open = false;
    }
}
