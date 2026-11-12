//! GPKG 도형 → WKT 시험. C++ 조사 파일(GDAL)이 쓸 수 있는 형식을 손으로 만든 바이트로
//! 하나씩 확인한다 — 머리말 범위 0~4·빈 도형·바이트 순서 둘, WKB 두 순서, Z·M·ZM(ISO·EWKB),
//! Multi* 는 부분을 나누어 그대로, GeometryCollection.

use strata_types::error::StrataResult;

use crate::gpkg_geometry::gpkg_blob_to_wkt;

/// GPKG 머리말(+ 범위 `env` 개 f64) 뒤에 WKB 를 붙인다. `le` 는 머리말 바이트 순서.
fn gp(srs: i32, env_ind: u8, env: &[f64], le: bool, empty: bool, wkb: &[u8]) -> Vec<u8> {
    let flags = (env_ind << 1) | u8::from(le) | if empty { 0x10 } else { 0 };
    let mut out = vec![b'G', b'P', 0, flags];
    out.extend_from_slice(&if le { srs.to_le_bytes() } else { srs.to_be_bytes() });
    for v in env {
        out.extend_from_slice(&if le { v.to_le_bytes() } else { v.to_be_bytes() });
    }
    out.extend_from_slice(wkb);
    out
}

/// WKB 조각을 이어 붙이는 도우미(바이트 순서 하나로).
struct Wkb {
    le: bool,
    out: Vec<u8>,
}

impl Wkb {
    fn new(le: bool) -> Self {
        Self { le, out: Vec::new() }
    }
    fn head(mut self, ty: u32) -> Self {
        self.out.push(u8::from(self.le));
        self.u32(ty)
    }
    fn u32(mut self, v: u32) -> Self {
        self.out.extend_from_slice(&if self.le { v.to_le_bytes() } else { v.to_be_bytes() });
        self
    }
    fn f(mut self, vs: &[f64]) -> Self {
        for v in vs {
            self.out.extend_from_slice(&if self.le { v.to_le_bytes() } else { v.to_be_bytes() });
        }
        self
    }
    fn raw(mut self, bytes: &[u8]) -> Self {
        self.out.extend_from_slice(bytes);
        self
    }
}

fn ring(le: bool, pts: &[[f64; 2]]) -> Wkb {
    let mut w = Wkb::new(le).u32(pts.len() as u32);
    for p in pts {
        w = w.f(p);
    }
    w
}

#[test]
fn point_and_linestring_in_both_byte_orders_and_every_envelope_kind() -> StrataResult<()> {
    let p_le = Wkb::new(true).head(1).f(&[200_000.5, 600_000.25]).out;
    let p_be = Wkb::new(false).head(1).f(&[200_000.5, 600_000.25]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &p_le))?, (5186, "POINT (200000.5 600000.25)".to_owned()));
    assert_eq!(gpkg_blob_to_wkt(&gp(5187, 0, &[], false, false, &p_be))?, (5187, "POINT (200000.5 600000.25)".to_owned()));

    let line = Wkb::new(true).head(2).u32(2).f(&[0.0, 0.0, 10.0, 5.5]).out;
    let want = (5186, "LINESTRING (0 0,10 5.5)".to_owned());
    for (ind, n) in [(0_u8, 0_usize), (1, 4), (2, 6), (3, 6), (4, 8)] {
        let env = vec![1.0; n];
        assert_eq!(gpkg_blob_to_wkt(&gp(5186, ind, &env, true, false, &line))?, want, "envelope {ind}");
        assert_eq!(gpkg_blob_to_wkt(&gp(5186, ind, &env, false, false, &line))?, want, "BE envelope {ind}");
    }
    assert!(gpkg_blob_to_wkt(&gp(5186, 5, &[1.0; 8], true, false, &line)).is_err(), "envelope 5 is invalid");
    Ok(())
}

