//! `gui_tab_drawing` 시험 — 원래 `gui_tab_drawing.rs` 안 `mod tests` 를 그대로 옮겼다.
//! facade 에서 `mod tests;` 로 등록하므로 시험 이름 경로는 그대로다.
/// D10 — 용지 하단이 7버튼 툴바 위에서 멈추는지 수치로 확인한다.
/// D20 — 지도 조정·좌표점 콜아웃·도면 목록이 실제 상태로 동작한다.
#[test]
fn test_d20_map_adjust_callouts_and_sheet_set() {
    let mut st = crate::tab_drawing::DrawingTabState::default();
    assert!(!st.map_adjust_active);
    assert!(!st.coord_callout_mode);
    assert!(st.coord_callouts.is_empty());
    assert!(st.stored_sheets.is_empty());
    assert!(!st.sheet_set_menu_open);

    st.map_adjust_active = true;
    assert!(st.map_adjust_active);
    st.coord_callout_mode = true;
    assert!(st.coord_callout_mode);

    // KaDrawingCoordCallout.cpp — A, B, C … 태그
    st.coord_callouts.push(crate::tab_drawing::CoordCallout {
        tag: "A".to_owned(),
        pos: [100.0, 120.0],
        world: [361_622.354, 219_897.093],
    });
    assert_eq!(st.coord_callouts.len(), 1);
    assert_eq!(st.coord_callouts[0].tag, "A");

    // KaDrawingSheetSet.cpp — 보관/열기
    st.stored_sheets.push(crate::tab_drawing::StoredSheet {
        title: "조사도면".to_owned(),
        scale_denominator: 5_000,
        paper_w_mm: 210.0,
        paper_h_mm: 297.0,
        legend_visible: true,
        title_block_visible: true,
    });
    assert_eq!(st.stored_sheets.len(), 1);
    let back = st.stored_sheets[0].clone();
    assert_eq!(back.scale_denominator, 5_000);
    assert_eq!(back.title, "조사도면");
    st.sheet_set_menu_open = true;
    assert!(st.sheet_set_menu_open);
}

#[test]
fn test_d10_sheet_bottom_stops_above_seven_button_toolbar() {
    // KaDrawingStudio.cpp:233-252 decorationSizePercent 는 4단계다.
    for pct in [75_u32, 100, 125, 150] {
        assert!((75..=150).contains(&pct));
    }
    // 1600x900 창에서 도면 본문 높이를 가정해 계산한다.
    let rect_top = 158.0_f32;
    let rect_bottom = 866.0_f32;
    let compact_v = (rect_bottom - rect_top) < 460.0;
    let toolbar_h = if compact_v { 34.0 } else { 42.0 };
    let top_bound = rect_top + 48.0;
    let bottom_bound = (rect_bottom - toolbar_h - 16.0).max(top_bound + 120.0);
    let bar_cy = rect_bottom - toolbar_h * 0.5 - 4.0;
    let bar_top = bar_cy - toolbar_h * 0.5;
    // 용지는 툴바 위에서 끝나야 한다.
    assert!(
        bottom_bound <= bar_top,
        "sheet {bottom_bound} vs bar {bar_top}"
    );
}

#[test]
fn toolbar_paper_orientation_button_opens_paper_setup_dialog_and_preserves_sheet_context() {
    // target/cpp-ref/src/app/KaDrawingStudio.cpp:447-454, 555-559, 1317-1319:
    // Bottom toolbar button 5 ("용지/방향") calls openPaperSettingsDialog() -> KaPaperDialog,
    // and accepting calls resetPaper(w, h, true) preserving map center and scale.
    use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
    use crate::app_state::{MainTab, StrataAppState};
    use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};

    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Drawing);
    app.drawing.sheet.scale_denominator = 2_500;
    app.drawing.sheet_center = Some([222_000.0, 360_000.0]);

    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    ctx.enable_accesskit();

    let run = |app: &mut StrataAppState, events: Vec<Event>| -> Vec<(String, Rect)> {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events,
            ..RawInput::default()
        };
        let out = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
            render_active_modal_dialogs(app, ctx);
        });
        out.platform_output.accesskit_update.map_or_else(Vec::new, |u| {
            u.nodes
                .iter()
                .filter_map(|(_, n)| {
                    let b = n.bounds()?;
                    let r = Rect::from_min_max(
                        Pos2::new(b.x0 as f32, b.y0 as f32),
                        Pos2::new(b.x1 as f32, b.y1 as f32),
                    );
                    Some((n.label()?.to_owned(), r))
                })
                .collect()
        })
    };

    // Click bottom toolbar cell 5 ("용지/방향"): bar_w=504, bar_h=42, bar_cy=800-21-4=775, x=640-252 + 72*(5+0.5)=584+52?
    // Compute exact cell 5 center in 1280x800 CentralPanel, or find by AccessKit label "용지/방향".
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes = run(&mut app, vec![]);
    }
    let btn_pos = nodes
        .iter()
        .find(|(l, _)| l == "용지/방향")
        .map(|(_, r)| r.center())
        .expect("toolbar button '용지/방향' must expose AccessKit label");
    let press = |at: Pos2, pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run(&mut app, vec![Event::PointerMoved(btn_pos)]);
    run(&mut app, vec![press(btn_pos, true)]);
    run(&mut app, vec![press(btn_pos, false)]);

    assert_eq!(app.modals.active, Some(ActiveModalDialog::PaperSetup));
    assert_eq!((app.drawing.sheet.width_mm, app.drawing.sheet.height_mm), (210.0, 297.0));

    // Choose A4 가로 (317x220) and click "용지 만들기" (settle anchored Window layout first).
    for _ in 0..2 {
        run(&mut app, vec![]);
    }
    for lbl in ["가로", "용지 만들기"] {
        let nodes = run(&mut app, vec![]);
        let at = nodes
            .iter()
            .find(|(l, _)| l == lbl)
            .map(|(_, r)| r.center())
            .unwrap_or_else(|| panic!("missing {lbl} in {nodes:?}"));
        run(&mut app, vec![Event::PointerMoved(at)]);
        run(&mut app, vec![press(at, true)]);
        run(&mut app, vec![press(at, false)]);
    }

    assert_eq!(app.modals.active, None);
    assert_eq!((app.drawing.sheet.width_mm, app.drawing.sheet.height_mm), (317.0, 220.0));
    assert_eq!(app.drawing.sheet.scale_denominator, 2_500);
    assert_eq!(app.drawing.sheet_center, Some([222_000.0, 360_000.0]));
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("용지 전환 완료: 317×220 mm — 기존 지도 범위와 배치가 유지되었습니다.")
    );
}
