//! 홈 「최근 조사」·마지막 조사 폴더 — C++ rememberSurvey·rememberSurveyDir·preferredSurveyDir·
//! openRecentSurvey 와 같은지. 사용자 ini 는 건드리지 않고 임시 폴더의 ini 만 붙인다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_storage::recent_surveys::{absolute_file_path, last_dir, load_stored};

use super::preferred_survey_dir_from;
use crate::app_state::{MainTab, StrataAppState};

fn demo_copy(dir: &Path, name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dst = dir.join(name);
    std::fs::copy(src, &dst).expect("copy demo.gpkg to a temp folder");
    dst
}

/// 사용자 ini 와 같은 모양(CRLF, 앞뒤 절, facts 키, 없는 조사 세 줄).
fn shaped_ini(items: &str) -> String {
    format!(
        "[MainWindow]\r\nstate=@ByteArray(\\0\\x1)\r\n\r\n[RecentSurveys]\r\nfactsOrder=a\r\nfacts\\0b2aeb2b959bb5e9=1, 0\r\n\
         skipAutoRestore=false\r\nitems={items}\r\n\r\n[ui]\r\nlook=paper\r\n\r\n[Survey]\r\nLastDir=C:/nowhere-strata\r\n\r\n\
         [network]\r\nproxy=none\r\n"
    )
}

const MISSING: &str = r"C:/nowhere-strata/verify1-저장.gpkg\tverify1-저장\t1791667851245, C:/nowhere-strata/verify1.gpkg\tverify1\t1791635164245, C:/nowhere-strata/andong/andong.gpkg\tandong\t1791630752776";

fn app_with_ini(dir: &Path, items: &str) -> (StrataAppState, PathBuf) {
    let ini = dir.join("profiles").join("ka-hgis.ini");
    std::fs::create_dir_all(ini.parent().expect("parent")).expect("dir");
    std::fs::write(&ini, shaped_ini(items)).expect("ini");
    let mut app = StrataAppState::new_startup();
    app.attach_user_settings(Some(ini.clone()));
    (app, ini)
}

/// 시작하면 조사 자료가 하나도 없다(경주시 견본 없음). 시험의 상태는 사용자 ini 를 붙이지 않는다.
#[test]
fn startup_has_no_survey_data_and_no_user_settings() {
    let app = StrataAppState::new_startup();
    assert!(app.settings_ini.is_none(), "only main() attaches the user ini");
    assert!(app.recent_surveys.is_empty());
    assert!(app.survey_path.is_none() && app.survey_tables.is_empty());
    assert_eq!(app.window_title(), "Strata");
    let rs = &app.home_map.real_survey;
    assert!(rs.survey_rings.is_empty() && rs.trenches.is_empty());
    assert!(rs.cadastral_parcels.is_empty() && rs.heritage_polys.is_empty() && rs.buffer_500m.is_empty());
    assert!(app.home_map.scene.polygons.is_empty() && app.home_map.scene.heritage_badges_world.is_empty());
    assert!(app.home_map.korea_view_pending, "the first map view is all of Korea (ensureStartupViewReady)");
    assert!(app.left_layers.entries.is_empty());
}

/// 홈 목록은 저장된 차례 그대로, 없는 조사는 「원본 없음」(available=false), 이어서 작업은 열 수 있는 첫 조사.
#[test]
fn home_list_follows_the_store_and_continue_skips_missing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let here = demo_copy(dir.path(), "경주시.gpkg");
    let items = format!(r"C:/nowhere-strata/first.gpkg\tfirst\t9, {}\t경주시\t1794142119965, {MISSING}", absolute_file_path(&here));
    let (app, _) = app_with_ini(dir.path(), &items);
    let names: Vec<(&str, bool)> = app.recent_surveys.iter().map(|it| (it.name.as_str(), it.available)).collect();
    assert_eq!(
        names,
        [("first", false), ("경주시", true), ("verify1-저장", false), ("verify1", false), ("andong", false)]
    );
    assert_eq!(app.continue_survey().map(|it| it.name.as_str()), Some("경주시"));
}

