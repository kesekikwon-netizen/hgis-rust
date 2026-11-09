//! `verify_harness` 시험 — 게이트 17개 전부 통과, 스모크 3단계, 충돌·비상 저장.
//!
//! 원래 `verify_harness.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 가 창구를 가리키지 않으므로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::verify_harness::{run_cpp_smoke_suite_report, run_v1_to_v10_verification_harness};

    #[test]
    fn v1_to_v17_verification_harness_passes_all_seventeen_gates() {
        let gates = run_v1_to_v10_verification_harness();
        assert_eq!(gates.len(), 17);
        for g in &gates {
            assert!(g.passed, "Gate {} failed: {}", g.gate_id, g.detail);
        }
    }

    #[test]
    fn test_cpp_smoke_suite_and_crash_handler_emergency_autosave_and_shortcut_conflict() {
        let (smoke_ok, report) = run_cpp_smoke_suite_report();
        assert!(smoke_ok, "smoke suite failed: {report}");
        assert!(report.contains("ALL 3 PASSED, 0 FAILED"));

        let mut shortcuts = crate::app_state::ShortcutManager::new();
        assert!(shortcuts.register_shortcut("file.save", "Ctrl+S"));
        assert!(!shortcuts.register_shortcut("layer.save_dup", "ctrl+s"));
        assert!(shortcuts.register_shortcut("view.pan", "Space"));

        let mut app = StrataAppState::new_startup();
        app.left_layers
            .ensure_domain_layer(DomainLayerKey::SurveyArea);
        let crash_path =
            std::env::temp_dir().join(format!("crash_recovery_{}.gpkg", std::process::id()));
        assert!(app.emergency_auto_save(&crash_path));
        assert!(crash_path.exists());
        let recovered = GpkgRepository::open_read_only(&crash_path)
            .expect("crash recovery GPKG must open cleanly");
        assert!(recovered
            .layers
            .contains_key(DomainLayerKey::SurveyArea.id()));
        let _ = std::fs::remove_file(&crash_path);
    }
