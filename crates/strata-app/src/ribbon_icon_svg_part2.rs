//! Lucide SVG 아이콘 조각 데이터 (2/2).
//!
//! 원문은 `data/icons/lucide/*.svg` 이고, C++ `KaIconsMockupMap.cpp` 의 id→SVG 매핑을
//! 그대로 옮긴 것이다. 좌표계는 Lucide 의 24x24 viewBox 다.

use crate::ribbon_icon_svg::SvgPart;

/// 이 파일이 담당하는 아이콘 id 목록.
pub fn parts_2(id: &str) -> Option<&'static [SvgPart]> {
    Some(match id {
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
        // C++ KaIconsOutline::oGps on a 64x64 grid scaled by 24/64 to the 24x24 viewBox
        // (target/cpp-ref/src/app/KaIconsOutlineA.cpp:113-122, target/cpp-ref/src/app/KaIcons.cpp:1033-1036).
        "gps" => &[
            SvgPart::Path("M12 19.5C6 13.5 6 7.5 12 5.25C18 7.5 18 13.5 12 19.5Z"),
            SvgPart::Circle("12|10.5|1.875"),
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