#[test]
fn z_m_and_zm_in_iso_and_ewkb_type_codes() -> StrataResult<()> {
    let line_z = Wkb::new(true).head(1002).u32(2).f(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &line_z))?.1, "LINESTRING Z (1 2 3,4 5 6)");

    let poly_m = Wkb::new(false).head(2003).u32(1).u32(4)
        .f(&[0.0, 0.0, 9.0, 1.0, 0.0, 9.0, 1.0, 1.0, 9.0, 0.0, 0.0, 9.0]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &poly_m))?.1, "POLYGON M ((0 0 9,1 0 9,1 1 9,0 0 9))");

    let point_zm = Wkb::new(true).head(3001).f(&[1.0, 2.0, 3.0, 4.0]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &point_zm))?.1, "POINT ZM (1 2 3 4)");

    let ewkb_z = Wkb::new(true).head(0x8000_0001).f(&[1.0, 2.0, 3.0]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &ewkb_z))?.1, "POINT Z (1 2 3)");
    let ewkb_srid = Wkb::new(true).head(0x2000_0001).u32(5186).f(&[1.0, 2.0]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &ewkb_srid))?.1, "POINT (1 2)");
    Ok(())
}

#[test]
fn multipolygon_parts_stay_separate_and_collections_and_empty_are_kept() -> StrataResult<()> {
    let a = ring(true, &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]).out;
    let b = ring(false, &[[5.0, 5.0], [6.0, 5.0], [6.0, 6.0], [5.0, 5.0]]).out;
    let multi = Wkb::new(true).head(6).u32(2)
        .raw(&Wkb::new(true).head(3).u32(1).raw(&a).out)
        .raw(&Wkb::new(false).head(3).u32(1).raw(&b).out)
        .out;
    assert_eq!(
        gpkg_blob_to_wkt(&gp(5186, 1, &[0.0, 6.0, 0.0, 6.0], true, false, &multi))?.1,
        "MULTIPOLYGON (((0 0,1 0,1 1,0 0)),((5 5,6 5,6 6,5 5)))"
    );

    let multi_point = Wkb::new(true).head(4).u32(2)
        .raw(&Wkb::new(true).head(1).f(&[1.0, 2.0]).out)
        .raw(&Wkb::new(true).head(1).f(&[3.0, 4.0]).out)
        .out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &multi_point))?.1, "MULTIPOINT ((1 2),(3 4))");

    let collection = Wkb::new(true).head(7).u32(2)
        .raw(&Wkb::new(true).head(1).f(&[1.0, 2.0]).out)
        .raw(&Wkb::new(true).head(2).u32(2).f(&[0.0, 0.0, 1.0, 1.0]).out)
        .out;
    assert_eq!(
        gpkg_blob_to_wkt(&gp(5186, 0, &[], true, false, &collection))?.1,
        "GEOMETRYCOLLECTION (POINT (1 2),LINESTRING (0 0,1 1))"
    );

    let empty_poly = Wkb::new(true).head(3).u32(0).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, true, &empty_poly))?.1, "POLYGON EMPTY");
    let empty_point = Wkb::new(true).head(1).f(&[f64::NAN, f64::NAN]).out;
    assert_eq!(gpkg_blob_to_wkt(&gp(5186, 0, &[], true, true, &empty_point))?.1, "POINT EMPTY");
    Ok(())
}

#[test]
fn malformed_headers_are_errors() {
    let p = Wkb::new(true).head(1).f(&[1.0, 2.0]).out;
    let mut bad_magic = gp(5186, 0, &[], true, false, &p);
    bad_magic[0] = b'X';
    assert!(gpkg_blob_to_wkt(&bad_magic).is_err());
    let mut extended = gp(5186, 0, &[], true, false, &p);
    extended[3] |= 0x20;
    assert!(gpkg_blob_to_wkt(&extended).is_err(), "extended GeoPackage geometry is not read");
    let truncated = gp(5186, 0, &[], true, false, &p[..10]);
    assert!(gpkg_blob_to_wkt(&truncated).is_err());
}
