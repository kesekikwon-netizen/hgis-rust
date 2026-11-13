//! 레이어 메뉴 줄 — C++ showLayerTreeContextMenu(:176-448) 조건과 docs/senior/context-menu-inventory.md 3-2 가지별 줄.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::layer_menu_label::{describe_label, LabelSetting};

fn input(key: &str, geom: Option<MenuGeom>, reference: bool) -> LayerMenuInput<'static> {
    let key: &'static str = Box::leak(key.to_owned().into_boxed_str());
    let vector = geom.is_some();
    let fields: Vec<String> = ["fid", "kind", "period", "feature_no", "note"].iter().map(|s| (*s).to_owned()).collect();
    let label = describe_label(&LabelSetting::Area(None), &fields, vector, geom == Some(MenuGeom::Polygon), key, false);
    LayerMenuInput {
        layer_key: key,
        kind: menu_kind(key, vector, reference),
        vector,
        valid: true,
        feature_count: 2,
        geom,
        busy: false,
        survey_open: true,
        own_survey: !reference,
        read_only: reference,
        reference_or_cadastral: reference,
        shp: false,
        xyz: false,
        at_group_bottom: false,
        opacity: 1.0,
        label,
        has_labeling: vector,
        labels_visible: true,
        fields,
        current_label_field: "feature_no".to_owned(),
        show_area: true,
        label_size_pt: 8.0,
    }
}

fn texts(entries: &[MenuEntry]) -> Vec<String> {
    entries.iter().filter_map(|e| if let MenuEntry::Row(r) = e { Some(r.text.clone()) } else { None }).collect()
}

