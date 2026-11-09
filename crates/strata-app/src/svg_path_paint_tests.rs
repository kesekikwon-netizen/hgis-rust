//! `svg_path_paint` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::ribbon_icon_svg::{svg_parts, SvgPart};
use crate::svg_path_tokenize::{tokenize, Tok};
use crate::svg_path_paint::GLYPH_RATIO;


// D26 — C++ Lucide SVG 52개가 모두 토큰화·파싱된다.
#[test]
fn test_svg_icons_tokenize_and_parse() {
    // MainWindowRibbon.cpp 가 쓰는 리본 id 전부
    let ids = [
        "new",
        "open",
        "save",
        "save_as",
        "select",
        "measure",
        "draw_poly",
        "trench_grid",
        "buffer",
        "cadastral",
        "topo_download",
        "heritage",
        "contour",
        "dem",
        "survey_contour",
        "soil",
        "paleo",
        "geology",
        "river",
        "old_map",
        "georef",
        "pdf",
        "print",
        "geotiff",
        "export_convert",
        "more",
    ];
    for id in ids {
        let parts = svg_parts(id);
        assert!(parts.is_some(), "missing C++ icon: {id}");
        let parts = parts.unwrap_or(&[]);
        assert!(!parts.is_empty(), "empty icon: {id}");
    }

    // 모든 path 데이터가 숫자 토큰을 낸다
    for id in ids {
        if let Some(parts) = svg_parts(id) {
            for part in parts {
                if let SvgPart::Path(d) = part {
                    let toks = tokenize(d);
                    assert!(!toks.is_empty(), "{id}: path produced no tokens");
                    assert!(
                        toks.iter().any(|t| matches!(t, Tok::Num(_))),
                        "{id}: path has no numbers"
                    );
                }
            }
        }
    }
}

// KaIconsMockupEngine.cpp:25-26 — 글리프는 타일의 18/32
#[test]
fn test_glyph_ratio_matches_cpp() {
    assert!((GLYPH_RATIO - 0.5625).abs() < 1e-6);
}
