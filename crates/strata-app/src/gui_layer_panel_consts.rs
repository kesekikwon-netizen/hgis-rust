//! 왼쪽 패널·파일함 치수와 안내말, 라벨 먹색 프리셋.

use eframe::egui::Color32;

/// UX_LAYOUT_SPEC.md 3절: 왼쪽 패널(폭 351), 상단 레이어 카드(높이 507), 하단 파일함 카드(높이 168)
pub const UX_LEFT_PANEL_WIDTH: f32 = 351.0;
pub const UX_LAYER_CARD_HEIGHT: f32 = 507.0;
pub const UX_FILE_DRAWER_HEIGHT: f32 = 168.0;
/// `KaLayerInformationView.cpp:18-21` labelColumnWidth — max(88, 「이름·면적」 너비 + 체크칸 + 18).
/// 이 글꼴(10.5)에서는 뒤쪽이 88 보다 작아 88 이다.
pub const UX_LABEL_COLUMN_WIDTH: f32 = 88.0;
/// 실제 C++ 범례(150% 화면 캡처를 잰 값)의 레이어 한 줄 높이.
pub const UX_LEGEND_ROW_HEIGHT: f32 = 22.0;
/// `KaLayerSectionDelegate.h:21` kBandHeight — 구역 첫 행 위에 붙는 띠.
pub const UX_LEGEND_BAND_HEIGHT: f32 = 16.0;
/// 기호 견본 테두리 상자(캡처 30×21 px @150%).
pub const UX_LEGEND_SWATCH_SIZE: [f32; 2] = [20.0, 14.0];
/// 교차 줄 배경(캡처).
pub const UX_LEGEND_ALT_ROW: Color32 = Color32::from_rgb(240, 238, 230);
/// 구역 띠 윗줄(캡처, palette Mid).
pub const UX_LEGEND_BAND_RULE: Color32 = Color32::from_rgb(135, 134, 127);
/// 구역 띠 제목 글자(캡처, palette PlaceholderText).
pub const UX_LEGEND_BAND_INK: Color32 = Color32::from_rgb(137, 137, 137);
/// 기호 견본 테두리(캡처).
pub const UX_LEGEND_SWATCH_FRAME: Color32 = Color32::from_rgb(20, 20, 19);
/// `KaFileBrowserPanel.cpp:92-136` — 폴더 단추 여섯 개와 목록 칸의 안내말
pub const FILE_BTN_PC_TIP: &str = "컴퓨터 드라이브 목록으로 이동합니다";
pub const FILE_BTN_DESK_TIP: &str = "바탕화면 폴더로 이동합니다";
pub const FILE_BTN_DOCS_TIP: &str = "문서 폴더로 이동합니다";
pub const FILE_BTN_DOWN_TIP: &str = "내려받기 폴더로 이동합니다";
pub const FILE_BTN_PICK_TIP: &str =
    "조사 자료 폴더를 직접 찾아 엽니다. 카메라·GPS 메모리카드(SD)도 여기서 엽니다";
pub const FILE_BTN_UP_TIP: &str = "상위 폴더로 한 단계 이동합니다";
pub const FILE_LIST_TIP: &str = "파일을 지도에 끌어 넣으면 레이어가 됩니다.";
/// KaLayerInformationPanel.cpp:26-33 `kLabelInks` — 6 label ink presets.
pub const KA_LABEL_INKS: [(&str, (u8, u8, u8)); 6] = [
    ("기본 (먹색)", (31, 35, 40)),
    ("흰색", (255, 255, 255)),
    ("진한 파랑", (30, 58, 138)),
    ("진한 빨강", (153, 27, 27)),
    ("갈색", (124, 45, 18)),
    ("진한 초록", (20, 83, 45)),
];
