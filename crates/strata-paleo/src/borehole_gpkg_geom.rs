//! GeoPackage 지오메트리 블롭 인코딩 — `crates/strata-storage/src/wkb_codec.rs:55-90` 의
//! `encode_gpkg_wkb` 와 같은 바이트 배치를 이 크레이트 안에서 재구현한다
//! (strata-storage 에 의존하지 않는다).

/// GeoPackage `application_id` = `'GPKG'` (`0x47504B47`).
pub const GPKG_APPLICATION_ID: i32 = 1_196_444_487;

/// GeoPackage 1.2.0 `user_version` (`10200`). 프로젝트 조사 파일은 1.3.0(10300)을 쓰지만
/// 이 산출물은 GDAL/QGIS 읽기 호환이 목적이므로 1.2.0 으로 충분하다.
pub const GPKG_USER_VERSION: i32 = 10200;

/// C++ 지오메트리 타입 (`GeoInfoBoreholeMaps.cpp:91` 의 `LineString?crs=...`).
pub const GPKG_GEOMETRY_TYPE: &str = "LINESTRING";

/// GPKG 지오메트리 flags — bit0=1 리틀엔디언, bit1~3=1 (XY 봉투) => `0b0000_0011`.
///
/// `wkb_codec::decode_gpkg_wkb` 도 `env_ind = (flags >> 1) & 0x07` 로 봉투 유무를
/// 판정하므로 flags 는 반드시 `3` 이어야 한다. `1` 로 쓰면 봉투를 기록하고도
/// GDAL 이 `Unsupported WKB type 0` 으로 지오메트리를 못 읽는다.
pub const GPKG_GEOM_FLAGS_LE_XY: u8 = 0x03;

/// GPKG 지오메트리 블롭 헤더(8바이트) + WKB LineString 을 만든다.
///
/// 배치: `"GP"`, version=0, flags=3, srs_id(i32 LE), envelope(minx,maxx,miny,maxy),
/// 이어서 WKB(byte order=1, type=2, 개수, 각 점 x/y).
#[must_use]
pub fn encode_gpkg_linestring(srs_id: i32, points: &[[f64; 2]]) -> Vec<u8> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }
    if !min_x.is_finite() {
        min_x = 0.0;
        max_x = 0.0;
        min_y = 0.0;
        max_y = 0.0;
    }

    let mut out = Vec::with_capacity(8 + 32 + 9 + points.len() * 16);
    out.extend_from_slice(b"GP");
    out.push(0); // version
    out.push(GPKG_GEOM_FLAGS_LE_XY);
    out.extend_from_slice(&srs_id.to_le_bytes());
    for v in [min_x, max_x, min_y, max_y] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.push(1); // WKB byte order: little-endian
    out.extend_from_slice(&2_u32.to_le_bytes()); // wkbLineString
    out.extend_from_slice(&(points.len() as u32).to_le_bytes());
    for p in points {
        out.extend_from_slice(&p[0].to_le_bytes());
        out.extend_from_slice(&p[1].to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpkg_geometry_blob_has_gp_header_and_wkb_linestring() {
        let blob = encode_gpkg_linestring(5187, &[[1.0, 2.0], [3.0, 4.0]]);
        assert_eq!(&blob[0..2], b"GP");
        assert_eq!(blob[2], 0, "version");
        assert_eq!(blob[3], GPKG_GEOM_FLAGS_LE_XY);
        assert_eq!((blob[3] >> 1) & 0x07, 1, "envelope indicator must be XY");
        assert_eq!(blob[3] & 0x01, 1, "byte order must be little-endian");
        let srs = i32::from_le_bytes([blob[4], blob[5], blob[6], blob[7]]);
        assert_eq!(srs, 5187);

        let f = |o: usize| {
            let mut b = [0_u8; 8];
            b.copy_from_slice(&blob[o..o + 8]);
            f64::from_le_bytes(b)
        };
        assert_eq!(f(8), 1.0);
        assert_eq!(f(16), 3.0);
        assert_eq!(f(24), 2.0);
        assert_eq!(f(32), 4.0);
        assert_eq!(blob[40], 1);
        assert_eq!(
            u32::from_le_bytes([blob[41], blob[42], blob[43], blob[44]]),
            2
        );
        assert_eq!(
            u32::from_le_bytes([blob[45], blob[46], blob[47], blob[48]]),
            2,
            "2 points"
        );
    }

    #[test]
    fn empty_point_list_still_encodes_zero_length_linestring() {
        let blob = encode_gpkg_linestring(5186, &[]);
        assert_eq!(&blob[0..2], b"GP");
        assert_eq!(
            u32::from_le_bytes([blob[45], blob[46], blob[47], blob[48]]),
            0
        );
        // envelope 은 비유한 대신 0.0 으로 방어된다.
        let f = |o: usize| {
            let mut b = [0_u8; 8];
            b.copy_from_slice(&blob[o..o + 8]);
            f64::from_le_bytes(b)
        };
        assert_eq!(f(8), 0.0);
        assert_eq!(f(16), 0.0);
    }
}
