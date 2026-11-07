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

fn meridional_arc(lat_rad: f64) -> f64 {
    let e2 = 2.0 * GRS80_F - GRS80_F * GRS80_F;
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    GRS80_A
        * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * lat_rad
            - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * lat_rad).sin()
            + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * lat_rad).sin())
}

pub fn lonlat_to_tm(lon_deg: f64, lat_deg: f64, p: &TmParams) -> [f64; 2] {
    let e2 = 2.0 * GRS80_F - GRS80_F * GRS80_F;
    let ep2 = e2 / (1.0 - e2);
    let phi = lat_deg.to_radians();
    let lam = lon_deg.to_radians();
    let lam0 = p.lon0_deg.to_radians();
    let phi0 = p.lat0_deg.to_radians();

    let n = GRS80_A / (1.0 - e2 * phi.sin().powi(2)).sqrt();
    let t = phi.tan().powi(2);
    let c = ep2 * phi.cos().powi(2);
    let a = (lam - lam0) * phi.cos();
    let m = meridional_arc(phi);
    let m0 = meridional_arc(phi0);

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
                        + (5.0 - t + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0));
    [x, y]
}

pub fn tm_to_lonlat(x: f64, y: f64, p: &TmParams) -> [f64; 2] {
    let e2 = 2.0 * GRS80_F - GRS80_F * GRS80_F;
    let ep2 = e2 / (1.0 - e2);
    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
    let m0 = meridional_arc(p.lat0_deg.to_radians());
    let m = m0 + (y - p.false_n) / p.k0;
    let mu = m / (GRS80_A * (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0));
    let phi1 = mu
        + (1.5 * e1 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0) * (4.0 * mu).sin();

    let sin_p = phi1.sin();
    let cos_p = phi1.cos();
    let tan_p = phi1.tan();
    let n1 = GRS80_A / (1.0 - e2 * sin_p * sin_p).sqrt();
    let r1 = GRS80_A * (1.0 - e2) / (1.0 - e2 * sin_p * sin_p).powf(1.5);
    let t1 = tan_p * tan_p;
    let c1 = ep2 * cos_p * cos_p;
    let d = (x - p.false_e) / (n1 * p.k0);

    let lat_rad = phi1
        - (n1 * tan_p / r1)
            * (d * d / 2.0 - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * ep2) * d.powi(4) / 24.0);
    let lon_rad = p.lon0_deg.to_radians()
        + (d - (1.0 + 2.0 * t1 + c1) * d.powi(3) / 6.0) / cos_p;
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

    pub fn transform_xy(&self, x: f64, y: f64) -> [f64; 2] {
        if self.src == self.dst {
            return [x, y];
        }
        let [lon, lat] = match self.src {
            CrsCode::Epsg4326 => [x, y],
            CrsCode::Epsg3857 | CrsCode::Epsg900913 => {
                let lon = (x / GRS80_A).to_degrees();
                let lat = (2.0 * (y / GRS80_A).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
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
                let my = GRS80_A * ((std::f64::consts::FRAC_PI_4 + lat.to_radians() * 0.5).tan()).ln();
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
