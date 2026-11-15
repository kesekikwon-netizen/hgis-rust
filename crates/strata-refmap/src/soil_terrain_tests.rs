//! soilTerrainLegend_officialCodesAndStyle (`target/cpp-ref/tests/test_workflow.cpp:1014`): the eleven
//! official codes with their names and colours, and what an unknown code becomes.

use super::*;

#[test]
fn the_eleven_official_codes_have_their_names_colours_and_labels() {
    let expected: [(&str, &str, [u8; 3], &str); 11] = [
        ("01", "산악지", [1, 178, 0], "산악지"),
        ("02", "구릉지", [177, 210, 93], "구릉지"),
        ("03", "산록경사지", [139, 68, 108], "산록경사"),
        ("04", "곡간지/선상지", [254, 244, 182], "곡간·선상"),
        ("05", "해성평탄지", [132, 186, 230], "해성평탄"),
        ("06", "하성평탄지", [192, 239, 245], "하성평탄"),
        ("07", "고원지", [78, 78, 78], "고원지"),
        ("08", "홍적대지", [254, 160, 0], "홍적대지"),
        ("09", "용암류대지", [192, 151, 155], "용암대지"),
        ("10", "용암류평탄", [194, 179, 182], "용암평탄"),
        ("99", "기타", [255, 255, 255], "기타"),
    ];
    assert_eq!(TERRAIN.iter().map(|t| (t.code, t.name, t.rgb, t.label)).collect::<Vec<_>>(), expected);
    for (code, name, rgb, label) in expected {
        assert_eq!((terrain_name(code), terrain_colour(code), label_text(code)), (name, rgb, label), "{code}");
    }
}

/// The C++ test asks for code 77: 미분류 in 200/200/200, and no label.
#[test]
fn a_code_that_is_none_of_them_is_unclassed_grey_without_a_label() {
    for code in ["77", "", "4", "004", "1"] {
        assert_eq!((terrain_name(code), terrain_colour(code), label_text(code)), ("미분류", [200, 200, 200], ""), "{code:?}");
    }
}
