//! GPKG 도형(GeoPackage 1.4 §2.1.3 머리말 + OGC WKB)을 WKT 글로 푼다.
//!
//! C++ 은 조사 파일을 GDAL(OGR)로 읽어 어떤 도형이든 그대로 가진다(`LayerOps.cpp:1831-1913`).
//! Rust 판도 열 때 잃는 것이 없도록 GDAL 이 쓰는 범위를 모두 읽는다: 머리말 범위 0~4·빈 도형·
//! 머리말 바이트 순서, WKB 두 순서, Z·M·ZM(ISO 1000/2000/3000 과 EWKB 깃발), Multi* 와
//! GeometryCollection 은 부분을 나눈 그대로. `wkb_codec::decode_gpkg_wkb`(2차원 단일 도형)는
//! 그대로 둔다.

use strata_types::error::{StrataError, StrataResult};

fn bad(msg: &str) -> StrataError {
    StrataError::Storage(format!("GPKG 도형을 읽지 못했습니다: {msg}"))
}

/// 머리말의 좌표계 번호와 도형 WKT.
pub fn gpkg_blob_to_wkt(blob: &[u8]) -> StrataResult<(i32, String)> {
    if blob.len() < 8 || &blob[..2] != b"GP" {
        return Err(bad("GP 머리말이 아닙니다"));
    }
    let flags = blob[3];
    if flags & 0x20 != 0 {
        return Err(bad("확장 GeoPackage 도형"));
    }
    let header_le = flags & 0x01 == 0x01;
    let env_len = match (flags >> 1) & 0x07 {
        0 => 0,
        1 => 32,
        2 | 3 => 48,
        4 => 64,
        _ => return Err(bad("범위 표시 값이 잘못되었습니다")),
    };
    let srs = [blob[4], blob[5], blob[6], blob[7]];
    let srs_id = if header_le { i32::from_le_bytes(srs) } else { i32::from_be_bytes(srs) };
    let wkb = blob.get(8 + env_len..).ok_or_else(|| bad("범위가 잘렸습니다"))?;
    let mut r = WkbReader { b: wkb, pos: 0 };
    let mut out = String::new();
    r.geometry(&mut out, true)?;
    Ok((srs_id, out))
}

struct WkbReader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl WkbReader<'_> {
    fn take<const N: usize>(&mut self) -> StrataResult<[u8; N]> {
        let s = self.b.get(self.pos..self.pos + N).ok_or_else(|| bad("WKB 가 잘렸습니다"))?;
        self.pos += N;
        let mut a = [0_u8; N];
        a.copy_from_slice(s);
        Ok(a)
    }

    fn u32(&mut self, le: bool) -> StrataResult<u32> {
        let a = self.take::<4>()?;
        Ok(if le { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) })
    }

    fn f64(&mut self, le: bool) -> StrataResult<f64> {
        let a = self.take::<8>()?;
        Ok(if le { f64::from_le_bytes(a) } else { f64::from_be_bytes(a) })
    }

    /// 좌표 하나(차원 수만큼)를 "x y[ z][ m]" 로 쓴다. 모두 NaN 이면 None(빈 점).
    fn coord(&mut self, le: bool, dims: usize) -> StrataResult<Option<String>> {
        let mut vals = Vec::with_capacity(dims);
        for _ in 0..dims {
            vals.push(self.f64(le)?);
        }
        if vals.iter().all(|v| v.is_nan()) {
            return Ok(None);
        }
        Ok(Some(vals.iter().map(f64::to_string).collect::<Vec<_>>().join(" ")))
    }

    fn coord_list(&mut self, le: bool, dims: usize) -> StrataResult<String> {
        let n = self.u32(le)?;
        let mut pts = Vec::new();
        for _ in 0..n {
            pts.push(self.coord(le, dims)?.unwrap_or_default());
        }
        Ok(if pts.is_empty() { "EMPTY".to_owned() } else { format!("({})", pts.join(",")) })
    }

    /// WKB 도형 하나를 WKT 로 쓴다. `named` 이면 형식 이름을 앞에 붙인다(Multi* 의 부분은 붙이지 않는다).
    fn geometry(&mut self, out: &mut String, named: bool) -> StrataResult<()> {
        let le = match self.take::<1>()?[0] {
            0 => false,
            1 => true,
            _ => return Err(bad("WKB 바이트 순서 값이 잘못되었습니다")),
        };
        let raw = self.u32(le)?;
        let (mut z, mut m) = (raw & 0x8000_0000 != 0, raw & 0x4000_0000 != 0);
        if raw & 0x2000_0000 != 0 {
            self.u32(le)?; // EWKB SRID
        }
        let iso = raw & 0x0FFF_FFFF;
        match iso / 1000 {
            0 => {}
            1 => z = true,
            2 => m = true,
            3 => (z, m) = (true, true),
            _ => return Err(bad("WKB 형식 번호가 잘못되었습니다")),
        }
        let dims = 2 + usize::from(z) + usize::from(m);
        let base = iso % 1000;
        let name = match base {
            1 => "POINT",
            2 => "LINESTRING",
            3 => "POLYGON",
            4 => "MULTIPOINT",
            5 => "MULTILINESTRING",
            6 => "MULTIPOLYGON",
            7 => "GEOMETRYCOLLECTION",
            _ => return Err(bad("지원하지 않는 WKB 도형 형식")),
        };
        if named {
            out.push_str(name);
            out.push_str(match (z, m) {
                (true, true) => " ZM ",
                (true, false) => " Z ",
                (false, true) => " M ",
                (false, false) => " ",
            });
        }
        let body = match base {
            1 => self.coord(le, dims)?.map_or_else(|| "EMPTY".to_owned(), |c| format!("({c})")),
            2 => self.coord_list(le, dims)?,
            3 => {
                let n = self.u32(le)?;
                let mut rings = Vec::new();
                for _ in 0..n {
                    rings.push(self.coord_list(le, dims)?);
                }
                if rings.is_empty() { "EMPTY".to_owned() } else { format!("({})", rings.join(",")) }
            }
            _ => {
                let n = self.u32(le)?;
                let mut parts = Vec::new();
                for _ in 0..n {
                    let mut part = String::new();
                    self.geometry(&mut part, base == 7)?;
                    parts.push(part);
                }
                if parts.is_empty() { "EMPTY".to_owned() } else { format!("({})", parts.join(",")) }
            }
        };
        out.push_str(&body);
        Ok(())
    }
}
