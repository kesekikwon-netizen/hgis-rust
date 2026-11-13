//! C++ `KaBoreholeDialog.cpp` (`d7afcc8`) 의 문구·선택지·옵션 상태를 그대로 옮긴 데이터.
//!
//! 화면(egui)은 `crate::dialogs_borehole` 이 그린다. 이 파일은 C++ 원문 상수만 갖는다.

/// C++ `KaBoreholeDialog.cpp:81-84` `dlg->resize(640, 560)`.
pub const BOREHOLE_DIALOG_SIZE: eframe::egui::Vec2 = eframe::egui::Vec2::new(640.0, 560.0);

/// C++ `KaBoreholeDialog.cpp:79` 창 제목.
pub const BOREHOLE_DIALOG_TITLE: &str = "시추자료 전용 GIS 도면 설정 · 국토지반정보 주상도";

/// C++ `KaBoreholeDialog.cpp:88-91` 머리글.
pub const BOREHOLE_DIALOG_HEADER: &str = "시추자료 전용 5대 지반·고지형 GIS 도면 — 조사구역 주변 시추공 주상도를 별도로 내려받아 굴착 계획·문화층 깊이·배수 대책 도면을 생성합니다.";

/// C++ `KaBoreholeDialog.cpp:94` 반경 콤보 라벨.
pub const BOREHOLE_RADIUS_LABEL: &str = "조사구역 중심 수집 반경:";

/// C++ `KaBoreholeDialog.cpp:97-99` 반경 세 선택지 1:1 (`cmbBoreholeRadius`).
pub const BOREHOLE_RADIUS_CHOICES: [(i32, &str); 3] = [
    (2000, "반경 2 km (2,000 m · 정밀)"),
    (5000, "반경 5 km (5,000 m · 기본 권장)"),
    (10000, "반경 10 km (10,000 m · 광역 수계)"),
];

/// C++ `KaBoreholeDialog.cpp:114-140` 다섯 도면 카드의 제목과 설명 1:1.
pub const BOREHOLE_MAP_CARDS: [(&str, &str); 5] = [
    (
        "1. 시추공 분포 및 주상도 요약 마커 맵 (점 벡터)",
        "지점마다 [공번] 충적층 4.2m / 기반암 GL-6.5m (세립질 점토 우세) 라벨을 표시하고, 클릭 시 오른쪽 속성 카드에 전체 지층 구성(0~1.2m 매립토 → 1.2~4.5m 실트질점토 → 4.5m~ 풍화암)을 표시합니다.",
    ),
    (
        "2. 충적층(문화층 부존 가능 퇴적층) 두께 등층후선도 · Isopach Map (격자 래스터 + 등치선)",
        "인위적인 매립·성토층을 제외하고 자연 퇴적층(점토·실트·모래·자갈) 두께(m)를 공간 보간(IDW)하여 래스터와 등층후선(m)으로 표시합니다. 트렌치 굴착 예상 깊이를 바로 잡을 수 있습니다.",
    ),
    (
        "3. 성토·매립층(교란층) 두께 분포도 (격자 래스터)",
        "주상도 최상부의 매립토·성토 두께(m)만 추출해 보간합니다. 굴착 전 중장비로 표토·교란층을 몇 m까지 걷어내야 원지반(자연층)이 나오는지 현장 작업 계획을 세울 때 바로 씁니다.",
    ),
    (
        "4. 풍화암·기반암 출현 깊이(GL-m) 분포도 (기반암 지형도)",
        "풍화토·풍화암·연암이 시작되는 깊이(GL-m) 및 해발고도(EL.m)를 보간해 흙을 걷어냈을 때의 옛 기반암 바닥 지형(매몰 곡강·옛 골짜기 위치)을 복원합니다.",
    ),
    (
        "5. 평균 지하수위(GL-m) 분포도 (용수·배수 위험도 맵)",
        "시추 지하수위와 KIGAM 지하수 관정 수위를 결합해 지표면 아래 몇 m에서 지하수가 출현하는지(GL-m) 표시합니다. 양수기·배수로 준비가 필요한 구간(GL-2m 이내)을 사전에 파악합니다.",
    ),
];

/// C++ `KaBoreholeDialog.cpp:136` 4번 카드의 서브 체크 문구.
pub const BOREHOLE_ELEV_SUBCHECK: &str = "해발고도(EL.m) 도면 함께 생성";

/// C++ `KaBoreholeDialog.cpp:145-147` 하단 세 단추 1:1 (`btnPrimary` / `btnRebuildBoreholeMaps`).
pub const BOREHOLE_BTN_FETCH: &str = "시추자료 받기 및 선택 도면 생성";
pub const BOREHOLE_BTN_REBUILD: &str = "적재된 시추공으로 선택 도면 다시 생성";
pub const BOREHOLE_BTN_LAYOUT: &str = "도면 조판 열기";

/// C++ `KaBoreholeDialog.cpp:66-70` 조사 폴더가 없을 때의 안내.
pub const BOREHOLE_NEED_SURVEY: &str = "먼저 「새 조사」 또는 「열기」로 조사 폴더를 준비하세요.";

/// C++ `KaBoreholeDialog.cpp:171-176` 적재된 주상도가 없을 때의 안내.
pub const BOREHOLE_NEED_FETCH: &str = "먼저 「시추자료 받기 및 선택 도면 생성」을 눌러 시추공을 받아 오세요.";

