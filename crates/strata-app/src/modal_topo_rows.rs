//! 지형도 표 행·CRS 후보와 기본 용지 헬퍼.
//!
//! `modal_types.rs` 가 300줄을 넘어 나눴다. `dialogs_modal` 이 재수출한다.


/// C++ `KaTopographicImportDialog::recordKey` — 같은 자료를 알아보는 열쇠.
/// C++ 은 원본·레이어·서명·CAD 레이어·도형 갈래를 SHA-256 으로 묶는다(`:401-404`).
#[must_use]
pub fn topo_record_key(record: &strata_net::topographic_catalog::Record) -> String {
    format!(
        "{}|{}|{}",
        record.source, record.layer_name, record.cad_layer
    )
}

/// C++ `KaTopographicImportDialog.cpp:476` 콤보 첫 줄.
pub const TOPO_CRS_UNKNOWN: &str = "좌표계 미확인 — 선택 필요";

/// C++ `KaTopographicImportDialog.cpp:479` 후보 목록(이 순서 그대로).
pub const TOPO_CRS_CANDIDATES: [u32; 12] = [
    5185, 5186, 5187, 5188, 5173, 5174, 5175, 5176, 5177, 5178, 5179, 4326,
];

/// C++ `KaTopographicImportDialog.cpp:243-259` `crsLabel` 이 돌려주는 이름.
#[must_use]
pub const fn topo_crs_label_ko(epsg: u32) -> &'static str {
    match epsg {
        5185 => "서부원점(GRS80) · EPSG:5185",
        5186 => "중부원점(GRS80) · EPSG:5186",
        5187 => "동부원점(GRS80) · EPSG:5187",
        5188 => "동해원점(GRS80) · EPSG:5188",
        5173 => "서부원점(보정 베셀) · EPSG:5173",
        5174 => "중부원점(보정 베셀) · EPSG:5174",
        5175 => "제주원점(보정 베셀) · EPSG:5175",
        5176 => "동부원점(보정 베셀) · EPSG:5176",
        5177 => "동해원점(보정 베셀) · EPSG:5177",
        5178 => "UTM-K(베셀) · EPSG:5178",
        5179 => "UTM-K(GRS80) · EPSG:5179",
        4326 => "경위도(WGS84) · EPSG:4326",
        _ => TOPO_CRS_UNKNOWN,
    }
}

/// 표 한 줄에 그릴 값 — 창을 그리는 동안 `self` 를 빌리지 않게 미리 뽑아 둔다.
#[derive(Debug, Clone, PartialEq)]
pub struct TopoTableRow {
    /// `KaTopographicImportDialog.cpp:466` — `[검토 필요]` 를 뺀 이름.
    pub label: String,
    pub file_name: String,
    /// 있으면 `[검토 필요]` 로 표시하고 자동 체크하지 않는다(`:465-470`).
    pub review_reason: Option<String>,
    /// `:488-491` 범위 문구.
    pub extent_label: String,
    /// `:476-486` 콤보 항목. 0 번은 늘 「좌표계 미확인」, 1 번은 `.prj` 파일 정보(있을 때).
    pub choices: Vec<String>,
}

impl TopoTableRow {
    #[must_use]
    pub fn choice_label(&self, index: usize) -> String {
        self.choices
            .get(index)
            .cloned()
            .unwrap_or_else(|| TOPO_CRS_UNKNOWN.to_owned())
    }

    /// `.prj` 에서 읽은 좌표계를 콤보 자리로 바꾼다. 없으면 0(미확인).
    #[must_use]
    pub fn default_choice_index(&self, record: &strata_net::topographic_catalog::Record) -> usize {
        if record.crs_epsg().is_some() && self.choices.len() > 1 {
            1
        } else {
            0
        }
    }

    /// 고른 좌표계가 작업 좌표계 계열(GRS80 원점대)인가.
    /// C++ `previewCurrent(:498-522)` 는 실제로 좌표변환해 겹쳐 보지만, 이 창에는 지도 캔버스가
    /// 없어 같은 판단만 보인다. 베셀·경위도는 원점대가 달라 그대로 겹치지 않는다.
    #[must_use]
    pub fn choice_is_work_crs_family(&self, index: usize) -> bool {
        self.choices
            .get(index)
            .and_then(|choice| choice.rsplit_once("EPSG:"))
            .and_then(|(_, code)| code.parse::<u32>().ok())
            .is_some_and(|epsg| matches!(epsg, 5186 | 5187 | 5188 | 5185 | 5179))
    }
}

/// `.prj` 에서 읽은 EPSG 를 콤보 자리로. 못 읽었으면 0(미확인), 읽었으면 1(파일 정보).
#[must_use]
pub fn topo_default_crs_idx(epsg: Option<u32>) -> usize {
    if epsg.is_some() {
        1
    } else {
        0
    }
}
