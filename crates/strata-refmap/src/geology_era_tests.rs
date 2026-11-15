//! Era classes against the C++ test values (`target/cpp-ref/tests/test_workflow.cpp`, geology era and legend
//! cases, as listed in the director's notes on GeologyMapService), and the two colour rules of Qt against
//! what Qt itself answers.
//!
//! Origin of the Qt tables below: observed on this project's reference toolkit, Qt 6.11 (PyQt6 6.11.0 of
//! the OSGeo4W installation the C++ app is built with), by calling `QColor(r, g, b).lighter(factor)` and
//! `QColor(r, g, b).lightness()` on 2026-11-15. They are measurements, not source text.

use super::*;

#[test]
fn the_class_is_the_first_keyword_found_in_the_era_text() {
    assert_eq!(era_class("신생대 제 4기"), "제4기", "the Jeju layer writes a space");
    assert_eq!(era_class("현생누대 신생대 제4기"), "제4기", "the period, not the era that holds it");
    assert_eq!(era_class("현생누대 중생대 백악기"), "백악기");
    assert_eq!(era_class("선캄브리아시대"), "선캄브리아시대");
    assert_eq!(era_class("쥬라기"), "쥐라기", "the older spelling");
    assert_eq!(era_class("신제3기"), "네오기(신제3기)");
    assert_eq!(era_class("고제3기"), "팔레오기(고제3기)");
    assert_eq!(era_class("현생누대 중생대"), "중생대", "no period named: the era");
    assert_eq!(era_class(""), "시대미상");
    assert_eq!(era_class("알 수 없음"), "시대미상");
}

#[test]
fn the_rank_is_the_line_of_the_table() {
    assert_eq!(era_rank("제4기"), 0);
    assert_eq!((era_rank("네오기"), era_rank("신제3기")), (1, 2), "one class, two ranks");
    assert_eq!(era_rank("현생누대 중생대 백악기"), 5);
    assert_eq!(era_rank("선캄브리아시대"), 20);
    assert_eq!(era_rank("시대미상"), 21);
    assert_eq!(ERAS.len(), 21);
    // Younger before older: a period is found before the era that holds it.
    assert!(era_rank("캄브리아기") < era_rank("고생대"));
    // 선캄브리아 holds the text 캄브리아: the era text 선캄브리아시대 must not be read as 캄브리아기.
    assert_eq!(era_class("선캄브리아"), "선캄브리아시대");
}

#[test]
fn the_colour_of_a_class_is_the_table_colour() {
    assert_eq!(era_color("제4기"), [249, 249, 127]);
    assert_eq!(era_color("네오기(신제3기)"), [255, 230, 25]);
    assert_eq!(era_color("선캄브리아시대"), [247, 67, 112]);
    assert_eq!(era_color("시대미상"), [200, 200, 200]);
    assert_eq!(era_color("기타"), [200, 200, 200]);
}

/// Each line: a colour and what Qt's `lighter` makes of it for the factors 76, 88, 100, 112 and 124.
const LIGHTER: [([u8; 3], [[u8; 3]; 5]); 19] = [
    ([249, 249, 127], [[190, 190, 97], [220, 220, 112], [249, 249, 127], [255, 255, 154], [255, 255, 184]]),
    ([255, 230, 25], [[195, 176, 19], [226, 204, 22], [255, 230, 25], [255, 233, 56], [255, 237, 86]]),
    ([253, 154, 82], [[193, 118, 63], [224, 136, 73], [253, 154, 82], [255, 172, 111], [255, 189, 141]]),
    ([127, 198, 78], [[97, 151, 60], [112, 175, 69], [127, 198, 78], [142, 222, 87], [157, 246, 97]]),
    ([52, 178, 201], [[40, 136, 153], [46, 158, 178], [52, 178, 201], [58, 199, 225], [64, 221, 249]]),
    ([129, 43, 146], [[98, 33, 111], [114, 38, 129], [129, 43, 146], [144, 48, 164], [160, 53, 181]]),
    ([240, 64, 40], [[183, 49, 31], [212, 57, 35], [240, 64, 40], [255, 80, 56], [255, 105, 85]]),
    ([103, 165, 153], [[79, 126, 117], [91, 146, 135], [103, 165, 153], [115, 185, 171], [128, 205, 190]]),
    ([203, 140, 55], [[155, 107, 42], [180, 124, 49], [203, 140, 55], [227, 157, 62], [252, 174, 68]]),
    ([179, 225, 182], [[137, 172, 139], [158, 199, 161], [179, 225, 182], [200, 252, 204], [227, 255, 229]]),
    ([0, 146, 112], [[0, 111, 85], [0, 129, 99], [0, 146, 112], [0, 164, 125], [0, 181, 139]]),
    ([127, 160, 86], [[97, 122, 66], [112, 142, 76], [127, 160, 86], [142, 179, 96], [157, 198, 107]]),
    ([242, 249, 29], [[185, 190, 22], [214, 220, 26], [242, 249, 29], [249, 255, 54], [250, 255, 83]]),
    ([103, 197, 202], [[79, 150, 154], [91, 174, 179], [103, 197, 202], [115, 221, 226], [128, 244, 250]]),
    ([153, 192, 141], [[117, 147, 108], [135, 170, 125], [153, 192, 141], [171, 215, 158], [190, 238, 175]]),
    ([247, 53, 99], [[189, 40, 76], [219, 47, 88], [247, 53, 99], [255, 76, 119], [255, 106, 141]]),
    ([240, 4, 127], [[183, 3, 97], [212, 4, 112], [240, 4, 127], [255, 18, 142], [255, 47, 155]]),
    ([247, 67, 112], [[189, 51, 85], [219, 59, 99], [247, 67, 112], [255, 91, 132], [255, 120, 154]]),
    ([200, 200, 200], [[153, 153, 153], [177, 177, 177], [200, 200, 200], [224, 224, 224], [248, 248, 248]]),
];

