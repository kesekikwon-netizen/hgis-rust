//! 시추 도면 Float32 GeoTIFF 기록 — C++ `PaleoAnalysisRunner::writeFloatGeoTiff`
//! (`HgisPaleoTerrain.cpp:213-239`) 가 GDAL 로 만들던 단일 밴드 `Float32` GeoTIFF 를
//! 순수 Rust `tiff` 크레이트로 같은 지구참조 태그 구성으로 기록한다.
//!
//! C++ 는 `GDALDriver::Create(..., 1, GDT_Float32)` + `SetGeoTransform` +
//! `SetProjection(WKT)` + `SetNoDataValue(-9999.0)` 을 쓴다. 여기서는
//! GeoTransform 을 `ModelPixelScaleTag(33550)` + `ModelTiepointTag(33922)` 로,
//! 투영을 `GeoKeyDirectoryTag(34735)` 의 `ProjectedCSTypeGeoKey(3072)` 로 기록한다.

use std::path::Path;

use strata_types::error::{StrataError, StrataResult};
use tiff::encoder::{colortype, TiffEncoder};
use tiff::tags::Tag;

use crate::terrain_edt_tpi::{GridTransform, HGIS_NODATA_FLOAT};

/// C++ `band->SetNoDataValue(-9999.0)` (`HgisPaleoTerrain.cpp:236`).
pub const BOREHOLE_TIFF_NODATA: f64 = HGIS_NODATA_FLOAT;

/// C++ `QgsCoordinateReferenceSystem::toWkt()` 대신 기록하는 EPSG 코드.
/// C++ `GeoInfoBoreholeMaps.cpp:199-200` 의 작업 좌표계 폴백(`EPSG:5187`)과 같은 체계다.
#[must_use]
pub fn work_crs_epsg(work_crs: strata_types::crs::WorkCrs) -> u32 {
    work_crs.epsg_code()
}

/// GeoKeyDirectoryTag 항목 — C++
/// `ProjectedCSTypeGeoKey(3072) = epsg`, `GTModelTypeGeoKey(1024) = 1 (projected)`.
///
/// C++ `GeoInfoBoreholeMaps.cpp` 는 GDAL `SetProjection` 으로 GeoKeys 를 만들지만
/// 결과 태그 내용은 이 4개 항목 헤더 + 2개 키와 같다.
#[must_use]
pub fn build_geokey_directory(epsg_code: u32) -> Vec<u16> {
    vec![
        1,    // KeyDirectoryVersion
        1,    // KeyRevision
        0,    // MinorRevision
        2,    // NumberOfKeys
        1024, // GTModelTypeGeoKey
        0,    // TIFFTagLocation (0 = 값이 이 배열 안에)
        1,    // Count
        1,    // ModelTypeProjected
        3072, // ProjectedCSTypeGeoKey
        0,
        1,
        epsg_code as u16,
    ]
}

/// `ModelPixelScaleTag` 값 — `(scaleX, scaleY, scaleZ)`.
///
/// C++ `double gt[6] = {origin_x, cell, 0, origin_y, 0, -cell}` (`HgisPaleoTerrain.cpp:229`)
/// 에서 `scaleX = cell`, `scaleY = cell`.
#[must_use]
pub fn build_pixel_scale(tf: &GridTransform) -> [f64; 3] {
    [tf.cell_size, tf.cell_size, 0.0]
}

/// `ModelTiepointTag` 값 — C++ `SetGeoTransform` 의 `(origin_x, origin_y)` 를
/// 래스터 `(0, 0, 0)` 에 묶는다.
#[must_use]
pub fn build_tiepoint(tf: &GridTransform) -> [f64; 6] {
    [0.0, 0.0, 0.0, tf.origin_x, tf.origin_y, 0.0]
}