/// 한 줄 요약: 하위 메뉴는 `>`, 구분선 `|`, 흐림 `~`.
fn outline(entries: &[MenuEntry]) -> String {
    entries
        .iter()
        .map(|e| match e {
            MenuEntry::Separator => "|".to_owned(),
            MenuEntry::Row(r) => format!("{}{}{}", if r.enabled { "" } else { "~" }, if r.submenu.is_some() { ">" } else { "" }, r.text),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn find<'a>(entries: &'a [MenuEntry], id: &str) -> &'a MenuRow {
    entries.iter().find_map(|e| if let MenuEntry::Row(r) = e { (r.id == id).then_some(r) } else { None }).expect(id)
}

const AREA_ROWS: [&str; 17] = [
    "레이어 불러오기 (SHP·DXF·GPKG)", "이름 바꾸기 (두 번 클릭)", "면·외곽선 색", "도형 속성", "라벨·명칭 속성", "투명도", "이 레이어로 이동", "전체 보기",
    "같은 묶음의 맨 아래로", "그리기·편집 시작", "꼭짓점 수정", "폴리곤 묶기", "면적 확인", "시굴격자", "제출 꾸러미 만들기…", "그린 도형 모두 지우기…", "레이어 삭제",
];

/// inventory 3-2 가지별 줄 — 조사구역 17, 유구 면 16(시굴격자 없음), 유구 선·단면선 14, 유물·기준점 13, 시굴 16(「면적·조사 비율 확인」).
#[test]
fn each_survey_branch_shows_the_cpp_rows() {
    let rows = |key: &str, g: MenuGeom| texts(&layer_menu_entries(&input(key, Some(g), false)));
    assert_eq!(rows("survey_area", MenuGeom::Polygon), AREA_ROWS.to_vec());
    let poly: Vec<&str> = AREA_ROWS.iter().copied().filter(|t| *t != "시굴격자").collect();
    assert_eq!(rows("feature_poly", MenuGeom::Polygon), poly);
    let trench: Vec<&str> = poly.iter().map(|t| if *t == "면적 확인" { "면적·조사 비율 확인" } else { t }).collect();
    assert_eq!(rows("trial_trench", MenuGeom::Polygon), trench);
    let line: Vec<&str> = poly.iter().copied().filter(|t| !["폴리곤 묶기", "면적 확인"].contains(t)).collect();
    assert_eq!(rows("feature_line", MenuGeom::Line), line);
    assert_eq!(rows("section_line", MenuGeom::Line), line);
    let point: Vec<&str> = line.iter().copied().filter(|t| *t != "꼭짓점 수정").collect();
    assert_eq!(rows("artifact_point", MenuGeom::Point), point);
    assert_eq!(rows("control_points", MenuGeom::Point), point);
    assert_eq!(rows("survey_area_2", MenuGeom::Polygon).len(), 17, "a named survey area table is still 조사구역");
}

/// 구분선은 C++ 자리(Qt 처럼 겹친 것·앞뒤 것은 숨김), 아직 잇지 않은 줄은 흐림.
#[test]
fn separators_and_deferred_rows() {
    let m = layer_menu_entries(&input("survey_area", Some(MenuGeom::Polygon), false));
    assert_eq!(
        outline(&m),
        "~레이어 불러오기 (SHP·DXF·GPKG) | 이름 바꾸기 (두 번 클릭) 면·외곽선 색 ~도형 속성 >라벨·명칭 속성 >투명도 | 이 레이어로 이동 전체 보기 같은 묶음의 맨 아래로 | \
         그리기·편집 시작 ~꼭짓점 수정 ~폴리곤 묶기 면적 확인 ~>시굴격자 | ~제출 꾸러미 만들기… | ~그린 도형 모두 지우기… 레이어 삭제"
    );
    assert_eq!(find(&m, "layer.attributes").tooltip, "아직 Rust 판에서 연결하지 않았습니다 (지시 CM2a-3)");
    assert_eq!(find(&m, "layer.export").tooltip, "아직 Rust 판에서 연결하지 않았습니다 (지시 R2)");
    assert_eq!(find(&m, "layer.trenchCreate").tooltip, "아직 Rust 판에서 연결하지 않았습니다 (지시 W-T1)");
    assert_eq!(find(&m, "layer.remove").tooltip, "목록에서 제거합니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다.");
}

/// 도형 없는 레이어·맨 아래·처리 중 — C++ 흐림 이유. 아직 잇지 않은 줄도 C++ 이유가 있으면 그것.
#[test]
fn disabled_reasons_follow_cpp() {
    let mut i = input("feature_poly", Some(MenuGeom::Polygon), false);
    i.feature_count = 0;
    i.at_group_bottom = true;
    let m = layer_menu_entries(&i);
    for id in ["layer.zoom", "layer.area", "layer.vertices"] {
        assert_eq!(find(&m, id).tooltip, "이 레이어에는 도형이 없습니다.", "{id}");
        assert!(!find(&m, id).enabled);
    }
    assert_eq!(find(&m, "layer.bottom").tooltip, "이 묶음에서 이미 맨 아래에 있습니다.");
    i.busy = true;
    let m = layer_menu_entries(&i);
    assert_eq!(find(&m, "layer.remove").tooltip, "조사 파일 처리가 끝난 뒤 다시 실행하세요.");
    assert_eq!(find(&m, "layer.draw").tooltip, "조사 파일 처리가 끝난 뒤 다시 실행하세요.");
}

/// 라벨 ▸: 「면적 숨기기」, 표시할 항목(fid 포함, 체크 = 지금 항목), 면적(㎡) 함께 표시(체크), 글자 크기 10개(체크 = 8 pt). 투명도 6개.
#[test]
fn label_and_opacity_submenus() {
    let mut i = input("feature_poly", Some(MenuGeom::Polygon), false);
    i.opacity = 0.6;
    let m = layer_menu_entries(&i);
    let labels = find(&m, "layer.labels").submenu.clone().expect("labels");
    assert_eq!(outline(&labels), "면적 숨기기 >표시할 항목 면적(㎡) 함께 표시 >글자 크기");
    let fields = find(&labels, "layer.labelField").submenu.clone().unwrap();
    assert_eq!(texts(&fields), vec!["fid", "종류", "시대", "유구 번호", "비고"]);
    assert_eq!(find(&fields, "layer.labelFieldValue").checked, Some(false));
    let sizes = find(&labels, "layer.labelSize").submenu.clone().unwrap();
    assert_eq!(texts(&sizes), vec!["3 pt", "4 pt", "5 pt", "6 pt", "7 pt", "8 pt", "9 pt", "10 pt", "12 pt", "14 pt"]);
    let checked: Vec<String> = sizes.iter().filter_map(|e| if let MenuEntry::Row(r) = e { (r.checked == Some(true)).then(|| r.text.clone()) } else { None }).collect();
    assert_eq!(checked, vec!["8 pt"]);
    let opacity = find(&m, "layer.opacity");
    assert_eq!(opacity.tooltip, "지도에서만 흐리게 보입니다. 자료와 색은 바뀌지 않습니다.");
    let rows = opacity.submenu.clone().unwrap();
    assert_eq!(texts(&rows), vec!["0% 투명하게", "20% 투명하게", "40% 투명하게", "50% 투명하게", "60% 투명하게", "80% 투명하게"]);
    let on: Vec<String> = rows.iter().filter_map(|e| if let MenuEntry::Row(r) = e { (r.checked == Some(true)).then(|| r.text.clone()) } else { None }).collect();
    assert_eq!(on, vec!["40% 투명하게"]);
}

/// 항목이 10개를 넘으면 「항목 1–10」 묶음, 90개 넘으면 「다음 항목…」.
#[test]
fn many_fields_are_grouped() {
    let mut i = input("feature_poly", Some(MenuGeom::Polygon), false);
    i.fields = (0..95).map(|n| format!("f{n}")).collect();
    let m = layer_menu_entries(&i);
    let labels = find(&m, "layer.labels").submenu.clone().unwrap();
    let fields = find(&labels, "layer.labelField").submenu.clone().unwrap();
    let t = texts(&fields);
    assert_eq!(t.first().map(String::as_str), Some("항목 1–10"));
    assert_eq!(t[8], "항목 81–90");
    assert_eq!(t.last().map(String::as_str), Some("다음 항목…"));
    let next = find(&fields, "layer.labelFieldsNext").submenu.clone().unwrap();
    assert_eq!(texts(&next), vec!["항목 91–95"]);
}

/// 참조 래스터(위성): 색·속성·라벨 흐림, 편집 줄 없음. 참조 벡터 면(지적): 편집 줄 없이 면적 확인만, .shp 면 깨진 한글. 깨진 한글은 GPKG 에 없다.
#[test]
fn reference_branches() {
    let mut sat = input("satellite", None, true);
    sat.xyz = true;
    let m = layer_menu_entries(&sat);
    assert_eq!(
        outline(&m),
        // :408 — 편집 묶음 앞 구분선은 편집·면·조사구역일 때만, 래스터는 「오프라인 저장」이 바로 붙는다.
        "~레이어 불러오기 (SHP·DXF·GPKG) | 이름 바꾸기 (두 번 클릭) ~면·외곽선 색 ~도형 속성 ~>라벨·명칭 속성 >투명도 | 이 레이어로 이동 전체 보기 같은 묶음의 맨 아래로 ~오프라인 저장 — 지금 화면 범위 | 레이어 삭제"
    );
    assert_eq!(find(&m, "layer.labels").tooltip, "영상에 포함된 글자는 별도로 크기를 바꿀 수 없습니다.");
    let mut cad = input("cadastral", Some(MenuGeom::Polygon), true);
    cad.shp = true;
    let t = texts(&layer_menu_entries(&cad));
    assert!(t.contains(&"면적 확인".to_owned()) && t.contains(&"깨진 한글 바로잡기".to_owned()));
    assert!(!t.contains(&"그리기·편집 시작".to_owned()) && !t.contains(&"그린 도형 모두 지우기…".to_owned()));
    assert!(!texts(&layer_menu_entries(&input("feature_poly", Some(MenuGeom::Polygon), false))).contains(&"깨진 한글 바로잡기".to_owned()));
}

/// :195-213 — 레이어 아닌 줄: 지울 것이 있거나 묶음이면 「레이어 삭제」(묶음 툴팁), 늘 「참고 자료 불러오기…」.
#[test]
fn blank_and_group_rows() {
    assert_eq!(texts(&blank_menu_entries(false, false)), vec!["참고 자료 불러오기…"]);
    let g = blank_menu_entries(true, true);
    assert_eq!(texts(&g), vec!["레이어 삭제", "참고 자료 불러오기…"]);
    assert!(find(&g, "layer.remove").tooltip.starts_with("이 묶음과 안의 레이어를"));
}