/// Every era colour, and the grey of an unknown class, for the five shades: Qt's own answers. Qt keeps
/// colours in 16 bits a channel and this works in fractions, so a channel may differ by one step of 255;
/// the test names how many do.
#[test]
fn lighter_gives_what_qt_gives() {
    let mut off_by_one = 0;
    for (rgb, steps) in LIGHTER {
        for (factor, want) in [76, 88, 100, 112, 124].into_iter().zip(steps) {
            let got = lighter(rgb, factor);
            for (g, w) in got.iter().zip(want) {
                assert!(g.abs_diff(w) <= 1, "{rgb:?} lighter({factor}) gives {got:?}, Qt {want:?}");
                off_by_one += u32::from(*g != w);
            }
        }
    }
    assert_eq!(off_by_one, 0, "channels that differ from Qt by one step");
    assert_eq!(lighter([10, 20, 30], 0), [10, 20, 30], "a factor of nothing changes nothing");
}

#[test]
fn a_unit_without_an_official_colour_gets_its_era_colour_shaded_by_its_symbol() {
    // "Qa": 81 + 97 = 178, 178 % 5 = 3: one step lighter (112) of 제4기.
    assert_eq!(unit_fallback_color("현생누대 신생대 제4기", "Qa"), [255, 255, 154]);
    // "Kswt": 75 + 115 + 119 + 116 = 425, 425 % 5 = 0: two steps darker (76) of 백악기.
    assert_eq!(unit_fallback_color("현생누대 중생대 백악기", "Kswt"), [97, 151, 60]);
    // "PCEbngn" = 80+67+69+98+110+103+110 = 637, 637 % 5 = 2: the era colour itself.
    assert_eq!(unit_fallback_color("선캄브리아시대", "PCEbngn"), [247, 67, 112]);
    // No era known: the grey, shaded the same way. "x" = 120, 120 % 5 = 0.
    assert_eq!(unit_fallback_color("", "x"), [153, 153, 153]);
    // A symbol in Korean letters counts its UTF-8 bytes: "가" = 0xEA 0xB0 0x80 = 234 + 176 + 128 = 538, % 5 = 3.
    assert_eq!(unit_fallback_color("제4기", "가"), [255, 255, 154]);
}

/// Qt's `lightness()` for colours around the limit of 80 that the colour picking uses, and odd sums.
#[test]
fn lightness_gives_what_qt_gives() {
    for (rgb, want) in [([0, 0, 0], 0), ([255, 255, 255], 255), ([249, 249, 127], 188), ([80, 80, 80], 80), ([79, 79, 79], 79), ([159, 0, 0], 80), ([160, 0, 0], 80), ([161, 0, 0], 81), ([10, 200, 30], 105), ([250, 244, 180], 215), ([1, 2, 160], 81), ([255, 0, 1], 128), ([33, 77, 120], 77)] {
        assert_eq!(lightness(rgb), want, "{rgb:?}");
    }
}
