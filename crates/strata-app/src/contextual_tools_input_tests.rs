//! Connected tools and retained state, target/cpp-ref/src/app/MainWindowRibbon.cpp:798-952.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, PointerButton, Rect, Shape};

fn start(raster: bool) -> Harness {
    let directory = tempfile::tempdir().expect("temporary survey");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = directory.path().join("demo.gpkg");
    std::fs::copy(root.join("samples/demo_survey/demo.gpkg"), &path).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = String::new;
    assert!(app.open_survey_gpkg(&path));
    assert_eq!(app.work_crs.auth_id(), "EPSG:5187");
    app.active_tab = MainTab::Map;
    if raster {
        let image = directory.path().join("scan.png");
        std::fs::copy(root.join("data/theme/ka-hgis-app.png"), &image).expect("tracked first-party image");
        app.add_raster_from_path(&image).expect("real raster row");
        app.right_inspector.selected_layer_key = Some(app.pictures.layers[0].key.clone());
    }
    let mut h = Harness::new(app, directory);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

#[path = "contextual_tools_input_tests_b.rs"]
mod bounds;
#[path = "contextual_tools_input_tests_a.rs"]
mod flows;

fn visible_texts(output: &egui::FullOutput) -> Vec<(String, Rect)> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t) => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                s.clip_rect.contains_rect(rect).then(|| (t.galley.text().to_owned(), rect))
            }
            _ => None,
        })
        .collect()
}

fn open(h: &mut Harness, action: &str, group: &str) -> Rect {
    let output = h.frame(vec![]);
    let compact = !visible_texts(&output).iter().any(|(s, _)| s == "새 조사");
    let origin = visible_texts(&output).iter().rev().find(|(s, _)| s == if compact { group } else { action }).expect("initiating caption").1.center();
    h.press(origin, PointerButton::Primary);
    if compact {
        let output = h.frame(vec![]);
        let at = visible_texts(&output).iter().rev().find(|(s, _)| s == action).expect("compact action").1.center();
        h.press(at, PointerButton::Primary);
    }
    assert!(h.app.ribbon.sub_toolbar_visible, "real initiating click did not open the toolbar");
    let output = h.frame(vec![]);
    visible_texts(&output)
        .into_iter()
        .rev()
        .find(|(s, r)| s == if compact { group } else { action } && r.top() < 100.0)
        .expect("visible initiating control")
        .1
}

fn connected(h: &mut Harness, origin: Rect) {
    let output = h.frame(vec![]);
    // GUI logical points: 1 point allows the one-point painted stroke at the tail tip.
    assert!(
        output.shapes.iter().any(|s| match &s.shape {
            Shape::Path(p) if p.closed && p.points.len() == 3 && p.fill != egui::Color32::TRANSPARENT => {
                let tip = p.points.iter().min_by(|a, b| a.y.total_cmp(&b.y)).expect("triangle");
                tip.y >= origin.bottom() - 1.0 && tip.y <= origin.bottom() + 24.0 && (tip.x - origin.center().x).abs() <= origin.width() * 0.5 + 1.0
            }
            _ => false,
        }),
        "active tools have no visible connection to their initiating control"
    );
}

#[test]
fn context_tools_buffer_is_connected_to_its_button() {
    let mut h = start(false);
    let origin = open(&mut h, "버퍼", "기록");
    h.save("context-buffer-wide");
    connected(&mut h, origin);
    h.point_at("500m");
    h.point_at("1000m");
}

#[test]
fn context_tools_align_is_connected_and_has_five_reachable_actions() {
    let mut h = start(true);
    let path = h.app.survey_path.clone().expect("survey");
    let bytes = std::fs::read(&path).expect("before");
    let image = h.app.pictures.layers[0].file.clone();
    let original_image = std::fs::read(&image).expect("image");
    let records = super::vertex_crs_input_tests::records(&h);
    let origin = open(&mut h, "정합", "정합");
    h.save("context-align-wide");
    connected(&mut h, origin);
    for action in ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"] {
        let at = h.point_at(action);
        h.press(at, PointerButton::Primary);
        if action == "이동" {
            assert!(h.app.status_left_message().contains("점을 2개 이상"));
        }
        if action == "맞추기 저장" {
            assert!(h.app.status_left_message().contains("점을 2곳 이상"));
        }
    }
    assert!(!h.app.ribbon.sub_toolbar_visible);
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::Pan);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
    assert_eq!(std::fs::read(image).expect("image after"), original_image);
    assert_eq!(super::vertex_crs_input_tests::records(&h), records);
}

#[test]
fn context_tools_draw_close_is_reachable_at_800() {
    let mut h = start(false);
    h.size[0] = 800;
    open(&mut h, "그리기", "기록");
    h.save("context-draw-800");
    let at = h.point_at("닫기");
    h.press(at, PointerButton::Primary);
    assert!(!h.app.draw_row_open());
}

#[test]
fn context_tools_ribbon_captions_fit_at_1280() {
    let mut h = start(false);
    h.size[0] = 1280;
    let output = h.frame(vec![]);
    h.save("context-ribbon-1280");
    let captions: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t) if t.pos.y < 100.0 => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                Some((t.galley.text(), rect, s.clip_rect))
            }
            _ => None,
        })
        .collect();
    for (text, rect, clip) in &captions {
        assert!(clip.contains_rect(*rect), "ribbon caption clipped: {text}");
    }
    for (i, (a, ra, _)) in captions.iter().enumerate() {
        for (b, rb, _) in &captions[i + 1..] {
            assert!(!ra.intersects(*rb), "ribbon captions overlap: {a}, {b}");
        }
    }
}
