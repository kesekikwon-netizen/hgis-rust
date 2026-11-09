//! 怨좉퀬??議곗궗 7?④퀎 ?뚰겕?뚮줈 ?먯젙.
//!
//! `project_actions` ?먯꽌 遺꾨━?덈떎.

/// 고고학 조사 7단계 워크플로 단계 (C++ `WorkflowStep`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStep {
    Survey,
    Background,
    SurveyArea,
    Features,
    ControlPoints,
    Review,
    Submission,
}

/// 7단계 워크플로 판정 입력 상태 (C++ `WorkflowGuide::evaluate` 입력)
#[derive(Debug, Clone, Default)]
pub struct WorkflowProjectInput {
    pub survey_name: String,
    pub layer_count: usize,
    pub survey_area_count: usize,
    pub feature_poly_count: usize,
    pub feature_line_count: usize,
    pub control_points_count: usize,
    pub has_datum: bool,
    pub has_ellipsoid: bool,
    pub has_projection: bool,
    pub has_visible_reference_layer: bool,
    pub checklist_error_count: usize,
    pub package_created: bool,
}

/// 7단계 워크플로 개별 단계 판정 결과 (C++ `WorkflowStepState`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub title: &'static str,
    pub completed: bool,
}

/// 프로젝트의 레이어·피처·기준점·검수 현황을 감사하여 7단계 완료 여부를 판정
/// (C++ `WorkflowGuide::evaluate` 1:1 대응)
#[must_use]
pub fn evaluate_workflow_guide(input: &WorkflowProjectInput) -> [WorkflowStepState; 7] {
    let survey_done = !input.survey_name.trim().is_empty()
        || input.layer_count > 0
        || input.survey_area_count > 0;
    let survey_area_done = input.survey_area_count > 0;
    let features_done = input.feature_poly_count > 0 || input.feature_line_count > 0;
    let cp_done = input.control_points_count >= 2
        && input.has_datum
        && input.has_ellipsoid
        && input.has_projection;
    let review_done = input.checklist_error_count == 0;

    [
        WorkflowStepState {
            step: WorkflowStep::Survey,
            title: "1. 새 조사",
            completed: survey_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Background,
            title: "2. 배경·지적도",
            completed: input.has_visible_reference_layer,
        },
        WorkflowStepState {
            step: WorkflowStep::SurveyArea,
            title: "3. 조사구역",
            completed: survey_area_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Features,
            title: "4. 유구·단면선",
            completed: features_done,
        },
        WorkflowStepState {
            step: WorkflowStep::ControlPoints,
            title: "5. GPS 기준점",
            completed: cp_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Review,
            title: "6. 도면 검수",
            completed: review_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Submission,
            title: "7. 제출 패키지",
            completed: input.package_created,
        },
    ]
}
