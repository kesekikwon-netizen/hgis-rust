//! `render_reference_status` 의 7종 참조 자료 행 정의.
//!
//! `modal_reference.rs` 가 300줄을 넘어 옮겼다. 값은 원문 그대로이고
//! 준비 여부만 인자로 받는다.
use crate::dialogs_modal::ActiveModalDialog;

/// 7종 참조 자료 한 행: (제목, 종류, 범위, 받은 시각, 크기, 상태, 준비됨, 받기 버튼, 설정 버튼)
pub type ReferenceStatusRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
    crate::dialogs_modal::ActiveModalDialog,
    crate::dialogs_modal::ActiveModalDialog,
);

pub(crate) fn reference_status_rows(
    cad_ready: bool,
    dem_ready: bool,
    her_ready: bool,
) -> Vec<ReferenceStatusRow> {
        let rows = [
            (
                "지적도 · 조사 주변 5km",
                "지적도",
                "약 10.0 × 10.0 km",
                if cad_ready { "2026-11-08 02:15" } else { "받지 않음" },
                if cad_ready { "4.8 MB" } else { "—" },
                if cad_ready {
                    "현장 사용 가능 (준비됨)"
                } else {
                    "파일 없음 (없음)"
                },
                cad_ready,
                ActiveModalDialog::CadastralDownload,
                ActiveModalDialog::AccountSettings,
            ),
            (
                "국토지리정보원 수치지형도 1:5,000",
                "수치지형도",
                "약 6.5 × 5.2 km",
                "2026-11-08 02:18",
                "12.4 MB",
                "현장 사용 가능 (준비됨)",
                true,
                ActiveModalDialog::NgiiTopoDownload,
                ActiveModalDialog::AccountSettings,
            ),
            (
                "지표모델(DSM) 30m · 국토지리원 DEM",
                "DEM",
                "약 10.0 × 10.0 km",
                if dem_ready { "2026-11-08 02:20" } else { "받지 않음" },
                if dem_ready { "6.2 MB" } else { "—" },
                if dem_ready {
                    "현장 사용 가능 (준비됨)"
                } else {
                    "파일 없음 (없음)"
                },
                dem_ready,
                ActiveModalDialog::DemDownload,
                ActiveModalDialog::DemClass,
            ),
            (
                "국가유산청 지정·비지정 주변유적 (경주시)",
                "주변유적",
                "약 12.0 × 12.0 km",
                if her_ready { "2026-11-08 02:22" } else { "받지 않음" },
                if her_ready { "9.1 MB" } else { "—" },
                if her_ready {
                    "현장 사용 가능 (준비됨)"
                } else {
                    "파일 없음 (없음)"
                },
                her_ready,
                ActiveModalDialog::HeritageRegionSelect,
                ActiveModalDialog::AccountSettings,
            ),
            (
                "한국지질자원연구원 1:50,000 지질도 (KIGAM)",
                "지질도",
                "전국 이상",
                "받지 않음",
                "—",
                "인터넷 필요 · 오프라인 저장 가능 (설정 필요)",
                false,
                ActiveModalDialog::PaleoSourceDownload,
                ActiveModalDialog::AccountSettings,
            ),
            (
                "농촌진흥청 흙토람 정밀토양도 (RDA)",
                "토양도",
                "전국 이상",
                "받지 않음",
                "—",
                "인터넷 필요 · 오프라인 저장 전 제공처 조건 확인 (설정 필요)",
                false,
                ActiveModalDialog::PaleoSourceDownload,
                ActiveModalDialog::AccountSettings,
            ),
            (
                "국가수자원관리종합정보 하천망 수계도 (WAMIS)",
                "수계도",
                "전국 이상",
                "파일 없음",
                "—",
                "파일 없음 · 오프라인 저장 불가 (없음)",
                false,
                ActiveModalDialog::PaleoSourceDownload,
                ActiveModalDialog::AccountSettings,
            ),
        ];
    rows.to_vec()
}