/// 창의 체크 상태. C++ `GeoInfoBoreholeService::MapOptions` 기본값과 같다
/// (`GeoInfoBoreholeService.h:63-71`): 반경 5000, 4-B 해발고도만 기본 꺼짐.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoreholeDialogState {
    pub radius_meters: i32,
    pub marker_map: bool,
    pub alluvial_isopach_map: bool,
    pub fill_thickness_map: bool,
    pub bedrock_depth_map: bool,
    pub bedrock_elev_map: bool,
    pub groundwater_map: bool,
}

impl Default for BoreholeDialogState {
    fn default() -> Self {
        Self {
            radius_meters: 5000,
            marker_map: true,
            alluvial_isopach_map: true,
            fill_thickness_map: true,
            bedrock_depth_map: true,
            bedrock_elev_map: false,
            groundwater_map: true,
        }
    }
}

impl BoreholeDialogState {
    /// C++ `KaBoreholeDialog.cpp:101` `radCombo->currentIndex()` — 현재 반경의 콤보 자리.
    #[must_use]
    pub fn radius_choice_index(&self) -> usize {
        BOREHOLE_RADIUS_CHOICES
            .iter()
            .position(|(meters, _)| *meters == self.radius_meters)
            .unwrap_or(1)
    }
}

/// C++ `KaBoreholeDialog.cpp:161-166` `refreshCounts()` — 지금 적재된 시추공 수를 보여 준다.
#[must_use]
pub fn borehole_count_text(count: usize) -> String {
    if count > 0 {
        format!("현재 적재 시추공: {count}공")
    } else {
        "현재 적재 시추공 없음".to_owned()
    }
}

/// Button pressed in the window (cpp connects KaBoreholeDialog.cpp:169-192; rebuild is disabled until P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoreholeDialogAction {
    /// 「시추자료 받기 및 선택 도면 생성」 — 반경을 넘겨 수집을 시작한다.
    FetchAndGenerate { radius_meters: i32 },
    /// 「도면 조판 열기」 — 읽은 옵션을 저장하고 도면 스튜디오로 간다.
    OpenLayout,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C++ `KaBoreholeDialog.cpp:97-99` 반경 세 선택지와 기본값(5000) 1:1.
    #[test]
    fn test_borehole_radius_choices_and_default_match_cpp() {
        assert_eq!(BOREHOLE_RADIUS_CHOICES.len(), 3);
        assert_eq!(BOREHOLE_RADIUS_CHOICES[0].0, 2000);
        assert_eq!(BOREHOLE_RADIUS_CHOICES[1].0, 5000);
        assert_eq!(BOREHOLE_RADIUS_CHOICES[2].0, 10000);
        assert_eq!(BOREHOLE_RADIUS_CHOICES[0].1, "반경 2 km (2,000 m · 정밀)");
        assert_eq!(
            BOREHOLE_RADIUS_CHOICES[1].1,
            "반경 5 km (5,000 m · 기본 권장)"
        );
        assert_eq!(
            BOREHOLE_RADIUS_CHOICES[2].1,
            "반경 10 km (10,000 m · 광역 수계)"
        );

        let d = BoreholeDialogState::default();
        assert_eq!(d.radius_meters, 5000);
        assert_eq!(d.radius_choice_index(), 1);
        // GeoInfoBoreholeService.h:63-71 — 4-B 해발고도만 기본 꺼짐.
        assert!(d.marker_map);
        assert!(d.alluvial_isopach_map);
        assert!(d.fill_thickness_map);
        assert!(d.bedrock_depth_map);
        assert!(!d.bedrock_elev_map);
        assert!(d.groundwater_map);
    }

    /// C++ `KaBoreholeDialog.cpp:161-166` `refreshCounts()` 문구 1:1.
    #[test]
    fn test_borehole_count_text_matches_cpp() {
        assert_eq!(borehole_count_text(0), "현재 적재 시추공 없음");
        assert_eq!(borehole_count_text(38), "현재 적재 시추공: 38공");
    }

    /// 다섯 카드의 제목이 C++ 원문 순서와 같다(요약 없이 그대로).
    #[test]
    fn test_borehole_map_cards_count_and_headings() {
        assert_eq!(BOREHOLE_MAP_CARDS.len(), 5);
        assert!(BOREHOLE_MAP_CARDS[0].0.starts_with("1. 시추공 분포"));
        assert!(BOREHOLE_MAP_CARDS[1].0.contains("Isopach Map"));
        assert!(BOREHOLE_MAP_CARDS[2].0.starts_with("3. 성토·매립층"));
        assert!(BOREHOLE_MAP_CARDS[3].0.contains("출현 깊이(GL-m)"));
        assert!(BOREHOLE_MAP_CARDS[4].0.contains("평균 지하수위(GL-m)"));
    }

    /// The two enabled buttons point to different actions (`KaBoreholeDialog.cpp:169-192`).
    #[test]
    fn test_borehole_actions_are_distinct() {
        assert_ne!(
            BoreholeDialogAction::FetchAndGenerate {
                radius_meters: 5000
            },
            BoreholeDialogAction::OpenLayout
        );
    }
}
