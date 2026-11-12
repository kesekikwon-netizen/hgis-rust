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
            // 마지막 ':' 로 나눈다 — 「@open=C:...」 처럼 경로에 ':' 가 있어도 된다.
            match raw.rsplit_once(':') {
                Some((act, name)) if !act.is_empty() => {
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