/// 열기에 성공하면 그 조사가 맨 앞에, `Survey/LastDir` 은 그 폴더 — ini 의 다른 바이트는 그대로.
#[test]
fn opening_remembers_the_survey_and_its_folder() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, ini) = app_with_ini(dir.path(), MISSING);
    let before = std::fs::read_to_string(&ini).expect("read");
    let demo = demo_copy(dir.path(), "demo.gpkg");

    assert!(app.open_survey_gpkg(&demo));

    let stored = load_stored(&ini);
    assert_eq!(stored.len(), 4);
    assert_eq!((stored[0].path.as_str(), stored[0].name.as_str()), (absolute_file_path(&demo).as_str(), "demo"));
    let folder = absolute_file_path(dir.path());
    assert_eq!(last_dir(&ini), folder);
    assert_eq!(app.recent_surveys.first().map(|it| it.name.as_str()), Some("demo"));
    assert!(dir.path().join("지형도").is_dir());
    let after = std::fs::read_to_string(&ini).expect("read");
    let changed: Vec<(&str, &str)> = before.lines().zip(after.lines()).filter(|(a, b)| a != b).collect();
    assert_eq!(changed.len(), 2, "only items and LastDir change: {changed:?}");
    assert!(changed[0].1.starts_with("items=") && changed[1].1 == format!("LastDir={folder}"));
}

/// 없는 조사 줄을 누르면 알리고 목록은 그대로(지우지 않는다). 있는 줄은 연다.
#[test]
fn recent_rows_open_or_warn_like_open_recent_survey() {
    let dir = tempfile::tempdir().expect("temp dir");
    let demo = demo_copy(dir.path(), "demo.gpkg");
    let items = format!(r"{}\tdemo\t5, {MISSING}", absolute_file_path(&demo));
    let (mut app, ini) = app_with_ini(dir.path(), &items);

    app.open_recent_survey("C:/nowhere-strata/verify1.gpkg");
    assert!(app.survey_path.is_none());
    let msg = app.home_map.draw_guide_message.clone().unwrap_or_default();
    assert!(msg.contains("조사 파일을 찾을 수 없습니다") && msg.contains(r"C:\nowhere-strata\verify1.gpkg"), "{msg}");
    assert_eq!(load_stored(&ini).len(), 4);

    app.open_recent_survey(&absolute_file_path(&demo));
    assert_eq!(app.survey_path.as_deref(), Some(demo.as_path()));
    assert_eq!(app.active_tab, MainTab::Map);
}

/// 오른쪽 단추 「목록에서 제거」 — 그 줄만 빠진다.
#[test]
fn forgetting_a_row_removes_only_that_survey() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, ini) = app_with_ini(dir.path(), MISSING);
    app.forget_recent_survey("C:/nowhere-strata/verify1.gpkg");
    let names: Vec<String> = load_stored(&ini).into_iter().map(|it| it.name).collect();
    assert_eq!(names, ["verify1-저장", "andong"]);
    assert_eq!(app.recent_surveys.len(), 2);
}

/// 새 조사·저장도 C++ 처럼 목록 맨 앞과 마지막 폴더를 바꾼다(`MainWindowSession.cpp:741-742, :963-964`).
#[test]
fn new_survey_and_save_remember_like_cpp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, ini) = app_with_ini(dir.path(), MISSING);
    let site = dir.path().join("현장");
    std::fs::create_dir_all(&site).expect("site dir");
    let path = site.join("새 조사.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, app.modals.new_survey_crs).expect("create");
    app.start_new_survey_project_with_path("이름표", &path, app.modals.new_survey_crs);
    let stored = load_stored(&ini);
    assert_eq!((stored[0].name.as_str(), stored.len()), ("이름표", 4));
    assert_eq!(last_dir(&ini), absolute_file_path(&site));
    assert!(site.join("지형도").is_dir());

    assert!(app.save_survey_project());
    assert_eq!(load_stored(&ini)[0].name, "새 조사", "save uses the file name");
}

