//! C++ `kyi/main` 의 Lucide SVG 아이콘을 그대로 옮긴 데이터.
//!
//! 출처: `target/cpp-ref/src/app/KaIconsMockupMap.cpp` 의 id→SVG 매핑과
//! `target/cpp-ref/data/**/icons/**/*.svg` 원문. 좌표계는 Lucide 의 24×24
//! viewBox 를 그대로 쓴다. 그리기는 `paint_svg_icon` 이 한다.

/// SVG 한 조각 — Lucide 원문 요소 하나.
#[derive(Debug, Clone, Copy)]
pub enum SvgPart {
    /// `<path d="...">`
    Path(&'static str),
    /// `<circle cx|cy|r>`
    Circle(&'static str),
    /// `<line x1|y1|x2|y2>`
    Line(&'static str),
    /// `<rect x|y|width|height|rx>`
    Rect(&'static str),
    /// `<polyline points>`
    Polyline(&'static str),
}

/// 앱 아이콘 id → Lucide 원문 조각들.
pub fn svg_parts(id: &str) -> Option<&'static [SvgPart]> {
    Some(match id {
        "new" => &[
            SvgPart::Path("M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"),
            SvgPart::Path("M14 2v5a1 1 0 0 0 1 1h5"),
            SvgPart::Path("M9 15h6"),
            SvgPart::Path("M12 18v-6"),
        ],
        "open" => &[
            SvgPart::Path("M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"),
        ],
        "save" => &[
            SvgPart::Path("M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"),
            SvgPart::Path("M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"),
            SvgPart::Path("M7 3v4a1 1 0 0 0 1 1h7"),
        ],
        "save_unsaved" => &[
            SvgPart::Path("M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"),
            SvgPart::Path("M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"),
            SvgPart::Path("M7 3v4a1 1 0 0 0 1 1h7"),
        ],
        "save_as" => &[
            SvgPart::Path("M13.33 13H8a1 1 0 00-1 1v7"),
            SvgPart::Path("M14.363 17.634a2 2 0 00-.506.854l-.837 2.87a.5.5 0 00.62.62l2.87-.837a2 2 0 00.854-.506l4.013-4.009a1 1 0 10-3.004-3.004z"),
            SvgPart::Path("M7 3v4a1 1 0 001 1h7"),
            SvgPart::Path("M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h10.2a2 2 0 011.4.6l3.8 3.8a2 2 0 01.6 1.4v.3"),
        ],
        "select" => &[
            SvgPart::Path("M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"),
        ],
        "measure" => &[
            SvgPart::Rect("2|8|20|8|2"),
            SvgPart::Path("M6 8v3"),
            SvgPart::Path("M10 8v3"),
            SvgPart::Path("M14 8v3"),
            SvgPart::Path("M18 8v3"),
        ],
        "draw_poly" => &[
            SvgPart::Path("m12.828 4.813 5.344 2.375"),
            SvgPart::Path("m15.769 18.153 3.461-8.306"),
            SvgPart::Path("m5.687 14.074 7.625 4.852"),
            SvgPart::Path("M9.772 5.579 5.228 11.42"),
            SvgPart::Circle("11|4|2"),
            SvgPart::Circle("15|20|2"),
            SvgPart::Circle("20|8|2"),
            SvgPart::Circle("4|13|2"),
        ],
        "trench_grid" => &[
            SvgPart::Rect("3|3|18|18|2"),
            SvgPart::Path("M3 9h18"),
            SvgPart::Path("M3 15h18"),
            SvgPart::Path("M9 3v18"),
            SvgPart::Path("M15 3v18"),
        ],
        "buffer" => &[
            SvgPart::Path("M10.1 2.18a9.93 9.93 0 0 1 3.8 0"),
            SvgPart::Path("M17.6 3.71a9.95 9.95 0 0 1 2.69 2.7"),
            SvgPart::Path("M21.82 10.1a9.93 9.93 0 0 1 0 3.8"),
            SvgPart::Path("M20.29 17.6a9.95 9.95 0 0 1-2.7 2.69"),
            SvgPart::Path("M13.9 21.82a9.94 9.94 0 0 1-3.8 0"),
            SvgPart::Path("M6.4 20.29a9.95 9.95 0 0 1-2.69-2.7"),
            SvgPart::Path("M2.18 13.9a9.93 9.93 0 0 1 0-3.8"),
            SvgPart::Path("M3.71 6.4a9.95 9.95 0 0 1 2.7-2.69"),
            SvgPart::Circle("12|12|1"),
        ],
        "cadastral" => &[
            SvgPart::Rect("3|3|18|18|2"),
            SvgPart::Path("M14 3v18"),
            SvgPart::Path("M3 11h11"),
        ],
        "topo_download" => &[
            SvgPart::Path("M12 15V3"),
            SvgPart::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
            SvgPart::Path("m7 10 5 5 5-5"),
        ],
        "heritage" => &[
            SvgPart::Path("M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"),
            SvgPart::Path("M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"),
        ],
        "contour" => &[
            SvgPart::Path("m8 3 4 8 5-5 5 15H2L8 3z"),
        ],
        "dem" => &[
            SvgPart::Path("m8 3 4 8 5-5 5 15H2L8 3z"),
            SvgPart::Path("M4.14 15.08c2.62-1.57 5.24-1.43 7.86.42 2.74 1.94 5.49 2 8.23.19"),
        ],
        "survey_contour" => &[
            SvgPart::Circle("12|12|10"),
            SvgPart::Circle("12|12|6"),
            SvgPart::Circle("12|12|2"),
        ],
        "soil" => &[
            SvgPart::Path("M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"),
            SvgPart::Path("M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"),
            SvgPart::Path("M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"),
        ],
        "paleo" => &[
            SvgPart::Path("M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"),
            SvgPart::Path("M3 3v5h5"),
            SvgPart::Path("M12 7v5l4 2"),
        ],
        "geology" => &[
            SvgPart::Path("M3 9h18"),
            SvgPart::Path("M3 14h18"),
            SvgPart::Path("M3 19h18"),
            SvgPart::Path("M10 5v9"),
        ],
        "river" => &[
            SvgPart::Path("M2 12q2.5 2 5 0t5 0 5 0 5 0"),
            SvgPart::Path("M2 19q2.5 2 5 0t5 0 5 0 5 0"),
            SvgPart::Path("M2 5q2.5 2 5 0t5 0 5 0 5 0"),
        ],
        "old_map" => &[
            SvgPart::Path("M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"),
            SvgPart::Path("M15 5.764v15"),
            SvgPart::Path("M9 3.236v15"),
        ],
        "georef" => &[
            SvgPart::Line("0|0|0|0"),
            SvgPart::Line("0|0|0|0"),
            SvgPart::Line("0|0|0|0"),
            SvgPart::Line("0|0|0|0"),
            SvgPart::Circle("12|12|7"),
            SvgPart::Circle("12|12|3"),
        ],
        "pdf" => &[
            SvgPart::Path("M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"),
            SvgPart::Path("M14 2v5a1 1 0 0 0 1 1h5"),
            SvgPart::Path("M10 9H8"),
            SvgPart::Path("M16 13H8"),
            SvgPart::Path("M16 17H8"),
        ],
        "print" => &[
            SvgPart::Path("M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"),
            SvgPart::Path("M6 9V3a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v6"),
            SvgPart::Rect("6|14|12|8|1"),
        ],
        "section" => &[
            SvgPart::Path("M3 3v16a2 2 0 0 0 2 2h16"),
            SvgPart::Path("m19 9-5 5-4-4-3 3"),
        ],
        "geotiff" => &[
            SvgPart::Rect("3|3|18|18|2"),
            SvgPart::Circle("9|9|2"),
            SvgPart::Path("m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"),
        ],
        "export_convert" => &[
            SvgPart::Path("M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"),
            SvgPart::Path("m3.3 7 8.7 5 8.7-5"),
            SvgPart::Path("M12 22V12"),
        ],
        "more" => &[
            SvgPart::Circle("12|12|1"),
            SvgPart::Circle("19|12|1"),
            SvgPart::Circle("5|12|1"),
        ],
        "home" => &[
            SvgPart::Path("M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"),
            SvgPart::Path("M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"),
        ],
        "map" => &[
            SvgPart::Path("M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"),
            SvgPart::Path("M15 5.764v15"),
            SvgPart::Path("M9 3.236v15"),
        ],
        "region" => &[
            SvgPart::Path("M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0"),
            SvgPart::Circle("12|10|3"),
        ],
        "search" => &[
            SvgPart::Path("m21 21-4.34-4.34"),
            SvgPart::Circle("11|11|8"),
        ],
        "undo" => &[
            SvgPart::Path("M9 14 4 9l5-5"),
            SvgPart::Path("M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"),
        ],
        "redo" => &[
            SvgPart::Path("m15 14 5-5-5-5"),
            SvgPart::Path("M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13"),
        ],
        "zoom_in" => &[
            SvgPart::Path("M5 12h14"),
            SvgPart::Path("M12 5v14"),
        ],
        "zoom_out" => &[
            SvgPart::Path("M5 12h14"),
        ],
        "zoom_fit" => &[
            SvgPart::Path("M8 3H5a2 2 0 0 0-2 2v3"),
            SvgPart::Path("M21 8V5a2 2 0 0 0-2-2h-3"),
            SvgPart::Path("M3 16v3a2 2 0 0 0 2 2h3"),
            SvgPart::Path("M16 21h3a2 2 0 0 0 2-2v-3"),
        ],
        "folder" => &[
            SvgPart::Path("M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"),
        ],
        "note" => &[
            SvgPart::Rect("8|2|8|4|1"),
            SvgPart::Path("M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"),
            SvgPart::Path("M12 11h4"),
            SvgPart::Path("M12 16h4"),
            SvgPart::Path("M8 11h.01"),
            SvgPart::Path("M8 16h.01"),
        ],
        "layer" => &[
            SvgPart::Path("M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"),
            SvgPart::Path("M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"),
            SvgPart::Path("M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"),
        ],
        "warn" => &[
            SvgPart::Path("m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"),
            SvgPart::Path("M12 9v4"),
            SvgPart::Path("M12 17h.01"),
        ],
        "snap" => &[
            SvgPart::Path("m12 15 4 4"),
            SvgPart::Path("M2.352 10.648a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.029-6.029a1 1 0 1 1 3 3l-6.029 6.029a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.365-6.367A1 1 0 0 0 8.716 4.282z"),
            SvgPart::Path("m5 8 4 4"),
        ],
        "chevron_left" => &[
            SvgPart::Path("m15 18-6-6 6-6"),
        ],
        "chevron_right" => &[
            SvgPart::Path("m9 18 6-6-6-6"),
        ],
        "satellite" => &[
            SvgPart::Path("m13.5 6.5-3.148-3.148a1.205 1.205 0 0 0-1.704 0L6.352 5.648a1.205 1.205 0 0 0 0 1.704L9.5 10.5"),
            SvgPart::Path("M16.5 7.5 19 5"),
            SvgPart::Path("m17.5 10.5 3.148 3.148a1.205 1.205 0 0 1 0 1.704l-2.296 2.296a1.205 1.205 0 0 1-1.704 0L13.5 14.5"),
            SvgPart::Path("M9 21a6 6 0 0 0-6-6"),
            SvgPart::Path("M9.352 10.648a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l4.296-4.296a1.205 1.205 0 0 0 0-1.704l-2.296-2.296a1.205 1.205 0 0 0-1.704 0z"),
        ],
        "check" => &[
            SvgPart::Path("M20 6 9 17l-5-5"),
        ],
        "stop" => &[
            SvgPart::Path("M18 6 6 18"),
            SvgPart::Path("m6 6 12 12"),
        ],
        "draw_area" => &[
            SvgPart::Path("M5 3a2 2 0 0 0-2 2"),
            SvgPart::Path("M19 3a2 2 0 0 1 2 2"),
            SvgPart::Path("M21 19a2 2 0 0 1-2 2"),
            SvgPart::Path("M5 21a2 2 0 0 1-2-2"),
            SvgPart::Path("M9 3h1"),
            SvgPart::Path("M9 21h1"),
            SvgPart::Path("M14 3h1"),
            SvgPart::Path("M14 21h1"),
            SvgPart::Path("M3 9v1"),
            SvgPart::Path("M21 9v1"),
            SvgPart::Path("M3 14v1"),
            SvgPart::Path("M21 14v1"),
        ],
        "draw_line" => &[
            SvgPart::Path("M13 21h8"),
            SvgPart::Path("M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"),
        ],
        "easy_draw" => &[
            SvgPart::Path("M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"),
            SvgPart::Path("m15 5 4 4"),
        ],
        "artifact" => &[
            SvgPart::Path("M10 2v5.632c0 .424-.272.795-.653.982A6 6 0 0 0 6 14c.006 4 3 7 5 8"),
            SvgPart::Path("M10 5H8a2 2 0 0 0 0 4h.68"),
            SvgPart::Path("M14 2v5.632c0 .424.272.795.652.982A6 6 0 0 1 18 14c0 4-3 7-5 8"),
            SvgPart::Path("M14 5h2a2 2 0 0 1 0 4h-.68"),
            SvgPart::Path("M18 22H6"),
            SvgPart::Path("M9 2h6"),
        ],
        "transform" => &[
            SvgPart::Path("M12 2v20"),
            SvgPart::Path("m15 19-3 3-3-3"),
            SvgPart::Path("m19 9 3 3-3 3"),
            SvgPart::Path("M2 12h20"),
            SvgPart::Path("m5 9-3 3 3 3"),
            SvgPart::Path("m9 5 3-3 3 3"),
        ],
        _ => return None,
    })
}