/// 단일 밴드 Float32 GeoTIFF 를 기록한다.
///
/// C++ `writeFloatGeoTiff` 와 동일하게 격자 크기가 값 개수와 맞지 않으면 실패한다
/// (`HgisPaleoTerrain.cpp:216-219`).
pub fn write_float32_geotiff(
    out_path: &Path,
    tf: &GridTransform,
    epsg_code: u32,
    values: &[f64],
) -> StrataResult<()> {
    let expected = tf.width * tf.height;
    if tf.width == 0 || tf.height == 0 || values.len() < expected {
        return Err(StrataError::InvalidInput(format!(
            "Float32 GeoTIFF 격자 크기가 올바르지 않습니다 ({}x{}, 값 {}개)",
            tf.width,
            tf.height,
            values.len()
        )));
    }
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let width = u32::try_from(tf.width).map_err(|_| {
        StrataError::InvalidInput(format!("GeoTIFF 폭 {} 가 u32 범위를 넘습니다", tf.width))
    })?;
    let height = u32::try_from(tf.height).map_err(|_| {
        StrataError::InvalidInput(format!("GeoTIFF 높이 {} 가 u32 범위를 넘습니다", tf.height))
    })?;

    let mut samples: Vec<f32> = Vec::with_capacity(expected);
    for &v in values.iter().take(expected) {
        samples.push(v as f32);
    }

    let file = std::fs::File::create(out_path)?;
    let mut encoder = TiffEncoder::new(std::io::BufWriter::new(file))
        .map_err(|e| StrataError::Export(format!("GeoTIFF 생성 실패: {e}")))?;
    let mut image = encoder
        .new_image::<colortype::Gray32Float>(width, height)
        .map_err(|e| StrataError::Export(format!("GeoTIFF 밴드 생성 실패: {e}")))?;

    {
        let dir = image.encoder();
        dir.write_tag(Tag::ModelPixelScaleTag, &build_pixel_scale(tf)[..])
            .map_err(|e| StrataError::Export(format!("GeoTIFF 33550 기록 실패: {e}")))?;
        dir.write_tag(Tag::ModelTiepointTag, &build_tiepoint(tf)[..])
            .map_err(|e| StrataError::Export(format!("GeoTIFF 33922 기록 실패: {e}")))?;
        dir.write_tag(Tag::GeoKeyDirectoryTag, &build_geokey_directory(epsg_code)[..])
            .map_err(|e| StrataError::Export(format!("GeoTIFF 34735 기록 실패: {e}")))?;
    }

    image
        .write_data(&samples)
        .map_err(|e| StrataError::Export(format!("GeoTIFF 픽셀 기록 실패: {e}")))?;
    Ok(())
}

/// 기록된 GeoTIFF 에서 헤더 태그를 읽어 검증한다 (테스트 및 산출물 확인용).
#[derive(Debug, Clone, PartialEq)]
pub struct GeoTiffHeaderInfo {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: Vec<u16>,
    pub sample_format: Vec<u16>,
    pub samples_per_pixel: Option<u32>,
    pub pixel_scale: Vec<f64>,
    pub tiepoint: Vec<f64>,
    pub geo_keys: Vec<u16>,
}

