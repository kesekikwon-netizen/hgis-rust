//! 환경변수 가드 — 시험이 네이티브 파일 대화상자를 띄우지 않게 한다.
//!
//! ## 왜 필요한가
//!
//! `cargo test` 는 창을 띄우는 프로세스다. 리본 버튼 「열기」·「저장」·「다른 이름」 을
//! 누르면 `rfd::FileDialog` 가 **실제 창을 띄운다**. 2026-11-10, 리본 감사 시험이
//! 이걸 눌러 열기 창이 떠서 사용자 작업 화면을 12분간 덮었다.
//!
//! 검증용 경로를 환경변수로 주면 대화상자를 안 띄운다(`KA_HGIS_OPEN_PATH`,
//! `KA_HGIS_SAVE_PATH`). 이 가드는 그 환경변수를 **반드시** 세우고, 끝나면 지운다.
//!
//! 주의: 환경변수는 프로세스 전역이라 시험끼리 서로 볼 수 있다. 경로를 시험마다
//! 고유하게 만들고, 가드가 drop 될 때 확실히 지운다.

/// 임시 폴더 + 고유 이름을 만들고, 끝나면 지운다.
pub(crate) struct TempDirGuard {
    pub(crate) path: std::path::PathBuf,
}

impl TempDirGuard {
    /// `prefix` 로 시작하는 고유 임시 폴더를 만든다.
    pub(crate) fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{prefix}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("임시 폴더 생성");
        Self { path }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// 환경변수를 세우고, drop 되면 지운다.
///
/// 이게 있으면 시험이 실패해도 환경변수가 남지 않아 다음 시험을 오염시키지 않는다.
pub(crate) struct EnvGuard {
    name: &'static str,
}

impl EnvGuard {
    /// `name` 을 `value` 로 세운다.
    pub(crate) fn set(name: &'static str, value: &std::path::Path) -> Self {
        std::env::set_var(name, value);
        Self { name }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_guard_creates_and_removes() {
        let g = TempDirGuard::new("ka_guard_test");
        assert!(g.path.exists(), "폴더가 생겨야 한다");
        let p = g.path.clone();
        drop(g);
        assert!(!p.exists(), "drop 되면 지워져야 한다");
    }

    #[test]
    fn env_guard_restores_on_drop() {
        let dir = TempDirGuard::new("ka_env_test");
        {
            let _g = EnvGuard::set("KA_HGIS_OPEN_PATH", &dir.path);
            assert_eq!(
                std::env::var("KA_HGIS_OPEN_PATH").ok(),
                Some(dir.path.to_string_lossy().into_owned()),
                "가드가 세워져야"
            );
        }
        assert!(
            std::env::var("KA_HGIS_OPEN_PATH").is_err(),
            "drop 되면 지워져야"
        );
    }
}
