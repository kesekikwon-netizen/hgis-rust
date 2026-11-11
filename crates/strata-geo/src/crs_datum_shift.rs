//! 옛 지적 좌표계(EPSG:5174, Bessel 타원체)와 KGD2002(GRS80) 사이의 측지계 변환.
//!
//! 국토부 연속지적도 SHP 는 EPSG:5174 로 오는 일이 많다. C++ 은 QGIS(PROJ)가 바꾼다.
//! PROJ 가 고르는 변환 「Korean 1985 to KGD2002 (1)」(EPSG:1930 계열, 정확도 약 1 m)과 같은
//! Molodensky-Badekas 10매개변수(좌표틀 회전 방식)를 쓴다:
//! `x=-145.907 y=505.034 z=685.756 rx=-1.162 ry=2.347 rz=1.592 s=6.342`,
//! 회전 중심 `(-3159521.31, 4068151.32, 3748113.85)`.

/// Bessel 1841 타원체.
pub const BESSEL_A: f64 = 6_377_397.155;
pub const BESSEL_F: f64 = 1.0 / 299.152_812_8;

const SHIFT: [f64; 3] = [-145.907, 505.034, 685.756];
/// 회전(초) rx, ry, rz.
const ROT_ARCSEC: [f64; 3] = [-1.162, 2.347, 1.592];
const SCALE_PPM: f64 = 6.342;
const PIVOT: [f64; 3] = [-3_159_521.31, 4_068_151.32, 3_748_113.85];

fn to_cartesian(lon_deg: f64, lat_deg: f64, a: f64, f: f64) -> [f64; 3] {
    let e2 = f * (2.0 - f);
    let (sin_lat, cos_lat) = lat_deg.to_radians().sin_cos();
    let (sin_lon, cos_lon) = lon_deg.to_radians().sin_cos();
    let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    [n * cos_lat * cos_lon, n * cos_lat * sin_lon, n * (1.0 - e2) * sin_lat]
}

fn to_geodetic(c: [f64; 3], a: f64, f: f64) -> [f64; 2] {
    let e2 = f * (2.0 - f);
    let p = (c[0] * c[0] + c[1] * c[1]).sqrt();
    let mut lat = (c[2] / (p * (1.0 - e2))).atan();
    for _ in 0..8 {
        let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        lat = ((c[2] + e2 * n * lat.sin()) / p).atan();
    }
    [c[1].atan2(c[0]).to_degrees(), lat.to_degrees()]
}

/// 좌표틀 회전 행렬(작은 각 근사)을 곱한다. `sign` 이 -1 이면 역회전.
fn rotate(v: [f64; 3], sign: f64) -> [f64; 3] {
    let arcsec = std::f64::consts::PI / (180.0 * 3600.0);
    let [rx, ry, rz] = ROT_ARCSEC.map(|r| r * arcsec * sign);
    [
        v[0] + rz * v[1] - ry * v[2],
        -rz * v[0] + v[1] + rx * v[2],
        ry * v[0] - rx * v[1] + v[2],
    ]
}

/// Bessel 경위도(도) → GRS80(KGD2002) 경위도(도).
#[must_use]
pub fn bessel_to_grs80(lon_deg: f64, lat_deg: f64, grs80_a: f64, grs80_f: f64) -> [f64; 2] {
    let c = to_cartesian(lon_deg, lat_deg, BESSEL_A, BESSEL_F);
    let d = [c[0] - PIVOT[0], c[1] - PIVOT[1], c[2] - PIVOT[2]];
    let r = rotate(d, 1.0);
    let k = 1.0 + SCALE_PPM * 1e-6;
    let out = [
        SHIFT[0] + PIVOT[0] + k * r[0],
        SHIFT[1] + PIVOT[1] + k * r[1],
        SHIFT[2] + PIVOT[2] + k * r[2],
    ];
    to_geodetic(out, grs80_a, grs80_f)
}

/// GRS80(KGD2002) 경위도(도) → Bessel 경위도(도). 위 변환의 역.
#[must_use]
pub fn grs80_to_bessel(lon_deg: f64, lat_deg: f64, grs80_a: f64, grs80_f: f64) -> [f64; 2] {
    let c = to_cartesian(lon_deg, lat_deg, grs80_a, grs80_f);
    let k = 1.0 + SCALE_PPM * 1e-6;
    let d = [
        (c[0] - SHIFT[0] - PIVOT[0]) / k,
        (c[1] - SHIFT[1] - PIVOT[1]) / k,
        (c[2] - SHIFT[2] - PIVOT[2]) / k,
    ];
    let r = rotate(d, -1.0);
    to_geodetic([r[0] + PIVOT[0], r[1] + PIVOT[1], r[2] + PIVOT[2]], BESSEL_A, BESSEL_F)
}
