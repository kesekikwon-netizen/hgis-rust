//! The heritage intranet run (`target/cpp-ref/src/app/KaHeritageBrowser.cpp`), as a stage
//! machine that knows no browser.
//!
//! The C++ app drives a hidden `QWebEngineView` with one page script per step and moves
//! from stage to stage by the script's answer. This module is that stage logic: the same
//! scripts ([`scripts`]), the same order, the same words. The pages are behind
//! [`PageHost`]; time is a number of milliseconds the caller passes in.

pub mod scripts;
