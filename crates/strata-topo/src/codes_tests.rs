//! The layer names of `target/cpp-ref/tests/test_topographic_catalog.cpp:209-236` with the kind each is.

use super::*;

#[test]
fn a_layer_name_is_classed_by_its_standard_code() {
    let examples = [
        ("F0010000", Category::Contour),
        ("F0020000_37701001", Category::ElevationPoint),
        ("수치지형도_A0010000_37701", Category::Road),
        ("A0020000", Category::Road),
        ("B0010000", Category::Building),
        ("E0010001", Category::Water),
        ("E0020000", Category::Water),
        ("E0032111", Category::Water),
        ("G0010000", Category::Boundary),
        ("H0040000", Category::PlaceName),
        // Too short; followed by digits; preceded or followed by a letter.
        ("F001", Category::Unknown),
        ("F001000037701", Category::Unknown),
        ("XF0010000", Category::Unknown),
        ("F0010000X", Category::Unknown),
        // Not in the book; two kinds in one name.
        ("E0010000", Category::Unknown),
        ("F0010000_B0010000", Category::Unknown),
        ("", Category::Unknown),
        ("0", Category::Unknown),
    ];
    for (name, wanted) in examples {
        assert_eq!(infer_code(name).map_or(Category::Unknown, |info| info.category), wanted, "{name:?}");
    }
    // Two codes of one kind are that kind, named after the last (`:97-100`).
    let two = infer_code("A0013110 A0013111").expect("roads");
    assert_eq!((two.category, two.name.as_str()), (Category::Road, "(기존도로)고속국도"));
    // The frame of a sheet is in the book as a facility: its extent does not place the sheet.
    let frame = infer_code("H0017334").expect("frame");
    assert_eq!((frame.category, frame.name.as_str(), frame.category.is_primary()), (Category::Facility, "도곽", false));
}

#[test]
fn the_word_of_a_kind_is_that_kind() {
    for (word, kind) in [("등고선", Category::Contour), ("도로", Category::Road), ("건물", Category::Building), ("수계", Category::Water), ("식생", Category::Vegetation), ("경계", Category::Boundary), ("시설물", Category::Facility), ("표고점", Category::ElevationPoint), ("지명", Category::PlaceName)] {
        assert_eq!(kind.name(), word);
        assert_eq!(infer_code(&format!("  {word} ")), Some(CodeInfo { category: kind, name: word.to_owned() }));
    }
    assert_eq!((Category::Unknown.name(), infer_code("미분류")), ("미분류", None));
    let primary: Vec<&str> = [Category::Contour, Category::Road, Category::Building, Category::Water, Category::Vegetation, Category::Boundary, Category::Facility, Category::ElevationPoint, Category::PlaceName, Category::Unknown].into_iter().filter(|c| c.is_primary()).map(Category::name).collect();
    assert_eq!(primary, ["등고선", "도로", "건물", "수계", "경계", "표고점", "지명"]);
}
