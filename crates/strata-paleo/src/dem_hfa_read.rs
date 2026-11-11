//! 국토정보플랫폼 공개DEM(`.img`, ERDAS Imagine HFA)을 읽는다.
//!
//! C++ 은 GDAL 이 읽는다. 여기서는 공개DEM 이 쓰는 모양만 읽는다(2026-11-11 실제 파일로 확인):
//! 밴드 하나, Float32, 압축 없는 128×128 블록, 좌표는 EPSG:5179(m), 90 m 간격, 자료 없음 -9999.
//! 그 밖의 모양(압축 블록, 다른 자료형)은 읽지 않고 알린다.
//!
//! 파일 구조: `EHFA_HEADER_TAG` → 파일 머리 → 노드 나무. `Eimg_Layer`(크기·자료형·블록 크기)
//! 아래의 `RasterDMS`(블록 위치 표)와 `Map_Info`(왼쪽 위 칸의 가운데, 칸 크기)를 쓴다.

use std::path::Path;

use crate::dem_tiff_read::DemRaster;

/// 자료형 번호: Float32.
const PIXEL_F32: u16 = 9;
/// 한 변의 최대 칸 수(잘못된 파일로 메모리를 크게 잡지 않게).
const MAX_SIDE: usize = 20_000;

struct Node {
    next: usize,
    child: usize,
    data: usize,
    name: String,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}
fn f64_at(b: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// 노드 하나(`Ehfa_Entry`): next, prev, parent, child, data, dataSize, name[64], type[32].
fn node_at(b: &[u8], at: usize) -> Option<Node> {
    let name = b.get(at + 24..at + 88)?;
    let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
    Some(Node {
        next: u32_at(b, at)?,
        child: u32_at(b, at + 12)?,
        data: u32_at(b, at + 16)?,
        name: String::from_utf8_lossy(&name[..end]).into_owned(),
    })
}

/// `first` 부터 형제들 가운데 이름이 `name` 인 노드.
fn sibling_named(b: &[u8], first: usize, name: &str) -> Option<Node> {
    let mut at = first;
    for _ in 0..10_000 {
        if at == 0 {
            return None;
        }
        let node = node_at(b, at)?;
        if node.name == name {
            return Some(node);
        }
        at = node.next;
    }
    None
}

/// `.img` 를 읽는다. 좌표는 EPSG:5179(m).
pub fn read_hfa_dem(path: &Path) -> Result<DemRaster, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("DEM 파일을 열지 못했습니다: {e}"))?;
    parse_hfa_dem(&bytes).ok_or_else(|| "이 DEM 파일(.img)의 모양은 아직 읽지 못합니다.".to_owned())
}

fn parse_hfa_dem(b: &[u8]) -> Option<DemRaster> {
    if !b.starts_with(b"EHFA_HEADER_TAG\0") {
        return None;
    }
    let file = u32_at(b, 16)?;
    let root = node_at(b, u32_at(b, file + 8)?)?;
    // 첫 밴드: 뿌리의 자식 가운데 `RasterDMS` 를 가진 노드.
    let mut at = root.child;
    let (layer, blocks) = loop {
        if at == 0 {
            return None;
        }
        let node = node_at(b, at)?;
        if let Some(dms) = sibling_named(b, node.child, "RasterDMS") {
            break (node, dms);
        }
        at = node.next;
    };
    let (width, height) = (u32_at(b, layer.data)?, u32_at(b, layer.data + 4)?);
    let pixel_type = u16_at(b, layer.data + 10)?;
    let (block_w, block_h) = (u32_at(b, layer.data + 12)?, u32_at(b, layer.data + 16)?);
    if pixel_type != PIXEL_F32 || width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE || block_w == 0 || block_h == 0 {
        return None;
    }
    // 위치: 왼쪽 위 칸의 가운데와 칸 크기. 이름(글자) → 왼쪽 위 → 오른쪽 아래 → 칸 크기 순서.
    let map = sibling_named(b, layer.child, "Map_Info")?;
    let name_len = u32_at(b, map.data)?;
    let upper_left = map.data + 8 + name_len;
    let (center_x, center_y) = (f64_at(b, upper_left + 8)?, f64_at(b, upper_left + 16)?);
    let size = upper_left + 24 + 24;
    let (dx, dy) = (f64_at(b, size + 8)?, f64_at(b, size + 16)?);
    if !(dx > 0.0 && dy > 0.0 && center_x.is_finite() && center_y.is_finite()) {
        return None;
    }
    // 블록 표: 개수(4) 칸수(4) 다음번호(4) 압축(2) | 표 개수(4) 위치(4) | 항목 14바이트씩.
    let count = u32_at(b, blocks.data)?;
    let (across, down) = (width.div_ceil(block_w), height.div_ceil(block_h));
    if count != across * down {
        return None;
    }
    let mut values = vec![f32::NAN; width * height];
    for index in 0..count {
        let item = blocks.data + 22 + index * 14;
        let (offset, size) = (u32_at(b, item + 2)?, u32_at(b, item + 6)?);
        let (valid, compressed) = (u16_at(b, item + 10)?, u16_at(b, item + 12)?);
        if valid == 0 {
            continue;
        }
        if compressed != 0 || size < block_w * block_h * 4 {
            return None;
        }
        let block = b.get(offset..offset + block_w * block_h * 4)?;
        let (col0, row0) = ((index % across) * block_w, (index / across) * block_h);
        for row in 0..block_h.min(height - row0) {
            for col in 0..block_w.min(width - col0) {
                let i = (row * block_w + col) * 4;
                let v = f32::from_le_bytes([block[i], block[i + 1], block[i + 2], block[i + 3]]);
                // 자료 없음(-9999)은 비운다.
                if v.is_finite() && v > -1000.0 {
                    values[(row0 + row) * width + col0 + col] = v;
                }
            }
        }
    }
    Some(DemRaster {
        west: center_x - dx * 0.5,
        north: center_y + dy * 0.5,
        dx,
        dy,
        width,
        height,
        values,
        projected: true,
    })
}

