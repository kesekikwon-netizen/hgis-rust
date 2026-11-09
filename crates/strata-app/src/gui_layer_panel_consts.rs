//! 왼쪽 패널·파일함 치수와 안내말, 라벨 먹색 프리셋.

/// UX_LAYOUT_SPEC.md 3절: 왼쪽 패널(폭 351), 상단 레이어 카드(높이 507), 하단 파일함 카드(높이 168)
pub const UX_LEFT_PANEL_WIDTH: f32 = 351.0;
pub const UX_LAYER_CARD_HEIGHT: f32 = 507.0;
pub const UX_FILE_DRAWER_HEIGHT: f32 = 168.0;
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
