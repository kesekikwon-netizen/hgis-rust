//! 「선 색·지번 표시」 of the received cadastral map (`MainWindowCadastral.cpp:56-80`,
//! `CadastralImport.cpp:244-286`), on a run made in the test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::tests::{bench, table};
use super::*;

/// The dialog opens with what the row has, 확인 gives the row the chosen colour and switch, and the lot
/// numbers are written in the colour of the outline (`:62-79`, `CadastralImport.cpp:246-263`).
#[test]
fn the_colour_and_the_lot_numbers_of_the_row_are_chosen() {
    let mut bench = bench(table);
    bench.app.press_cadastral();
    bench.frame(1.0);
    bench.frame(1.1);
    bench.app.open_cadastral_style();
    assert_eq!(bench.app.modals.active, Some(ActiveModalDialog::CadastralStyle));
    assert_eq!(bench.app.cadastral.style, Style { colour: [0, 0, 0], labels: true });

    bench.app.cadastral.style = Style { colour: [200, 30, 30], labels: false };
    bench.app.apply_cadastral_style();
    let spec = bench.app.ref_vectors.spec_of(CADASTRAL_KEY).unwrap();
    assert_eq!(spec.look, Look::Polygons { stroke: [200, 30, 30, 255], width_mm: 0.2, fill: PolygonFill::None });
    assert_eq!(spec.label, None);

    bench.app.cadastral.style.labels = true;
    bench.app.apply_cadastral_style();
    let label = bench.app.ref_vectors.spec_of(CADASTRAL_KEY).unwrap().label.unwrap();
    assert_eq!((label.column, label.ink, label.size_pt, label.closest_scale), ("JIBUN", [200, 30, 30], 8.0, Some(10_000.0)));

    bench.app.cadastral.style = Style::default();
    bench.app.open_cadastral_style();
    assert_eq!(bench.app.cadastral.style, Style { colour: [200, 30, 30], labels: true }, "the dialog opens with what the row has now");
}

/// The look is the user's: a new run keeps it (`CadastralImport.cpp:277-286`), and the table kept for the
/// survey areas comes back with it when the survey is opened again.
#[test]
fn the_chosen_look_stays_with_the_table() {
    let blue = Style { colour: [0, 90, 200], labels: false };
    let mut bench = bench(table);
    bench.app.press_cadastral();
    bench.frame(1.0);
    bench.frame(1.1);
    bench.app.cadastral.style = blue;
    bench.app.apply_cadastral_style();

    bench.app.press_cadastral();
    bench.frame(2.0);
    bench.frame(2.1);
    assert_eq!(bench.app.ref_vectors.spec_of(CADASTRAL_KEY).map(Style::of), Some(blue), "a new run keeps the look");

    bench.app.remove_reference_vector_rows();
    assert!(bench.app.ref_vectors.spec_of(CADASTRAL_KEY).is_none());
    bench.app.press_cadastral();
    assert_eq!(bench.status(), "저장된 조사 주변 지적도를 사용합니다.");
    assert_eq!(bench.app.ref_vectors.spec_of(CADASTRAL_KEY).map(Style::of), Some(blue));
}

/// Without the parcels there is nothing to choose (`:60`): no dialog, and the reason on the status line.
#[test]
fn without_the_parcels_there_is_nothing_to_choose() {
    let mut bench = bench(table);
    bench.app.open_cadastral_style();
    assert_eq!(bench.status(), "지적도를 먼저 받아 주세요.");
    assert!(bench.app.modals.active.is_none());
}
