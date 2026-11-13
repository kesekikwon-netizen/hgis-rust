//! F1c-3 — ribbon 「GeoTIFF」 (R7) is deferred: muted on every tab, deferred hover text, a click opens no
//! dialog, writes no file and sets no message. cpp writes a real map GeoTIFF (MainWindowExport.cpp:58-96 +
//! MapGeoTiffExport.cpp); the Rust writer made a header-only TIFF.

use crate::app_state::{MainTab, StrataAppState};

#[test]
fn geotiff_chip_is_deferred_r7_and_click_does_nothing() {
    use super::super::gui_ribbon_draw::deferred_ribbon_tooltip;
    assert_eq!(deferred_ribbon_tooltip("GeoTIFF"), Some("아직 Rust 판에서 연결하지 않았습니다 (지시 R7)"));
    assert_eq!(deferred_ribbon_tooltip("인쇄"), None);
    // Map tab with no layers and the drawing tab both stop before the save dialog in the old code.
    for tab in [MainTab::Map, MainTab::Drawing] {
        let mut app = StrataAppState::new_startup();
        app.active_tab = tab;
        app.handle_ribbon_click("GeoTIFF");
        assert_eq!(app.home_map.draw_guide_message, None, "no message");
        assert!(app.modals.active.is_none(), "no dialog");
        assert!(app.active_tab == tab, "tab unchanged");
    }
}
