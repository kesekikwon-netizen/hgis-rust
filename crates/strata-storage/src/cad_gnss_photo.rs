//! GNSS(NMEA GGA) 품질 판정과 사진 촬영 원뿔 계산.
//!
//! `cad_csv_io` 에서 분리했다.

pub const HGIS_RTK_MAX_ALLOWED_HDOP: f64 = 1.5;

pub const HGIS_DEFAULT_PHOTO_FOV_DEG: f64 = 60.0;

pub const HGIS_DEFAULT_PHOTO_CONE_LEN_M: f64 = 15.0;

pub const HGIS_NMEA_MAX_SENTENCE_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GnssQuality {
    Invalid = 0,
    GpsFix = 1,
    DgpsFix = 2,
    PpsFix = 3,
    RtkFixed = 4,
    RtkFloat = 5,
    Estimated = 6,
    Manual = 7,
    Simulation = 8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GnssPositionRecord {
    pub is_valid: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub quality: GnssQuality,
    pub satellites_used: u32,
    pub hdop: f64,
}

/// C++ `KaGnssNmeaParser::parseGgaSentence` — `$GPGGA` / `$GNGGA` XOR 체크섬(`*XX`) 검증 및 `DDMM.MMMM` 도분 → 십진도(`deg + min / 60.0`) 변환
pub fn parse_gga_sentence(nmea_sentence: &str) -> Result<GnssPositionRecord, &'static str> {
    let s = nmea_sentence.trim();
    if s.len() > HGIS_NMEA_MAX_SENTENCE_LEN
        || (!s.starts_with("$GPGGA,") && !s.starts_with("$GNGGA,"))
    {
        return Err("MalformedSentence");
    }
    let Some(star_idx) = s.rfind('*') else {
        return Err("MalformedSentence");
    };
    if star_idx + 3 > s.len() {
        return Err("MalformedSentence");
    }
    let expected_cs =
        u8::from_str_radix(&s[star_idx + 1..star_idx + 3], 16).map_err(|_| "MalformedSentence")?;
    let actual_cs = s[1..star_idx].bytes().fold(0_u8, |acc, b| acc ^ b);
    if actual_cs != expected_cs {
        return Err("ChecksumMismatch");
    }
    let fields: Vec<&str> = s[1..star_idx].split(',').collect();
    if fields.len() < 10 {
        return Err("MalformedSentence");
    }
    let q_code = fields[6].parse::<u8>().unwrap_or(0);
    let quality = match q_code {
        1 => GnssQuality::GpsFix,
        2 => GnssQuality::DgpsFix,
        3 => GnssQuality::PpsFix,
        4 => GnssQuality::RtkFixed,
        5 => GnssQuality::RtkFloat,
        6 => GnssQuality::Estimated,
        7 => GnssQuality::Manual,
        8 => GnssQuality::Simulation,
        _ => GnssQuality::Invalid,
    };
    if quality == GnssQuality::Invalid || fields[2].is_empty() || fields[4].is_empty() {
        return Ok(GnssPositionRecord {
            is_valid: false,
            latitude: 0.0,
            longitude: 0.0,
            altitude_m: 0.0,
            quality: GnssQuality::Invalid,
            satellites_used: 0,
            hdop: 99.9,
        });
    }
    let parse_dm = |raw: &str, deg_len: usize, hemi: &str| -> Result<f64, &'static str> {
        if raw.len() <= deg_len {
            return Err("MalformedSentence");
        }
        let deg = raw[..deg_len]
            .parse::<f64>()
            .map_err(|_| "MalformedSentence")?;
        let min = raw[deg_len..]
            .parse::<f64>()
            .map_err(|_| "MalformedSentence")?;
        let mut dd = deg + min / 60.0;
        if hemi.eq_ignore_ascii_case("S") || hemi.eq_ignore_ascii_case("W") {
            dd = -dd;
        }
        Ok(dd)
    };
    let latitude = parse_dm(fields[2], 2, fields[3])?;
    let longitude = parse_dm(fields[4], 3, fields[5])?;
    let satellites_used = fields[7].parse::<u32>().unwrap_or(0);
    let hdop = fields[8].parse::<f64>().unwrap_or(99.9);
    let altitude_m = fields[9].parse::<f64>().unwrap_or(0.0);
    Ok(GnssPositionRecord {
        is_valid: true,
        latitude,
        longitude,
        altitude_m,
        quality,
        satellites_used,
        hdop,
    })
}

/// C++ `KaRtkPositionTracker::filterRtkQuality` — RTK Fixed(4) / Float(5) 및 `hdop <= maxAllowedHdop` 필터링
#[must_use]
pub fn filter_rtk_quality(rec: &GnssPositionRecord, max_allowed_hdop: f64) -> bool {
    rec.is_valid
        && matches!(rec.quality, GnssQuality::RtkFixed | GnssQuality::RtkFloat)
        && rec.hdop <= max_allowed_hdop
}

/// C++ `KaPhotoExifImporter` — EXIF Rational 도·분·초 `(num, den)` 3쌍을 십진도(`deg + min/60 + sec/3600`)로 변환
#[must_use]
pub fn exif_rational_dms_to_decimal(
    deg: (u32, u32),
    min: (u32, u32),
    sec: (u32, u32),
    hemisphere: char,
) -> Option<f64> {
    if deg.1 == 0 || min.1 == 0 || sec.1 == 0 {
        return None;
    }
    let d = f64::from(deg.0) / f64::from(deg.1);
    let m = f64::from(min.0) / f64::from(min.1);
    let s = f64::from(sec.0) / f64::from(sec.1);
    let mut dd = d + m / 60.0 + s / 3600.0;
    if hemisphere == 'S' || hemisphere == 's' || hemisphere == 'W' || hemisphere == 'w' {
        dd = -dd;
    }
    Some(dd)
}

/// C++ `KaPhotoMarkerLayer::buildViewConeGeometry` — 촬영 원점 `camera_pos`, 방위각 `direction_deg` (`0°`=북, `90°`=동), 시야각 `fov_deg`, 가시거리 `length_m`로부터 폐합 부채꼴 정점 링 생성
#[must_use]
pub fn build_photo_view_cone_ring(
    camera_pos: [f64; 2],
    direction_deg: f64,
    fov_deg: f64,
    length_m: f64,
    arc_steps: usize,
) -> Vec<[f64; 2]> {
    let steps = arc_steps.max(2);
    let r = if length_m > 0.0 && length_m.is_finite() {
        length_m
    } else {
        HGIS_DEFAULT_PHOTO_CONE_LEN_M
    };
    let fov = if fov_deg > 0.0 && fov_deg.is_finite() {
        fov_deg
    } else {
        HGIS_DEFAULT_PHOTO_FOV_DEG
    };
    let start_az = direction_deg - fov * 0.5;
    let mut ring = Vec::with_capacity(steps + 3);
    ring.push(camera_pos);
    for i in 0..=steps {
        let az_deg = start_az + fov * (i as f64 / steps as f64);
        let az_rad = az_deg.to_radians();
        let vx = camera_pos[0] + r * az_rad.sin();
        let vy = camera_pos[1] + r * az_rad.cos();
        ring.push([vx, vy]);
    }
    ring.push(camera_pos);
    ring
}
