//! 리본 실측 상수·7그룹 정의·그룹 배치 계산·안내 문구.

use eframe::egui::Vec2;
/// UX_LAYOUT_SPEC.md 1·2절: C++ 실측 상수 (창 1600×900 기준)
pub const UX_RIBBON_HEIGHT: f32 = 91.0;
pub const UX_VIEW_TAB_BAR_HEIGHT: f32 = 51.0;
pub const UX_RIBBON_BUTTON_SIZE: Vec2 = Vec2::new(69.0, 100.0);

/// Seven ribbon groups (target/cpp-ref/src/app/MainWindowRibbon.cpp:248-254) and their chip labels;
/// group 「자료 받기」 (:250) holds DEM (:402), 지적 (:472), 수치지형 (:385, :484), 유산 (:544, :550).
pub const UX_RIBBON_GROUPS: [(&str, &[&str]); 7] = [
    ("조사", &["새 조사", "열기", "저장", "다른 이름"]),
    ("기록", &["선택", "측거", "그리기", "시굴격자", "버퍼"]),
    ("자료 받기", &["DEM", "지적", "수치지형", "유산"]),
    (
        "배경 지도",
        &["지형", "등고선", "토양", "고지형", "지질", "수계", "옛 지도"],
    ),
    ("정합", &["정합"]),
    ("내보내기", &["도면", "인쇄", "GeoTIFF", "검수·제출"]),
    ("기타", &["더보기"]),
];
/// Retained scale factor; target/cpp-ref/src/app/KaTheme.h:97-98 defines ribbonChipWidth = 40 (restored in a separate order).
pub const RIBBON_CHIP_SCALE: f32 = 36.0 / 40.0;
/// `439-449` — 한 칩의 기준 폭(px, 12.5px 라벨 실측값). 배율을 곱하기 전 값이다.
#[must_use]
pub fn ribbon_chip_base_width(group_idx: usize, label: &str) -> f32 {
    match (group_idx, label) {
        (0, _) => 53.0,
        (1, "그리기" | "시굴격자") => 51.0,
        (1, _) => 47.0,
        (2, "수치지형") => 60.0,
        (2, _) => 47.0,
        (3, "옛 지도") => 51.0,
        (3, _) => 47.0,
        (4, _) => 47.0,
        (5, "GeoTIFF" | "검수·제출") => 60.0,
        (5, _) => 47.0,
        _ => 48.0,
    }
}
/// 7그룹의 `(시작 x, 폭)` 을 `UX_RIBBON_GROUPS` 에서 계산한다. 그룹 폭은 칩 폭 합 + 좌우 여백 2px.
/// 표를 손으로 고치지 않으므로 단추를 더해도 배치가 어긋나지 않는다.
#[must_use]
pub fn ribbon_group_layout() -> ([f32; 7], [f32; 7]) {
    let mut xs = [0.0_f32; 7];
    let mut widths = [0.0_f32; 7];
    let mut x = 9.0_f32;
    for (g_idx, (_, items)) in UX_RIBBON_GROUPS.iter().enumerate() {
        let sum: f32 = items
            .iter()
            .map(|label| ribbon_chip_base_width(g_idx, label) * RIBBON_CHIP_SCALE)
            .sum();
        xs[g_idx] = x;
        widths[g_idx] = sum + 2.0;
        x += widths[g_idx];
    }
    (xs, widths)
}
/// C++ `KaBeginnerRibbon` 은 폭을 재서 배치하므로 고정 표가 없다.
/// Rust 는 1600×900 목업 기준표(`UX_RIBBON_GROUP_XS`/`_WIDTHS`)를 같은 방식으로 계산해 쓴다.
#[must_use]
pub fn ux_ribbon_group_xs() -> [f32; 7] {
    ribbon_group_layout().0
}
#[must_use]
pub fn ux_ribbon_group_widths() -> [f32; 7] {
    ribbon_group_layout().1
}
/// `KaRegionLocator.cpp:156, 162-163, 192-194` — 동·읍·면·리 / 지번 칸 안내와 맨 아래 규칙 두 줄
pub const REGION_LOT_PLACEHOLDER: &str = "산 12-3";
pub const REGION_LOT_HINT: &str = "번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요.";
pub const REGION_BOTTOM_HINT: &str =
    "번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3";
/// `MainWindowAlign.cpp:721` — 정합 서브툴바 맨 앞 캡션 문구
pub const ALIGN_SUBTOOLBAR_CAPTION: &str = "  맞추기 › ";
/// `MainWindowAlign.cpp:724,726,730,745,793` — 정합 서브툴바 다섯 항목과 그 순서.
/// C++ 에는 이 다섯 개만 있다(기준점 CSV·CAD DXF 가져오기·품질 요약 줄 없음).
pub const ALIGN_SUBTOOLBAR_ITEMS: [&str; 5] =
    ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"];
