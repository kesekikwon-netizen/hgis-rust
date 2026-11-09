//! GeoInfo 시추 XML 파싱.
//!
//! orehole_log_parse 가 313줄이라 여기로 나눴다.

use crate::borehole_log_parse::sanitize_borehole_strata_depths;

use crate::borehole_types::{BoreholeRawInput, StratumLayer};

fn extract_xml_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let pat = format!("{attr}=\"");
    let start = tag.find(&pat)? + pat.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// 국토지반정보(GeoInfo / K-Geo) XML 시추 주상도 텍스트에서 시추공 및 지층(`top`, `bottom`, `uscs`, `n`, `desc`) 파싱
/// (C++ `GeoInfoBoreholeParser::parseRawLog` 1:1 대응)
#[must_use]
pub fn parse_geoinfo_borehole_xml(xml_text: &str) -> Vec<BoreholeRawInput> {
    let mut records = Vec::new();
    for block in xml_text.split("<Borehole").skip(1) {
        let Some((header, body)) = block.split_once('>') else {
            continue;
        };
        let hole_id = extract_xml_attr(header, "id")
            .or_else(|| extract_xml_attr(header, "HCODE"))
            .unwrap_or("BH-1")
            .trim()
            .to_owned();
        let x = extract_xml_attr(header, "x")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(200_000.0);
        let y = extract_xml_attr(header, "y")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(350_000.0);
        let ground_elev_m = extract_xml_attr(header, "elevation")
            .or_else(|| extract_xml_attr(header, "ALTITUDE"))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let water_depth_m = extract_xml_attr(header, "water")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(5.0);

        let mut strata = Vec::new();
        let end_idx = body.find("</Borehole>").unwrap_or(body.len());
        for s_tag in body[..end_idx].split("<Stratum").skip(1) {
            let Some((s_attrs, _)) = s_tag.split_once('>') else {
                continue;
            };
            let top = extract_xml_attr(s_attrs, "top")
                .or_else(|| extract_xml_attr(s_attrs, "from"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let bottom = extract_xml_attr(s_attrs, "bottom")
                .or_else(|| extract_xml_attr(s_attrs, "to"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let uscs = extract_xml_attr(s_attrs, "uscs")
                .unwrap_or("")
                .trim()
                .to_owned();
            let desc = extract_xml_attr(s_attrs, "desc")
                .or_else(|| extract_xml_attr(s_attrs, "name"))
                .unwrap_or("")
                .trim()
                .to_owned();
            let n_val = extract_xml_attr(s_attrs, "n")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            strata.push(StratumLayer {
                depth_from_m: top,
                depth_to_m: bottom,
                layer_name: desc.clone(),
                uscs_code: uscs,
                description: desc,
                n_value: n_val,
            });
        }
        sanitize_borehole_strata_depths(&mut strata);
        records.push(BoreholeRawInput {
            hole_id,
            x,
            y,
            ground_elev_m,
            water_depth_m,
            strata,
        });
    }
    records
}
