use strata_types::crs::CrsCode;

#[derive(Debug, Clone, Copy)]
pub struct TmParams {
    pub lon0_deg: f64,
    pub lat0_deg: f64,
    pub k0: f64,
    pub false_e: f64,
    pub false_n: f64,
}

impl TmParams {
    pub const fn for_crs(crs: CrsCode) -> Option<Self> {
        match crs {
            CrsCode::Epsg5187 => Some(Self {
                lon0_deg: 129.0,
                lat0_deg: 38.0,
                k0: 1.0,
                false_e: 200_000.0,
                false_n: 600_000.0,
            }),
            CrsCode::Epsg5186 => Some(Self {
                lon0_deg: 127.0,
                lat0_deg: 38.0,
                k0: 1.0,
                false_e: 200_000.0,
                false_n: 600_000.0,
            }),
            CrsCode::Epsg5179 => Some(Self {
                lon0_deg: 127.5,
                lat0_deg: 38.0,
                k0: 0.9996,
                false_e: 1_000_000.0,
                false_n: 2_000_000.0,
            }),
            CrsCode::Epsg5174 => Some(Self {
                lon0_deg: 127.002_890_277_777_8,
                lat0_deg: 38.0,
                k0: 1.0,
                false_e: 200_000.0,
                false_n: 500_000.0,
            }),
            _ => None,
        }
    }
}

const GRS80_A: f64 = 6_378_137.0;
const GRS80_F: f64 = 1.0 / 298.257_222_101;

/// 횡단 메르카토르(TM) 계산에 쓰는 Krüger 급수 계수(GRS80).
///
/// PROJ 의 `tmerc`(정확한 알고리즘)와 같은 계열의 공식이다. n³ 항까지 쓰면 한국 범위에서
/// 오차가 0.1 mm 아래다. 예전의 Snyder 근사식은 PROJ 와 북쪽으로 9 mm 쯤 어긋났다.
struct Kruger {
    /// 자오선 반경 A = a/(1+n)·(1 + n²/4 + n⁴/64)
    radius: f64,
    /// 2√n/(1+n) — 등각위도 계산에 쓴다.
    e_conf: f64,
    alpha: [f64; 3],
    beta: [f64; 3],
    delta: [f64; 3],
}

impl Kruger {
    fn grs80() -> Self {
        let n = GRS80_F / (2.0 - GRS80_F);
        let (n2, n3) = (n * n, n * n * n);
        Self {
            radius: GRS80_A / (1.0 + n) * (1.0 + n2 / 4.0 + n2 * n2 / 64.0),
            e_conf: 2.0 * n.sqrt() / (1.0 + n),
            alpha: [n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0, 13.0 * n2 / 48.0 - 3.0 * n3 / 5.0, 61.0 * n3 / 240.0],
            beta: [n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0, n2 / 48.0 + n3 / 15.0, 17.0 * n3 / 480.0],
            delta: [2.0 * n - 2.0 * n2 / 3.0 - 2.0 * n3, 7.0 * n2 / 3.0 - 8.0 * n3 / 5.0, 56.0 * n3 / 15.0],
        }
    }

    /// 등각위도의 tan (위도 → t).
    fn conformal_tan(&self, lat_rad: f64) -> f64 {
        let sin_lat = lat_rad.sin();
        (sin_lat.atanh() - self.e_conf * (self.e_conf * sin_lat).atanh()).sinh()
    }

    /// (위도, 중앙 자오선에서의 경도차) → (ξ, η)
    fn forward(&self, lat_rad: f64, dlon_rad: f64) -> (f64, f64) {
        let t = self.conformal_tan(lat_rad);
        let xi_p = t.atan2(dlon_rad.cos());
        let eta_p = (dlon_rad.sin() / (1.0 + t * t).sqrt()).atanh();
        let (mut xi, mut eta) = (xi_p, eta_p);
        for (j, a) in self.alpha.iter().enumerate() {
            let k = 2.0 * (j as f64 + 1.0);
            xi += a * (k * xi_p).sin() * (k * eta_p).cosh();
            eta += a * (k * xi_p).cos() * (k * eta_p).sinh();
        }
        (xi, eta)
    }

    /// (ξ, η) → (위도, 중앙 자오선에서의 경도차)
    fn inverse(&self, xi: f64, eta: f64) -> (f64, f64) {
        let (mut xi_p, mut eta_p) = (xi, eta);
        for (j, b) in self.beta.iter().enumerate() {
            let k = 2.0 * (j as f64 + 1.0);
            xi_p -= b * (k * xi).sin() * (k * eta).cosh();
            eta_p -= b * (k * xi).cos() * (k * eta).sinh();
        }
        let chi = (xi_p.sin() / eta_p.cosh()).asin();
        let mut lat = chi;
        for (j, d) in self.delta.iter().enumerate() {
            lat += d * (2.0 * (j as f64 + 1.0) * chi).sin();
        }
        (lat, eta_p.sinh().atan2(xi_p.cos()))
    }
}

/// 경위도(도) → TM 평면 좌표(m).
pub fn lonlat_to_tm(lon_deg: f64, lat_deg: f64, p: &TmParams) -> [f64; 2] {
    let k = Kruger::grs80();
    let (xi, eta) = k.forward(lat_deg.to_radians(), (lon_deg - p.lon0_deg).to_radians());
    let (xi0, _) = k.forward(p.lat0_deg.to_radians(), 0.0);
    [
        p.false_e + p.k0 * k.radius * eta,
        p.false_n + p.k0 * k.radius * (xi - xi0),
    ]
}

