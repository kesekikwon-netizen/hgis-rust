//! `--snap` — 앱이 스스로 화면을 캡처해 PNG 로 저장한다.
//!
//! 좌표 클릭으로 검증하면 포커스·DPI 문제로 실패한다. 대신 앱 안에서
//! `ViewportCommand::Screenshot` 을 보내고, 돌아온 이미지를 파일로 쓴다.
//! C++ 대조 검증용이며 사용자 화면 동작에는 영향이 없다.

use std::path::PathBuf;

/// 스냅샷 요청 상태.
#[derive(Debug, Default, Clone)]
pub struct SnapRequest {
    /// 남은 단계 목록: (할 일, 저장 경로). 앞에서부터 하나씩 처리한다.
    pub steps: Vec<SnapStep>,
    /// 파일명 접두사.
    pub prefix: PathBuf,
    /// 지금 캡처를 기다리는 중인지.
    pub waiting: bool,
    /// 대기 프레임 수(레이아웃이 자리 잡을 때까지 몇 프레임 기다린다).
    pub settle: u8,
}

/// 한 단계: 액션 하나를 하고 그 결과를 저장.
///
/// `action` 은 다음 중 하나다.
/// - `@이름` — 내장 동작(탭 전환·모달 닫기 등)
/// - `@이름:인자` — 인자가 있는 내장 동작
/// - 그 밖의 문자열 — 리본 라벨로 눌러 본다
#[derive(Debug, Clone)]
pub struct SnapStep {
    /// 누를 리본 라벨(없으면 그냥 캡처).
    pub action: Option<String>,
    /// 저장 파일 이름(확장자 제외).
    pub name: String,
}

impl SnapRequest {
    /// `--snap <out_dir> --snap-steps "새 조사:new_survey,지도:map"` 형식으로 만든다.
    #[must_use]
    pub fn parse(out_dir: &str, steps: &str) -> Self {
        let mut list = Vec::new();
        for raw in steps.split(',') {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            // `액션|파일이름` 형식을 먼저 본다.
            //
            // 액션에 인자가 붙으면 `@draw:600x300` 처럼 콜론이 들어간다.
            // 예전처럼 첫 콜론에서 자르면 인자가 통째로 파일 이름이 되어
            // 명령이 먹지 않는다(2026-11-10 실제로 그랬다).
            if let Some((act, name)) = raw.rsplit_once('|') {
                let (act, name) = (act.trim(), name.trim());
                if !act.is_empty() {
                    list.push(SnapStep {
                        action: Some(act.to_owned()),
                        name: if name.is_empty() { act.to_owned() } else { name.to_owned() },
                    });
                    continue;
                }
            }
            match raw.split_once(':') {
                Some((act, name)) if !act.is_empty() && !name.contains('x') => {
                    list.push(SnapStep {
                        action: Some(act.to_owned()),
                        name: name.to_owned(),
                    });
                }
                _ => list.push(SnapStep {
                    action: None,
                    name: raw.to_owned(),
                }),
            }
        }
        Self {
            steps: list,
            prefix: PathBuf::from(out_dir),
            waiting: false,
            settle: 12,
        }
    }

    /// 다음에 할 단계가 있는지.
    #[must_use]
    pub fn has_work(&self) -> bool {
        !self.steps.is_empty() || self.waiting
    }
}