/// `preferredSurveyDir` — 마지막 폴더 → 열린 조사 폴더 → 최근 조사 맨 위의 폴더 → 바탕 화면.
#[test]
fn preferred_survey_dir_follows_cpp_order() {
    let root = tempfile::tempdir().expect("temp dir");
    let (a, b, c) = (root.path().join("a"), root.path().join("b"), root.path().join("c"));
    for d in [&a, &b, &c] {
        std::fs::create_dir_all(d).expect("dir");
    }
    let s = |p: &Path| p.to_string_lossy().replace('\\', "/");
    let survey = b.join("s.gpkg");
    let recent = s(&c.join("r.gpkg"));
    let desk = || PathBuf::from("DESK");
    assert_eq!(preferred_survey_dir_from(&s(&a), Some(&survey), &recent, desk), PathBuf::from(s(&a)));
    let gone = s(&root.path().join("gone"));
    assert_eq!(preferred_survey_dir_from(&gone, Some(&survey), &recent, desk), PathBuf::from(absolute_file_path(&b)));
    assert_eq!(preferred_survey_dir_from("", None, &recent, desk), PathBuf::from(s(&c)));
    assert_eq!(preferred_survey_dir_from("", None, "", desk), PathBuf::from("DESK"));
}

/// S0 — `--snap` 은 사용자 ini 를 읽기만 한다: 홈은 실제 목록을 보이고, 스냅의 열기는 임시 폴더 사본에만
/// 기록한다. 시험의 「사용자 ini」도 임시 폴더의 가짜 파일이다.
#[test]
fn snap_reads_the_user_ini_but_writes_only_a_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let real = dir.path().join("real").join("ka-hgis.ini");
    std::fs::create_dir_all(real.parent().expect("parent")).expect("dir");
    std::fs::write(&real, shaped_ini(MISSING)).expect("ini");
    let before = std::fs::read(&real).expect("read");
    let snap_dir = dir.path().join("snap");
    let mut app = StrataAppState::new_startup();

    app.attach_user_settings_for_snap(Some(real.clone()), &snap_dir);

    let names: Vec<&str> = app.recent_surveys.iter().map(|it| it.name.as_str()).collect();
    assert_eq!(names, ["verify1-저장", "verify1", "andong"], "home snaps show the real list");
    let demo = demo_copy(dir.path(), "demo.gpkg");
    app.snap_apply_action(&format!("@open={}", demo.display()));
    assert_eq!(app.survey_path.as_deref(), Some(demo.as_path()));
    assert_eq!(std::fs::read(&real).expect("read"), before, "the user ini is never written");
    assert_eq!(app.recent_surveys.first().map(|it| it.name.as_str()), Some("demo"));
    assert_eq!(last_dir(&snap_dir.join("ka-hgis.ini")), absolute_file_path(dir.path()));
}

/// 사용자 ini 가 없거나 복사하지 못하면 빈 사본으로 — 그래도 사용자 쪽에는 아무 것도 만들지 않는다.
#[test]
fn snap_without_a_user_ini_creates_nothing_there() {
    let dir = tempfile::tempdir().expect("temp dir");
    let real = dir.path().join("real").join("ka-hgis.ini");
    let mut app = StrataAppState::new_startup();
    app.attach_user_settings_for_snap(Some(real.clone()), &dir.path().join("snap"));
    assert!(app.recent_surveys.is_empty());
    let demo = demo_copy(dir.path(), "demo.gpkg");
    app.snap_apply_action(&format!("@open={}", demo.display()));
    assert!(!real.exists() && !dir.path().join("real").exists());
}
