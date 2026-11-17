//! A heritage intranet to develop against, so no test and no development run touches the
//! real site (`intranet.gis-heritage.go.kr`).
//!
//! The pages carry the element ids the ported scripts look for
//! (`crates/strata-net/src/heritage_auto/scripts.rs`), so a run against this fixture
//! exercises the same selectors as a run against the site. The routes are in `FIXTURES.md`.

#[cfg(test)]
mod tests;

mod download_page;
mod pages;
mod routes;
mod server;
mod state;

pub use state::{Fixture, FixtureState, Presentation};
