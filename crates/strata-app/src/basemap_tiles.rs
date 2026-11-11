//! VWorld 타일 URL 규칙 — C++ `BasemapOps.cpp:1161-1175` 1:1.
//!
//! 위성은 WMTS(`/req/wmts/1.0.0/<키>/Satellite/{z}/{y}/{x}.jpeg`), 일반도는
//! 같은 형식의 `Base/{z}/{y}/{x}.png` 다. 키가 없으면 xdworld 공개 타일로
//! 떨어진다(키 없이도 위성 영상이 나온다 — C++ 과 같은 폴백).


/// 배경지도 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    /// VWorld 위성 영상.
    Satellite,
    /// VWorld 일반도(백지도).
    Base,
    /// 연속지적도 (WMS — 별도 처리).
    Cadastral,
    /// 지형맵(OpenTopoMap). C++ `LayerOps::addElevationHillshadeMap` 의 XYZ 주소와 같다.
    Terrain,
}

/// 지형맵 타일이 있는 가장 큰 확대 단계(C++ URI 의 `zmax=17`).
pub const TERRAIN_MAX_ZOOM: u8 = 17;

impl TileKind {
    #[must_use]
    pub const fn is_xyz(self) -> bool {
        matches!(self, Self::Satellite | Self::Base | Self::Terrain)
    }
}

/// XYZ 타일 URL 을 만든다. 키가 비어 있으면 xdworld 공개 경로를 쓴다.
#[must_use]
pub fn tile_url(kind: TileKind, key: &str, z: u8, x: u32, y: u32) -> String {
    let key = key.trim();
    match kind {
        TileKind::Satellite => {
            if key.is_empty() {
                format!("https://xdworld.vworld.kr/2d/Satellite/service/{z}/{x}/{y}.jpeg")
            } else {
                // VWorld WMTS 는 {z}/{y}/{x} 순서다(주의).
                format!("https://api.vworld.kr/req/wmts/1.0.0/{key}/Satellite/{z}/{y}/{x}.jpeg")
            }
        }
        TileKind::Base => {
            if key.is_empty() {
                format!("https://xdworld.vworld.kr/2d/Base/service/{z}/{x}/{y}.png")
            } else {
                format!("https://api.vworld.kr/req/wmts/1.0.0/{key}/Base/{z}/{y}/{x}.png")
            }
        }
        TileKind::Cadastral => String::new(),
        // 키가 필요 없는 공개 타일. 표준 XYZ({z}/{x}/{y}) 순서다.
        TileKind::Terrain => format!("https://tile.opentopomap.org/{z}/{x}/{y}.png"),
    }
}

/// 위경도 → XYZ 타일 좌표 (Web Mercator, EPSG:3857).
#[must_use]
pub fn lon_lat_to_tile(lon: f64, lat: f64, z: u8) -> (u32, u32) {
    let n = f64::from(2_u32.pow(u32::from(z)));
    let x = ((lon + 180.0) / 360.0 * n).floor();
    let lat_r = lat.to_radians();
    let y = ((1.0 - lat_r.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor();
    let max = n as i64 - 1;
    (
        x.clamp(0.0, max as f64) as u32,
        y.clamp(0.0, max as f64) as u32,
    )
}

/// 타일 좌표 → 그 타일의 북서 모서리(경위도).
///
/// 역변환은 `lat = atan(sinh(π(1 - 2y/n)))` 이다. (atan·tan 조합을 쓰면
/// 60도대 값이 나와 지도가 엉뚱한 곳을 가리킨다 — 시험이 잡았다.)
#[must_use]
pub fn tile_lon_lat(z: u8, x: u32, y: u32) -> (f64, f64) {
    let n = f64::from(2_u32.pow(u32::from(z)));
    let lon = f64::from(x) / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / n))
        .sinh()
        .atan()
        .to_degrees();
    (lon, lat)
}

/// 화면 1px 당 미터에서 XYZ 줌 레벨을 고른다.
///
/// 3857 타일 한 장은 256px 이고, 줌 z 에서 적도 기준 지구 둘레를
/// 256 * 2^z 로 나눈 값이 1px 당 미터다. 한국 위도(약 36도)에서는
/// cos(lat) 만큼 줄어들므로 그 보정을 넣는다. C++ QGIS 의 자동 줌과
/// 같은 감각이 되도록 0..=19 로 자른다.
#[must_use]
pub fn basemap_zoom_for(meters_per_px: f64, _rect: egui::Rect) -> u8 {
    if !meters_per_px.is_finite() || meters_per_px <= 0.0 {
        return 14;
    }
    const EARTH_CIRCUM: f64 = 40_075_016.686;
    const KOREA_LAT: f64 = 36.0;
    let lat_scale = KOREA_LAT.to_radians().cos();
    let effective = meters_per_px * lat_scale;
    let mut best = 6_u8;
    let mut best_err = f64::INFINITY;
    for z in 5..=19_u8 {
        let mpp = EARTH_CIRCUM / (256.0 * f64::from(1_u32 << u32::from(z)));
        let err = (mpp - effective).abs();
        if err < best_err {
            best_err = err;
            best = z;
        }
    }
    best
}
