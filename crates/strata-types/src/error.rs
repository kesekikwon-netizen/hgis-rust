use thiserror::Error;

pub type StrataResult<T> = Result<T, StrataError>;

#[derive(Debug, Error)]
pub enum StrataError {
    #[error("입출력 오류: {0}")]
    Io(#[from] std::io::Error),
    #[error("데이터베이스 오류: {0}")]
    Storage(String),
    #[error("좌표계 변환 오류: {0}")]
    Crs(String),
    #[error("네트워크·외부 자료 오류: {0}")]
    Network(String),
    #[error("보안·계정 설정 오류: {0}")]
    Secret(String),
    #[error("검수·내보내기 오류: {0}")]
    Export(String),
    #[error("잘못된 입력: {0}")]
    InvalidInput(String),
}
