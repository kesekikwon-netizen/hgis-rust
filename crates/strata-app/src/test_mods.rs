//! Test modules that belong to no single production file, kept out of `main.rs` for the 300-line rule.

#[path = "perf_tests.rs"]
mod perf_tests;
#[path = "sound_tests.rs"]
mod sound_tests;
#[path = "sound_tests_ui.rs"]
mod sound_tests_ui;
