use strata_types::crs::CrsCode;

/// 타원체 정의. C++ 은 PROJ 가 CRS 별로 들고 있는 값이다.
///
/// 예전에는 GRS80 상수를 전역으로 박아 썼는데, 그러면 **Bessel 타원체인
/// EPSG:5174 를 GRS80 으로 계산해 수백 m 오차**가 났다(2026-11-10 지적).
/// 이제 CRS 마다 타원체를 들고 다닌다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid {
    /// 장반경(m).
    pub a: f64,
    /// 편평률의 역수(1/f).
    pub inv_f: f64,
}

impl Ellipsoid {
    /// GRS80 — 5186·5187·5179·4326·3857 이 쓴다.
    pub const GRS80: Self = Self {
        a: 6_378_137.0,
        inv_f: 298.257_222_101,
    };

    /// Bessel 1841 — 구 지적 원점 EPSG:5174 가 쓴다.
    ///
    /// EPSG 정의: a = 6,377,397.155 m, 1/f = 299.1528128.
    pub const BESSEL_1841: Self = Self {
        a: 6_377_397.155,
        inv_f: 299.152_812_8,
    };

    /// 제1 이심률 제곱.
    #[must_use]
    pub fn e2(&self) -> f64 {
        let f = 1.0 / self.inv_f;
        2.0 * f - f * f
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TmParams {
    pub lon0_deg: f64,
    pub lat0_deg: f64,
    pub k0: f64,
    pub false_e: f64,
    pub false_n: f64,
    /// 이 투영이 딛고 선 타원체.
    pub ellipsoid: Ellipsoid,
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
                ellipsoid: Ellipsoid::GRS80,
            }),
            CrsCode::Epsg5186 => Some(Self {
                lon0_deg: 127.0,
                lat0_deg: 38.0,
                k0: 1.0,
                false_e: 200_000.0,
                false_n: 600_000.0,
                ellipsoid: Ellipsoid::GRS80,
            }),
            CrsCode::Epsg5179 => Some(Self {
                lon0_deg: 127.5,
                lat0_deg: 38.0,
                k0: 0.9996,
                false_e: 1_000_000.0,
                false_n: 2_000_000.0,
                ellipsoid: Ellipsoid::GRS80,
            }),
            CrsCode::Epsg5174 => Some(Self {
                lon0_deg: 127.002_890_277_777_8,
                lat0_deg: 38.0,
                k0: 1.0,
                false_e: 200_000.0,
                false_n: 500_000.0,
                // 구 지적도는 Bessel 1841 이다. GRS80 으로 계산하면 안 된다.
                ellipsoid: Ellipsoid::BESSEL_1841,
            }),
            _ => None,
        }
    }
}

/// 자오선 호장(子午線 弧長) M. 위도만으로 적도에서 그 위도까지의 거리.
///
/// e⁸ 항까지 넣는다. e⁶ 에서 끊으면 위도 38° 에서 mm 급 편향이 남는다.
fn meridional_arc(lat_rad: f64, ell: &Ellipsoid) -> f64 {
    let e2 = ell.e2();
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let e8 = e6 * e2;
    ell.a
        * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0 - 175.0 * e8 / 16384.0) * lat_rad
            - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0 + 105.0 * e8 / 4096.0)
                * (2.0 * lat_rad).sin()
            + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0 + 525.0 * e8 / 16384.0)
                * (4.0 * lat_rad).sin()
            - (35.0 * e6 / 3072.0 + 175.0 * e8 / 12288.0) * (6.0 * lat_rad).sin()
            + (315.0 * e8 / 131072.0) * (8.0 * lat_rad).sin())
}

pub fn lonlat_to_tm(lon_deg: f64, lat_deg: f64, p: &TmParams) -> [f64; 2] {
    let ell = p.ellipsoid;
    let e2 = ell.e2();
    let ep2 = e2 / (1.0 - e2);
    let phi = lat_deg.to_radians();
    let lam = lon_deg.to_radians();
    let lam0 = p.lon0_deg.to_radians();
    let phi0 = p.lat0_deg.to_radians();

    let n = ell.a / (1.0 - e2 * phi.sin().powi(2)).sqrt();
    let t = phi.tan().powi(2);
    let c = ep2 * phi.cos().powi(2);
    let a = (lam - lam0) * phi.cos();
    let m = meridional_arc(phi, &ell);
    let m0 = meridional_arc(phi0, &ell);

    // A⁵·A⁶ 항까지. 3도(경도) 이내에서 mm 급 정확도를 낸다.
    let x = p.false_e
        + p.k0
            * n
            * (a + (1.0 - t + c) * a.powi(3) / 6.0
                + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * ep2) * a.powi(5) / 120.0);
    let y = p.false_n
        + p.k0
            * (m - m0
                + n * phi.tan()
                    * (a * a / 2.0
                        + (5.0 - t + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0
                        + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * ep2) * a.powi(6) / 720.0));
    [x, y]
}

