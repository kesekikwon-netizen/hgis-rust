use strata_types::error::{StrataError, StrataResult};

#[derive(Debug, Clone, PartialEq)]
pub enum WkbGeometry {
    Point([f64; 2]),
    LineString(Vec<[f64; 2]>),
    Polygon(Vec<Vec<[f64; 2]>>),
}

impl WkbGeometry {
    #[must_use]
    pub fn envelope_xy(&self) -> [f64; 4] {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut visit = |pt: [f64; 2]| {
            min_x = min_x.min(pt[0]);
            max_x = max_x.max(pt[0]);
            min_y = min_y.min(pt[1]);
            max_y = max_y.max(pt[1]);
        };
        match self {
            Self::Point(p) => visit(*p),
            Self::LineString(pts) => {
                for &p in pts {
                    visit(p);
                }
            }
            Self::Polygon(rings) => {
                for r in rings {
                    for &p in r {
                        visit(p);
                    }
                }
            }
        }
        if !min_x.is_finite() {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            [min_x, max_x, min_y, max_y]
        }
    }
}

#[must_use]
pub fn encode_gpkg_wkb(srs_id: i32, geom: &WkbGeometry) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(b"GP\x00\x03");
    out.extend_from_slice(&srs_id.to_le_bytes());
    for v in geom.envelope_xy() {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.push(1);
    match geom {
        WkbGeometry::Point(pt) => {
            out.extend_from_slice(&1_u32.to_le_bytes());
            out.extend_from_slice(&pt[0].to_le_bytes());
            out.extend_from_slice(&pt[1].to_le_bytes());
        }
        WkbGeometry::LineString(pts) => {
            out.extend_from_slice(&2_u32.to_le_bytes());
            out.extend_from_slice(&(pts.len() as u32).to_le_bytes());
            for pt in pts {
                out.extend_from_slice(&pt[0].to_le_bytes());
                out.extend_from_slice(&pt[1].to_le_bytes());
            }
        }
        WkbGeometry::Polygon(rings) => {
            out.extend_from_slice(&3_u32.to_le_bytes());
            out.extend_from_slice(&(rings.len() as u32).to_le_bytes());
            for ring in rings {
                out.extend_from_slice(&(ring.len() as u32).to_le_bytes());
                for pt in ring {
                    out.extend_from_slice(&pt[0].to_le_bytes());
                    out.extend_from_slice(&pt[1].to_le_bytes());
                }
            }
        }
    }
    out
}

fn read_u32_le(buf: &[u8], idx: &mut usize) -> StrataResult<u32> {
    let slice = buf
        .get(*idx..*idx + 4)
        .ok_or_else(|| StrataError::Storage("WKB 카운트 읽기 범위 초과".into()))?;
    let mut arr = [0_u8; 4];
    arr.copy_from_slice(slice);
    *idx += 4;
    Ok(u32::from_le_bytes(arr))
}

fn read_f64_le(buf: &[u8], idx: &mut usize) -> StrataResult<f64> {
    let slice = buf
        .get(*idx..*idx + 8)
        .ok_or_else(|| StrataError::Storage("WKB 좌표 읽기 범위 초과".into()))?;
    let mut arr = [0_u8; 8];
    arr.copy_from_slice(slice);
    *idx += 8;
    Ok(f64::from_le_bytes(arr))
}

fn read_polygon_rings(wkb: &[u8], pos: &mut usize) -> StrataResult<Vec<Vec<[f64; 2]>>> {
    let n_rings = read_u32_le(wkb, pos)? as usize;
    let mut rings = Vec::with_capacity(n_rings);
    for _ in 0..n_rings {
        let n_pts = read_u32_le(wkb, pos)? as usize;
        let mut ring = Vec::with_capacity(n_pts);
        for _ in 0..n_pts {
            ring.push([read_f64_le(wkb, pos)?, read_f64_le(wkb, pos)?]);
        }
        rings.push(ring);
    }
    Ok(rings)
}

pub fn decode_gpkg_wkb(bytes: &[u8]) -> StrataResult<(i32, WkbGeometry)> {
    if bytes.len() < 8 || &bytes[0..2] != b"GP" {
        return Err(StrataError::Storage("유효한 GeoPackage WKB 헤더가 아닙니다".into()));
    }
    let flags = bytes[3];
    let env_ind = (flags >> 1) & 0x07;
    let env_bytes = match env_ind {
        0 => 0,
        1 => 32,
        2 | 3 => 48,
        4 => 64,
        _ => return Err(StrataError::Storage("지원하지 않는 GPKG envelope 코드".into())),
    };
    let srs_id = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let wkb_offset = 8 + env_bytes;
    if bytes.len() < wkb_offset + 5 {
        return Err(StrataError::Storage("WKB 페이로드 길이 부족".into()));
    }
    let wkb = &bytes[wkb_offset..];
    let mut pos = 1;
    let raw_type = read_u32_le(wkb, &mut pos)?;
    let geom_type = raw_type % 1000;

    let geom = match geom_type {
        1 => WkbGeometry::Point([read_f64_le(wkb, &mut pos)?, read_f64_le(wkb, &mut pos)?]),
        2 => {
            let n = read_u32_le(wkb, &mut pos)? as usize;
            let mut pts = Vec::with_capacity(n);
            for _ in 0..n {
                pts.push([read_f64_le(wkb, &mut pos)?, read_f64_le(wkb, &mut pos)?]);
            }
            WkbGeometry::LineString(pts)
        }
        3 => WkbGeometry::Polygon(read_polygon_rings(wkb, &mut pos)?),
        4 => {
            let n_sub = read_u32_le(wkb, &mut pos)? as usize;
            if n_sub == 0 {
                return Err(StrataError::Storage("빈 MultiPoint".into()));
            }
            pos += 5;
            WkbGeometry::Point([read_f64_le(wkb, &mut pos)?, read_f64_le(wkb, &mut pos)?])
        }
        5 => {
            let n_sub = read_u32_le(wkb, &mut pos)? as usize;
            let mut pts = Vec::new();
            for _ in 0..n_sub {
                pos += 5;
                let n = read_u32_le(wkb, &mut pos)? as usize;
                for _ in 0..n {
                    pts.push([read_f64_le(wkb, &mut pos)?, read_f64_le(wkb, &mut pos)?]);
                }
            }
            WkbGeometry::LineString(pts)
        }
        6 => {
            let n_polys = read_u32_le(wkb, &mut pos)? as usize;
            let mut all_rings = Vec::new();
            for _ in 0..n_polys {
                pos += 5; // child byteOrder (1) + child wkbType (4)
                let sub_rings = read_polygon_rings(wkb, &mut pos)?;
                all_rings.extend(sub_rings);
            }
            WkbGeometry::Polygon(all_rings)
        }
        other => {
            return Err(StrataError::Storage(format!(
                "지원하지 않는 WKB 지오메트리 타입: {other}"
            )))
        }
    };
    Ok((srs_id, geom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpkg_binary_header_and_wkb_polygon_roundtrip_preserves_srs_and_rings() -> StrataResult<()> {
        let poly = WkbGeometry::Polygon(vec![vec![
            [200_000.0, 500_000.0],
            [200_100.0, 500_000.0],
            [200_100.0, 500_100.0],
            [200_000.0, 500_100.0],
            [200_000.0, 500_000.0],
        ]]);
        let encoded = encode_gpkg_wkb(5187, &poly);
        assert_eq!(&encoded[0..2], b"GP");
        let (srs, decoded) = decode_gpkg_wkb(&encoded)?;
        assert_eq!(srs, 5187);
        assert_eq!(decoded, poly);
        Ok(())
    }
}