/// GeoTIFF 를 열어 위 태그들을 읽는다. 파일이 TIFF 가 아니면 오류.
pub fn read_geotiff_header_info(path: &Path) -> StrataResult<GeoTiffHeaderInfo> {
    let bytes = std::fs::read(path)?;
    let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(bytes))
        .map_err(|e| StrataError::Export(format!("GeoTIFF 열기 실패: {e}")))?;
    let (width, height) = decoder
        .dimensions()
        .map_err(|e| StrataError::Export(format!("GeoTIFF 크기 읽기 실패: {e}")))?;
    let tag_vec_f64 = |dec: &mut tiff::decoder::Decoder<std::io::Cursor<Vec<u8>>>, tag: Tag| {
        dec.get_tag_f64_vec(tag).unwrap_or_default()
    };
    let tag_vec_u16 = |dec: &mut tiff::decoder::Decoder<std::io::Cursor<Vec<u8>>>, tag: Tag| {
        dec.get_tag_u16_vec(tag).unwrap_or_default()
    };
    let bits_per_sample = tag_vec_u16(&mut decoder, Tag::BitsPerSample);
    let sample_format = tag_vec_u16(&mut decoder, Tag::SampleFormat);
    let samples_per_pixel = decoder.get_tag_unsigned::<u32>(Tag::SamplesPerPixel).ok();
    let pixel_scale = tag_vec_f64(&mut decoder, Tag::ModelPixelScaleTag);
    let tiepoint = tag_vec_f64(&mut decoder, Tag::ModelTiepointTag);
    let geo_keys = tag_vec_u16(&mut decoder, Tag::GeoKeyDirectoryTag);
    Ok(GeoTiffHeaderInfo {
        width,
        height,
        bits_per_sample,
        sample_format,
        samples_per_pixel,
        pixel_scale,
        tiepoint,
        geo_keys,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain_edt_tpi::GridTransform;
    use strata_types::crs::WorkCrs;

    fn tf() -> GridTransform {
        GridTransform {
            origin_x: 200_000.0,
            origin_y: 400_000.0,
            cell_size: 12.5,
            width: 6,
            height: 4,
        }
    }

    #[test]
    fn written_geotiff_has_ii_magic_and_georeferencing_tags() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("시추_테스트.tif");
        let values: Vec<f64> = (0..24).map(|i| i as f64 * 0.5).collect();
        write_float32_geotiff(&path, &tf(), WorkCrs::Epsg5187.epsg_code(), &values)?;

        let bytes = std::fs::read(&path)?;
        assert_eq!(&bytes[0..2], b"II", "little-endian TIFF magic");
        assert_eq!(bytes[2], 42);
        assert_eq!(bytes[3], 0);

        let info = read_geotiff_header_info(&path)?;
        assert_eq!((info.width, info.height), (6, 4));
        // Float32 밴드: BitsPerSample 32, SampleFormat 3(IEEE float), 단일 밴드
        assert_eq!(info.bits_per_sample, vec![32]);
        assert_eq!(info.sample_format, vec![3]);
        assert_eq!(info.samples_per_pixel, Some(1));

        // Tag 33550 / 33922 / 34735 존재와 값
        assert_eq!(info.pixel_scale.len(), 3);
        assert!((info.pixel_scale[0] - 12.5).abs() < 1e-9);
        assert!((info.pixel_scale[1] - 12.5).abs() < 1e-9);
        assert_eq!(info.tiepoint.len(), 6);
        assert!((info.tiepoint[3] - 200_000.0).abs() < 1e-9);
        assert!((info.tiepoint[4] - 400_000.0).abs() < 1e-9);

        let keys = build_geokey_directory(5187);
        assert_eq!(info.geo_keys, keys);
        assert_eq!(info.geo_keys[3], 2, "NumberOfKeys");
        assert_eq!(info.geo_keys[4], 1024, "GTModelTypeGeoKey");
        assert_eq!(info.geo_keys[8], 3072, "ProjectedCSTypeGeoKey");
        assert_eq!(info.geo_keys[11], 5187);
        Ok(())
    }

    #[test]
    fn wrong_value_count_is_rejected_like_cpp() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bad.tif");
        let err = write_float32_geotiff(&path, &tf(), 5187, &[1.0, 2.0]);
        assert!(err.is_err());
        assert!(!path.exists() || std::fs::metadata(&path).map(|m| m.len()).unwrap_or(1) == 0);
        Ok(())
    }

    #[test]
    fn read_header_of_non_tiff_file_fails_cleanly() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("not_a_tiff.bin");
        std::fs::write(&path, b"not a tiff at all")?;
        assert!(read_geotiff_header_info(&path).is_err());
        Ok(())
    }

    #[test]
    fn geokey_directory_encodes_requested_epsg() {
        let keys = build_geokey_directory(5186);
        assert_eq!(keys.len(), 12);
        assert_eq!(keys[11], 5186);
        assert_eq!(work_crs_epsg(WorkCrs::Epsg5186), 5186);
        assert_eq!(work_crs_epsg(WorkCrs::Epsg5187), 5187);
    }
}