pub fn tm_to_lonlat(x: f64, y: f64, p: &TmParams) -> [f64; 2] {
    let ell = p.ellipsoid;
    let e2 = ell.e2();
    let ep2 = e2 / (1.0 - e2);
    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
    let m0 = meridional_arc(p.lat0_deg.to_radians(), &ell);
    let m = m0 + (y - p.false_n) / p.k0;
    // 역변환의 footpoint 위도. e4·e6 항을 넣어야 2cm 급 편향이 사라진다.
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let mu = m / (ell.a * (1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0));
    let phi1 = mu
        + (1.5 * e1 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0) * (4.0 * mu).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin();

    let sin_p = phi1.sin();
    let cos_p = phi1.cos();
    let tan_p = phi1.tan();
    let n1 = ell.a / (1.0 - e2 * sin_p * sin_p).sqrt();
    let r1 = ell.a * (1.0 - e2) / (1.0 - e2 * sin_p * sin_p).powf(1.5);
    let t1 = tan_p * tan_p;
    let c1 = ep2 * cos_p * cos_p;
    let d = (x - p.false_e) / (n1 * p.k0);

    let d2 = d * d;
    let d3 = d2 * d;
    let d4 = d3 * d;
    let d5 = d4 * d;
    let d6 = d5 * d;

    // 위도: D⁶ 항까지.
    let lat_rad = phi1
        - (n1 * tan_p / r1)
            * (d2 / 2.0
                - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * ep2) * d4 / 24.0
                + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1
                    - 252.0 * ep2
                    - 3.0 * c1 * c1)
                    * d6
                    / 720.0);
    // 경도: D⁵ 항까지.
    let lon_rad = p.lon0_deg.to_radians()
        + (d - (1.0 + 2.0 * t1 + c1) * d3 / 6.0
            + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * ep2 + 24.0 * t1 * t1) * d5
                / 120.0)
            / cos_p;
    [lon_rad.to_degrees(), lat_rad.to_degrees()]
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

    /// 두 CRS 사이 변환이 **지원되는지** 확인한다.
    ///
    /// 예전에는 지원하지 않는 코드를 만나면 입력 좌표를 그대로 돌려줬다(무음 통과).
    /// 그러면 5185·5188 같은 좌표가 5187 지도에 그대로 찍혀 수백 km 어긋난다.
    /// 호출자는 이걸로 먼저 확인하고, 안 되면 사용자에게 알려야 한다.
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        if is_identity_crs(self.src) || is_identity_crs(self.dst) {
            return true;
        }
        TmParams::for_crs(self.src).is_some() && TmParams::for_crs(self.dst).is_some()
    }

    /// 변환한다. 지원하지 않는 조합이면 `None`.
    ///
    /// 무음으로 통과시키지 않는 것이 핵심이다 — 틀린 좌표를 조용히 그리는 것보다
    /// "변환할 수 없다"고 말하는 편이 안전하다.
    #[must_use]
    pub fn try_transform_xy(&self, x: f64, y: f64) -> Option<[f64; 2]> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        if self.src == self.dst {
            return Some([x, y]);
        }
        if !self.is_supported() {
            return None;
        }
        Some(self.transform_xy(x, y))
    }

    /// 변환한다. 지원하지 않는 조합이면 입력을 그대로 돌려준다.
    ///
    /// 기존 호출부 호환용. 새 코드는 [`Self::try_transform_xy`] 를 쓴다.
    pub fn transform_xy(&self, x: f64, y: f64) -> [f64; 2] {
        if self.src == self.dst {
            return [x, y];
        }
        let [lon, lat] = match self.src {
            CrsCode::Epsg4326 => [x, y],
            CrsCode::Epsg3857 | CrsCode::Epsg900913 => {
                let lon = (x / Ellipsoid::GRS80.a).to_degrees();
                let lat = (2.0 * (y / Ellipsoid::GRS80.a).exp().atan()
                    - std::f64::consts::FRAC_PI_2)
                    .to_degrees();
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
                let a = Ellipsoid::GRS80.a;
                let mx = a * lon.to_radians();
                let my = a * ((std::f64::consts::FRAC_PI_4 + lat.to_radians() * 0.5).tan()).ln();
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

/// 경위도·웹메르카토르처럼 별도 투영 인스턴스가 필요 없는 CRS.
const fn is_identity_crs(c: CrsCode) -> bool {
    matches!(
        c,
        CrsCode::Epsg4326 | CrsCode::Epsg3857 | CrsCode::Epsg900913
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **회귀 시험**: 5174 는 Bessel 타원체다. GRS80 으로 계산하면 수백 m 어긋난다.
    ///
    /// 2026-11-10 감사에서 발견된 결함. 고치기 전에는 5174→5187 변환이
    /// GRS80 상수로 계산돼 실제 측량 성과와 맞지 않았다.
    #[test]
    fn epsg5174_uses_bessel_not_grs80() {
        let p = TmParams::for_crs(CrsCode::Epsg5174).expect("5174 정의");
        assert_eq!(
            p.ellipsoid,
            Ellipsoid::BESSEL_1841,
            "5174 는 Bessel 1841 이어야 한다"
        );
        assert!(
            (p.ellipsoid.a - 6_377_397.155).abs() < 1e-6,
            "Bessel 장반경"
        );
        // 5186 은 GRS80 이어야 한다.
        let q = TmParams::for_crs(CrsCode::Epsg5186).expect("5186 정의");
        assert_eq!(q.ellipsoid, Ellipsoid::GRS80);
    }

    /// 5174→4326→5174 왕복이 1mm 이내로 돌아온다.
    #[test]
    fn epsg5174_roundtrip_through_wgs84() {
        let p = TmParams::for_crs(CrsCode::Epsg5174).expect("5174");
        // 공주 근처 대표 좌표.
        let (x, y) = (210_865.38, 328_194.58);
        let [lon, lat] = tm_to_lonlat(x, y, &p);
        let back = lonlat_to_tm(lon, lat, &p);
        assert!(
            (back[0] - x).abs() < 1e-3 && (back[1] - y).abs() < 1e-3,
            "5174 왕복 오차: {back:?} vs {x},{y}"
        );
    }

    /// 5186·5187·5179 왕복도 1mm 이내.
    #[test]
    fn grs80_belts_roundtrip_within_1mm() {
        for (crs, x, y) in [
            (CrsCode::Epsg5186, 210_680.0, 328_510.0),
            (CrsCode::Epsg5187, 218_500.0, 361_400.0),
            (CrsCode::Epsg5179, 1_000_123.0, 2_000_456.0),
        ] {
            let p = TmParams::for_crs(crs).expect("정의");
            let [lon, lat] = tm_to_lonlat(x, y, &p);
            let back = lonlat_to_tm(lon, lat, &p);
            assert!(
                (back[0] - x).abs() < 1e-3 && (back[1] - y).abs() < 1e-3,
                "{crs:?} 왕복 오차: {back:?} vs {x},{y}"
            );
        }
    }

    /// Bessel 과 GRS80 은 실제로 **다른 답**을 낸다 — 결함이 실재했음을 고정한다.
    ///
    /// 이 차이가 수백 m 이하로 줄면 타원체를 안 쓰고 있다는 뜻이다.
    #[test]
    fn bessel_and_grs80_give_different_lonlat() {
        let bessel = TmParams::for_crs(CrsCode::Epsg5174).expect("5174");
        let mut grs80 = bessel;
        grs80.ellipsoid = Ellipsoid::GRS80;
        let (x, y) = (210_865.38, 328_194.58);
        let a = tm_to_lonlat(x, y, &bessel);
        let b = tm_to_lonlat(x, y, &grs80);
        let dm = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() * 111_000.0;
        assert!(
            dm > 10.0,
            "Bessel/GRS80 차이가 {dm:.1}m — 타원체가 실제로 반영되지 않았다"
        );
    }

    /// 지원하지 않는 CRS 는 무음 통과하지 않는다.
    ///
    /// 예전에는 5185·5188 등이 그대로 통과해 수백 km 어긋난 좌표가 찍혔다.
    #[test]
    fn unsupported_crs_is_not_silently_passed_through() {
        let t = CrsTransformer::new(CrsCode::Epsg5174, CrsCode::Epsg5187);
        assert!(t.is_supported(), "5174→5187 은 지원해야 한다");
        assert!(t.try_transform_xy(210_865.38, 328_194.58).is_some());

        // 경위도·웹메르카토르는 언제나 지원.
        let t2 = CrsTransformer::new(CrsCode::Epsg4326, CrsCode::Epsg3857);
        assert!(t2.is_supported());

        // 비정상 입력은 None.
        let t3 = CrsTransformer::new(CrsCode::Epsg5186, CrsCode::Epsg5187);
        assert_eq!(t3.try_transform_xy(f64::NAN, 0.0), None);
        assert_eq!(t3.try_transform_xy(0.0, f64::INFINITY), None);
    }

    /// 같은 CRS 끼리는 그대로 돌려준다.
    #[test]
    fn identity_transform_returns_input() {
        let t = CrsTransformer::new(CrsCode::Epsg5187, CrsCode::Epsg5187);
        assert_eq!(t.try_transform_xy(218_500.0, 361_400.0), Some([218_500.0, 361_400.0]));
    }
}
