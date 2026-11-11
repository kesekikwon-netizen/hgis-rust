//! 받은 DEM(GeoTIFF, Float32, 경위도)을 읽어 필요한 범위만 잘라 낸다.
//!
//! C++ 은 GDAL 이 `/vsicurl/` 로 필요한 부분만 읽고 QgsRasterLayer 가 그린다.
//! 여기서는 받은 Copernicus GLO-30 타일(1° × 1°, 3600 × 3600, Deflate + 부동소수 예측자)을
//! `tiff` 크레이트로 풀고, 화면 범위에 해당하는 창만 메모리에 남긴다.
//! 위치는 GeoTIFF 태그 ModelPixelScale(33550)·ModelTiepoint(33922)에서 읽는다.

use std::path::Path;

use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

/// 잘라 낸 표고 격자. 북쪽이 위, 좌표는 경위도(도).
#[derive(Debug, Clone, PartialEq)]
pub struct DemRaster {
    /// 왼쪽 위 모서리의 경도·위도.
    pub west: f64,
    pub north: f64,
    /// 한 칸의 크기(도). 둘 다 양수다.
    pub dx: f64,
    pub dy: f64,
    pub width: usize,
    pub height: usize,
    /// 줄 순서(북→남, 서→동)의 표고(m). 자료가 없는 칸은 NaN.
    pub values: Vec<f32>,
}

impl DemRaster {
    /// 덮는 범위 `[west, south, east, north]`.
    #[must_use]
    pub fn bbox(&self) -> [f64; 4] {
        [
            self.west,
            self.north - self.dy * self.height as f64,
            self.west + self.dx * self.width as f64,
            self.north,
        ]
    }

    /// 유효한 표고의 (최저, 최고).
    #[must_use]
    pub fn min_max(&self) -> Option<(f32, f32)> {
        let mut it = self.values.iter().copied().filter(|v| v.is_finite());
        let first = it.next()?;
        Some(it.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v))))
    }
}

/// GeoTIFF 를 읽어 `clip`(경위도 `[west, south, east, north]`)에 걸친 창만 돌려준다.
/// `clip` 이 없으면 전체. 범위가 파일과 겹치지 않으면 오류다.
pub fn read_dem_window(path: &Path, clip: Option<[f64; 4]>) -> Result<DemRaster, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("DEM 파일을 열지 못했습니다: {e}"))?;
    let mut decoder = Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("DEM 파일을 읽지 못했습니다: {e}"))?;
    let (w, h) = decoder.dimensions().map_err(|e| format!("DEM 크기를 읽지 못했습니다: {e}"))?;
    let (w, h) = (w as usize, h as usize);
    let scale = decoder
        .get_tag_f64_vec(Tag::Unknown(33550))
        .map_err(|_| "DEM 에 위치 정보(격자 간격)가 없습니다.".to_owned())?;
    let tie = decoder
        .get_tag_f64_vec(Tag::Unknown(33922))
        .map_err(|_| "DEM 에 위치 정보(기준점)가 없습니다.".to_owned())?;
    if scale.len() < 2 || tie.len() < 6 || scale[0] <= 0.0 || scale[1] <= 0.0 {
        return Err("DEM 의 위치 정보를 해석하지 못했습니다.".to_owned());
    }
    let (dx, dy) = (scale[0], scale[1]);
    // 기준점: 픽셀 (i, j) 가 지도 (x, y) 에 놓인다.
    let west = tie[3] - tie[0] * dx;
    let north = tie[4] + tie[1] * dy;

    let all = match decoder.read_image().map_err(|e| format!("DEM 표고를 풀지 못했습니다: {e}"))? {
        DecodingResult::F32(v) => v,
        DecodingResult::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f32::from).collect(),
        _ => return Err("DEM 표고 형식(밴드 1, Float32)이 아닙니다.".to_owned()),
    };
    if all.len() != w * h {
        return Err("DEM 표고 수가 크기와 맞지 않습니다.".to_owned());
    }

    // 창(픽셀) — 범위를 바깥쪽 칸까지 넓혀 잡는다.
    let (x0, y0, x1, y1) = match clip {
        None => (0, 0, w, h),
        Some([cw, cs, ce, cn]) => {
            let px = |lon: f64| ((lon - west) / dx).floor();
            let py = |lat: f64| ((north - lat) / dy).floor();
            let x0 = px(cw).max(0.0) as usize;
            let y0 = py(cn).max(0.0) as usize;
            let x1 = ((px(ce) + 1.0).max(0.0) as usize).min(w);
            let y1 = ((py(cs) + 1.0).max(0.0) as usize).min(h);
            if x0 >= x1 || y0 >= y1 {
                return Err("요청한 범위가 이 DEM 칸과 겹치지 않습니다.".to_owned());
            }
            (x0, y0, x1, y1)
        }
    };
    let (cw, ch) = (x1 - x0, y1 - y0);
    let mut values = Vec::with_capacity(cw * ch);
    for row in y0..y1 {
        values.extend_from_slice(&all[row * w + x0..row * w + x1]);
    }
    // Copernicus 는 자료 없음(바다 등)을 큰 음수로 적기도 한다 — NaN 으로 바꾼다.
    for v in &mut values {
        if !v.is_finite() || *v < -1000.0 {
            *v = f32::NAN;
        }
    }
    Ok(DemRaster {
        west: west + x0 as f64 * dx,
        north: north - y0 as f64 * dy,
        dx,
        dy,
        width: cw,
        height: ch,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제로 받은 Copernicus 타일로 확인한다. 파일이 없으면(다른 PC) 건너뛴다.
    /// 받는 주소: `https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N36_00_E127_00_DEM/…DEM.tif`
    #[test]
    fn a_real_copernicus_tile_is_decoded_and_cut_to_the_requested_window() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/probe/dem/N36E127.tif");
        if !path.is_file() {
            eprintln!("받은 DEM 타일이 없어 건너뜁니다: {}", path.display());
            return;
        }
        // 견본 조사 주변 약 11 km × 11 km (경도 127.0 은 타일 서쪽 끝이다).
        let clip = [127.0, 36.60, 127.12, 36.70];
        let dem = read_dem_window(&path, Some(clip)).expect("읽기");
        eprintln!("창 {}x{} dx={} dy={} 범위={:?} 표고={:?}", dem.width, dem.height, dem.dx, dem.dy, dem.bbox(), dem.min_max());
        // 1초(1/3600°) 격자: 0.12° × 0.10° ≈ 432 × 360 칸(±2).
        assert!((dem.dx - 1.0 / 3600.0).abs() < 1e-9 && (dem.dy - 1.0 / 3600.0).abs() < 1e-9);
        assert!((430..=436).contains(&dem.width) && (358..=364).contains(&dem.height), "{}x{}", dem.width, dem.height);
        let b = dem.bbox();
        assert!(b[0] <= clip[0] + 1e-9 && b[2] >= clip[2] - 1e-9 && b[1] <= clip[1] + 1e-9 && b[3] >= clip[3] - 1e-9, "{b:?}");
        // 충남 내륙: 표고는 0~900 m 사이.
        let (lo, hi) = dem.min_max().expect("표고");
        assert!(lo > -5.0 && hi < 900.0 && hi - lo > 50.0, "lo={lo} hi={hi}");
        // 범위가 타일 밖이면 알려 준다.
        assert!(read_dem_window(&path, Some([10.0, 10.0, 11.0, 11.0])).is_err());
    }
}
