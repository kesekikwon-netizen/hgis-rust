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
    /// 대동여지도 — NGII old-map WMS asked for by XYZ tile square (`BasemapOps.cpp:1547-1563`).
    Daedongyeojido,
    /// 지형맵 — OpenTopoMap XYZ tiles (`BasemapOps.cpp:1583-1586`).
    Terrain,
    /// 1919 조선지형도 1:5만 — WMTS of the history-GIS server, in its own grid (`BasemapOps.cpp:1486-1511`).
    History1919,
}

/// How a tile source cuts the ground into levels, columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileGrid {
    /// The XYZ scheme: the EPSG:3857 world square, 2^z by 2^z tiles.
    WebMercator,
    /// The grid of the history-GIS server (its WMTS capabilities, read 2026-11-15): EPSG:5179, north-west
    /// corner x -200 000 m, y 4 000 000 m, 256-pixel tiles, level 0 one tile, each level twice as fine
    /// (levels 0 to 16).
    Korea5179,
}

impl TileKind {
    /// Kinds that are fetched tile by tile (level, column, row).
    #[must_use]
    pub const fn is_xyz(self) -> bool {
        matches!(self, Self::Satellite | Self::Base | Self::Daedongyeojido | Self::Terrain | Self::History1919)
    }

    #[must_use]
    pub const fn grid(self) -> TileGrid {
        match self {
            Self::History1919 => TileGrid::Korea5179,
            _ => TileGrid::WebMercator,
        }
    }
}

/// The grid `Korea5179`: its north-west corner (x, y), the side of a tile of level 0 (256 pixels of
/// 55 845 639.682136305 * 0.28 mm) and its finest level.
const KOREA_CORNER: [f64; 2] = [-200_000.0, 4_000_000.0];
const KOREA_TILE_0: f64 = 256.0 * 55_845_639.682_136_305 * 0.000_28;
const KOREA_FINEST: u8 = 16;
const KOREA_TM: strata_geo::crs_transform::TmParams = match strata_geo::crs_transform::TmParams::for_crs(strata_types::crs::CrsCode::Epsg5179) {
    Some(params) => params,
    None => panic!("EPSG:5179 is a known TM system"),
};

fn korea_tile_side(z: u8) -> f64 {
    KOREA_TILE_0 / f64::from(1_u32 << u32::from(z.min(KOREA_FINEST)))
}

/// A point of a grid (level, column and row with fractions) as longitude and latitude.
#[must_use]
pub fn grid_point_lon_lat(grid: TileGrid, z: u8, fx: f64, fy: f64) -> (f64, f64) {
    match grid {
        TileGrid::WebMercator => tile_point_lon_lat(z, fx, fy),
        TileGrid::Korea5179 => {
            let side = korea_tile_side(z);
            let [lon, lat] = strata_geo::crs_transform::tm_to_lonlat(KOREA_CORNER[0] + fx * side, KOREA_CORNER[1] - fy * side, &KOREA_TM);
            (lon, lat)
        }
    }
}

/// The tile of a grid that holds a longitude and latitude at a level.
#[must_use]
pub fn grid_tile_of(grid: TileGrid, lon: f64, lat: f64, z: u8) -> (u32, u32) {
    match grid {
        TileGrid::WebMercator => lon_lat_to_tile(lon, lat, z),
        TileGrid::Korea5179 => {
            let (side, [x, y]) = (korea_tile_side(z), strata_geo::crs_transform::lonlat_to_tm(lon, lat, &KOREA_TM));
            let last = f64::from((1_u32 << u32::from(z.min(KOREA_FINEST))) - 1);
            // A whole number between 0 and the last column or row of the level.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let place = |along: f64| if along.is_finite() { (along / side).floor().clamp(0.0, last) as u32 } else { 0 };
            (place(x - KOREA_CORNER[0]), place(KOREA_CORNER[1] - y))
        }
    }
}

/// The level of a grid whose pixel is nearest to the map's (metres per pixel).
#[must_use]
pub fn level_for(grid: TileGrid, meters_per_px: f64, rect: egui::Rect) -> u8 {
    match grid {
        TileGrid::WebMercator => basemap_zoom_for(meters_per_px, rect),
        TileGrid::Korea5179 if !meters_per_px.is_finite() || meters_per_px <= 0.0 => 11,
        TileGrid::Korea5179 => (0..=KOREA_FINEST)
            .min_by(|a, b| (korea_tile_side(*a) / 256.0 - meters_per_px).abs().total_cmp(&(korea_tile_side(*b) / 256.0 - meters_per_px).abs()))
            .unwrap_or(KOREA_FINEST),
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
        TileKind::Daedongyeojido => {
            // `BasemapOps.cpp:1547-1563`: WMS 1.1.1 (1.3.0 answered empty), PNG, transparent, no key. The
            // square asked for is the tile's own: the XYZ scheme cuts the EPSG:3857 world square, of half
            // side π·a (a = 6 378 137 m), into 2^z by 2^z tiles with row 0 in the north.
            let half = std::f64::consts::PI * 6_378_137.0;
            let span = 2.0 * half / f64::from(1_u32 << u32::from(z));
            let min_x = -half + f64::from(x) * span;
            let max_y = half - f64::from(y) * span;
            format!(
                "https://map.ngii.go.kr/spcemapserver/korea_old_map/ows?SERVICE=WMS&VERSION=1.1.1&REQUEST=GetMap\
                 &LAYERS=korea_oldmap_ddymap_kyu&STYLES=&SRS=EPSG:3857&BBOX={:.3},{:.3},{:.3},{:.3}\
                 &WIDTH=256&HEIGHT=256&FORMAT=image/png&TRANSPARENT=TRUE",
                min_x,
                max_y - span,
                min_x + span,
                max_y
            )
        }
        // `BasemapOps.cpp:1585`: the first of the two addresses; zmin 5 and zmax 17 are kept by the layer.
        TileKind::Terrain => format!("https://tile.opentopomap.org/{z}/{x}/{y}.png"),
        // `BasemapOps.cpp:1486-1511`: the server's WMTS, asked for tile by tile (KVP). No key, no request.
        TileKind::History1919 if key.is_empty() => String::new(),
        TileKind::History1919 => format!(
            "https://hgis.history.go.kr/openapi/get.do?Service=WMTS&Request=GetTile&Version=1.0.0&Layer=history:map1919&Style=\
             &Format=image/png&TileMatrixSet=EPSG:5179&TileMatrix=EPSG:5179:{z}&TileRow={y}&TileCol={x}&apiKey={key}"
        ),
        TileKind::Cadastral => String::new(),
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
/// 시험에서만 쓴다 — 앱은 메시 꼭짓점을 `tile_point_lon_lat` 으로 구한다.
#[cfg(test)]
#[must_use]
pub fn tile_lon_lat(z: u8, x: u32, y: u32) -> (f64, f64) {
    tile_point_lon_lat(z, f64::from(x), f64::from(y))
}

/// 전역 타일 좌표(소수 허용) → 경위도. 정수면 그 타일의 북서 모서리다.
/// 타일을 메시로 펴서 그릴 때 타일 안의 점을 구하는 데 쓴다.
#[must_use]
pub fn tile_point_lon_lat(z: u8, fx: f64, fy: f64) -> (f64, f64) {
    let n = f64::from(2_u32.pow(u32::from(z)));
    let lon = fx / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * fy / n))
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