/// TM 평면 좌표(m) → 경위도(도).
pub fn tm_to_lonlat(x: f64, y: f64, p: &TmParams) -> [f64; 2] {
    let k = Kruger::grs80();
    let (xi0, _) = k.forward(p.lat0_deg.to_radians(), 0.0);
    let xi = (y - p.false_n) / (p.k0 * k.radius) + xi0;
    let eta = (x - p.false_e) / (p.k0 * k.radius);
    let (lat, dlon) = k.inverse(xi, eta);
    [p.lon0_deg + dlon.to_degrees(), lat.to_degrees()]
}

#[derive(Debug, Clone, Copy)]
pub struct CrsTransformer {
    pub src: CrsCode,
    pub dst: CrsCode,
}

impl CrsTransformer {
    pub const fn new(src: CrsCode, dst: CrsCode) -> Self {
        Self { src, dst }
    }

    pub fn transform_xy(&self, x: f64, y: f64) -> [f64; 2] {
        if self.src == self.dst {
            return [x, y];
        }
        let [lon, lat] = match self.src {
            CrsCode::Epsg4326 => [x, y],
            CrsCode::Epsg3857 | CrsCode::Epsg900913 => {
                let lon = (x / GRS80_A).to_degrees();
                let lat =
                    (2.0 * (y / GRS80_A).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
                [lon, lat]
            }
            other => {
                if let Some(p) = TmParams::for_crs(other) {
                    tm_to_lonlat(x, y, &p)
                } else {
                    [x, y]
                }
            }
        };
        match self.dst {
            CrsCode::Epsg4326 => [lon, lat],
            CrsCode::Epsg3857 | CrsCode::Epsg900913 => {
                let mx = GRS80_A * lon.to_radians();
                let my =
                    GRS80_A * ((std::f64::consts::FRAC_PI_4 + lat.to_radians() * 0.5).tan()).ln();
                [mx, my]
            }
            other => {
                if let Some(p) = TmParams::for_crs(other) {
                    lonlat_to_tm(lon, lat, &p)
                } else {
                    [lon, lat]
                }
            }
        }
    }
}

#[cfg(test)]
mod accuracy_tests {
    //! 기준값은 PROJ(pyproj 3.7.2)로 계산했다. 제출용 EPSG:5179 좌표가 PROJ 와 1 mm 안에서
    //! 같아야 한다 — C++ 은 `QgsCoordinateTransform`(PROJ)으로 바꾸기 때문이다.
    use super::*;

    /// (원 좌표계, x, y, 5179 x, 5179 y, 경도, 위도)
    #[allow(clippy::unreadable_literal)]
    const CASES: [(CrsCode, f64, f64, f64, f64, f64, f64); 8] = [
        (CrsCode::Epsg5187, 219745.6, 360930.5, 1155199.5965, 1762367.3595, 129.218571716, 35.845560725),
        (CrsCode::Epsg5187, 200000.0, 600000.0, 1131699.6695, 2001061.5033, 129.0, 38.0),
        (CrsCode::Epsg5187, 150000.0, 250000.0, 1087157.2974, 1650418.0667, 128.453317820, 34.844679066),
        (CrsCode::Epsg5187, 260000.0, 480000.0, 1193597.9007, 1882042.3852, 129.673332294, 36.916877034),
        (CrsCode::Epsg5186, 200050.0, 450040.0, 955358.0184, 1850216.1258, 127.000559163, 36.648811153),
        (CrsCode::Epsg5186, 200000.0, 600000.0, 956101.1997, 2000117.9284, 127.0, 38.0),
        (CrsCode::Epsg5186, 170000.0, 300000.0, 924550.3377, 1700385.8370, 126.670178237, 35.296144325),
        (CrsCode::Epsg5186, 245000.0, 520000.0, 1000656.3695, 1919911.3852, 127.507403976, 37.278124759),
    ];

    #[test]
    fn work_crs_to_submit_crs_matches_proj_within_a_millimetre() {
        for (src, x, y, ex, ey, _, _) in CASES {
            let [gx, gy] = CrsTransformer::new(src, CrsCode::Epsg5179).transform_xy(x, y);
            assert!((gx - ex).abs() < 0.001 && (gy - ey).abs() < 0.001, "{src:?} ({x}, {y}) → ({gx:.4}, {gy:.4}), PROJ ({ex}, {ey})");
        }
    }

    #[test]
    fn work_crs_to_lonlat_matches_proj_and_comes_back_within_a_millimetre() {
        for (src, x, y, _, _, elon, elat) in CASES {
            let [lon, lat] = CrsTransformer::new(src, CrsCode::Epsg4326).transform_xy(x, y);
            assert!((lon - elon).abs() < 1e-8 && (lat - elat).abs() < 1e-8, "{src:?} → ({lon:.9}, {lat:.9}), PROJ ({elon}, {elat})");
            let [bx, by] = CrsTransformer::new(CrsCode::Epsg4326, src).transform_xy(lon, lat);
            assert!((bx - x).abs() < 0.001 && (by - y).abs() < 0.001, "왕복 ({bx:.4}, {by:.4})");
        }
    }
}