/// 같은 격자의 도엽 여러 장을 한 장으로 붙인다(겹치면 값이 있는 쪽). 격자 간격이 다르면 `None`.
#[must_use]
pub fn mosaic_dems(sheets: &[DemRaster]) -> Option<DemRaster> {
    let first = sheets.first()?;
    let (dx, dy) = (first.dx, first.dy);
    if sheets.iter().any(|s| (s.dx - dx).abs() > 1e-9 || (s.dy - dy).abs() > 1e-9 || s.projected != first.projected) {
        return None;
    }
    let west = sheets.iter().map(|s| s.west).fold(f64::INFINITY, f64::min);
    let north = sheets.iter().map(|s| s.north).fold(f64::NEG_INFINITY, f64::max);
    let east = sheets.iter().map(|s| s.west + dx * s.width as f64).fold(f64::NEG_INFINITY, f64::max);
    let south = sheets.iter().map(|s| s.north - dy * s.height as f64).fold(f64::INFINITY, f64::min);
    let (width, height) = (((east - west) / dx).round() as usize, ((north - south) / dy).round() as usize);
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let mut values = vec![f32::NAN; width * height];
    for sheet in sheets {
        let (col0, row0) = (((sheet.west - west) / dx).round() as usize, ((north - sheet.north) / dy).round() as usize);
        for row in 0..sheet.height.min(height - row0) {
            for col in 0..sheet.width.min(width - col0) {
                let v = sheet.values[row * sheet.width + col];
                if v.is_finite() {
                    values[(row0 + row) * width + col0 + col] = v;
                }
            }
        }
    }
    Some(DemRaster { west, north, dx, dy, width, height, values, projected: first.projected })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제로 받은 공개DEM 으로 확인한다(값은 GDAL 로 읽은 것과 맞춘다). 파일이 없으면 건너뛴다.
    #[test]
    fn a_real_ngii_open_dem_sheet_is_read_like_gdal_reads_it() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/probe/dem_survey/DEM/원본/(B080)공개DEM_36705_img_2025/36705/36705.img");
        if !path.is_file() {
            eprintln!("받은 공개DEM 이 없어 건너뜁니다: {}", path.display());
            return;
        }
        let dem = read_hfa_dem(&path).expect("읽기");
        assert_eq!((dem.width, dem.height, dem.projected), (256, 315, true));
        assert_eq!((dem.west, dem.north, dem.dx, dem.dy), (955_080.0, 1_861_380.0, 90.0, 90.0));
        assert!((dem.values[0] - 51.795_338).abs() < 1e-4, "{}", dem.values[0]);
        let mid = (dem.height / 2) * dem.width + dem.width / 2;
        assert!((dem.values[mid] - 96.548_546).abs() < 1e-4, "{}", dem.values[mid]);
        let (lo, hi) = dem.min_max().expect("표고");
        assert!((lo - 10.862_904).abs() < 1e-4 && (hi - 680.275_63).abs() < 1e-3, "{lo} {hi}");
    }

    #[test]
    fn other_files_are_refused_and_sheets_on_one_grid_are_joined() {
        assert!(parse_hfa_dem(b"II*\0 not an img").is_none());
        assert!(parse_hfa_dem(b"EHFA_HEADER_TAG\0\xff\xff\xff\x7f").is_none());
        let sheet = |west: f64, north: f64, v: f32| DemRaster { west, north, dx: 90.0, dy: 90.0, width: 2, height: 2, values: vec![v; 4], projected: true };
        // 동쪽으로 한 칸 겹쳐 놓은 두 장: 3 × 2 칸이 되고 겹친 칸은 뒤의 값.
        let joined = mosaic_dems(&[sheet(0.0, 180.0, 1.0), sheet(90.0, 180.0, 2.0)]).expect("붙이기");
        assert_eq!((joined.width, joined.height), (3, 2));
        assert_eq!(joined.values, vec![1.0, 2.0, 2.0, 1.0, 2.0, 2.0]);
        // 간격이 다르면 붙이지 않는다.
        let mut other = sheet(0.0, 180.0, 1.0);
        other.dx = 30.0;
        assert!(mosaic_dems(&[sheet(0.0, 180.0, 1.0), other]).is_none());
    }
}
