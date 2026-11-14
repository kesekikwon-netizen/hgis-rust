//! `verify_harness` tests — gates V1..V14, 3-step smoke check, shortcut conflict and emergency save.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Mutex;

use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::verify_harness::{run_cpp_smoke_suite_report, run_v1_to_v10_verification_harness};

// Gates V9 and V13 share process-id temp directories (`ka_hgis_v9_<pid>`, `ka_hgis_v13_<pid>`).
static HARNESS_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn v1_to_v14_verification_harness_passes_all_fourteen_gates() {
        let _guard = HARNESS_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let gates = run_v1_to_v10_verification_harness();
        assert_eq!(gates.len(), 14);
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

    // Order F1f-1 — --verify-all runs gates V1..V14 only (no borehole gates V15..V17).
    #[test]
    fn verify_harness_lists_gates_v1_to_v14_only() {
        let _guard = HARNESS_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let gates = run_v1_to_v10_verification_harness();
        let ids: Vec<&str> = gates.iter().map(|g| g.gate_id).collect();
        assert_eq!(
            ids,
            vec![
                "V1", "V2", "V3", "V4", "V5", "V6", "V7",
                "V8", "V9", "V10", "V11", "V12", "V13", "V14",
            ]
        );
        assert!(gates.iter().all(|g| g.passed));
    }
