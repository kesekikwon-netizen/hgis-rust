//! The water colour of each class of river (`RiverMapService.cpp:51-64`, `:205-219`).

use super::*;

#[test]
fn a_river_is_coloured_by_the_class_its_level_text_names() {
    for (text, fill, line) in [
        ("국가하천", [0, 126, 212], [0, 84, 141]),
        ("지방1급하천", [16, 146, 222], [11, 97, 148]),
        ("지방2급하천", [40, 166, 232], [27, 111, 155]),
        // The keyword anywhere in the text counts, the first class of the list first.
        ("(국가)", [0, 126, 212], [0, 84, 141]),
        ("국가 · 지방2급", [0, 126, 212], [0, 84, 141]),
        // Any other text, and none: the light blue of 「기타 수계」.
        ("소하천", [120, 196, 236], [80, 131, 157]),
        ("지방하천", [120, 196, 236], [80, 131, 157]),
        ("", [120, 196, 236], [80, 131, 157]),
    ] {
        let level = level_of(text);
        assert_eq!((level.fill, level.line), (fill, line), "{text:?}");
    }
    assert_eq!(LEVELS.map(|l| l.label), ["국가하천", "지방1급하천", "지방2급하천"]);
    assert_eq!(OTHER.label, "기타 수계");
}
