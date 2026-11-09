//! GeoJSON 도형 → WKT 변환 (C++ `geometryToWkt` 계열).
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


/// C++ `AdminBoundaryService.cpp:109-125` `geometryToWkt`
pub(super) fn geometry_to_wkt(geom: &serde_json::Value) -> String {
    let kind = geom.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let rings = geom.get("coordinates");
    let Some(rings) = rings else {
        return String::new();
    };
    if kind.eq_ignore_ascii_case("Polygon") {
        polygon_wkt(rings)
    } else if kind.eq_ignore_ascii_case("MultiPolygon") {
        let Some(polys) = rings.as_array() else {
            return String::new();
        };
        let mut parts = Vec::new();
        for poly in polys {
            let w = polygon_wkt(poly);
            // "POLYGON(" 8자와 끝 ')' 를 떼어 MULTIPOLYGON 안에 넣는다.
            if w.starts_with("POLYGON(") && w.ends_with(')') {
                parts.push(w[8..w.len() - 1].to_owned());
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("MULTIPOLYGON({})", parts.join(", "))
        }
    } else {
        String::new()
    }
}

/// C++ `AdminBoundaryService.cpp:99-107` `polygonWkt`
fn polygon_wkt(rings: &serde_json::Value) -> String {
    let Some(rings) = rings.as_array() else {
        return String::new();
    };
    let mut parts = Vec::new();
    for ring in rings {
        let ring = ring_wkt(ring);
        if !ring.is_empty() {
            parts.push(format!("({ring})"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("POLYGON({})", parts.join(", "))
    }
}

/// C++ `AdminBoundaryService.cpp:90-97` `ringWkt`
fn ring_wkt(ring: &serde_json::Value) -> String {
    let Some(points) = ring.as_array() else {
        return String::new();
    };
    let mut pts = Vec::new();
    for point in points {
        let Some(pair) = point.as_array() else {
            continue;
        };
        if pair.len() < 2 {
            continue;
        }
        let num = |v: &serde_json::Value| -> Option<f64> {
            match v {
                serde_json::Value::String(s) => s.parse::<f64>().ok(),
                serde_json::Value::Number(n) => n.as_f64(),
                _ => None,
            }
        };
        let (Some(x), Some(y)) = (num(&pair[0]), num(&pair[1])) else {
            continue;
        };
        pts.push(format!("{x:.8} {y:.8}"));
    }
    pts.join(", ")
}
