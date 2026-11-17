//! Independent stage acknowledgments from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147`.
//! Only the page boundary is replaced; every transition uses a public returned script token.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "heritage_cpp_contract/driver.rs"]
mod driver;
#[path = "heritage_cpp_contract/filtered_results.rs"]
mod filtered_results;
#[path = "heritage_cpp_contract/retry.rs"]
mod retry;
#[path = "heritage_cpp_contract/stage_ack.rs"]
mod stage_ack;
