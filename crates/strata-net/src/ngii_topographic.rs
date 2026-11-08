//! 국토지리정보원(국토정보플랫폼) 수치지형도 실제 요청 파이프라인.
//!
//! C++ 정본:
//! - `KaTopographicBrowser.cpp:671-727` `prepareSheets` / `startAutomation` — 5km 반경의 1:25,000 도엽 목록
//! - `data/providers/ngii-topographic.js:34-121` 실제 HTTP 두 단계(`combineBufferQuery.do` → `suchiQuery.do`)
//! - `KaTopographicBrowser.cpp:757-761,811-843` 응답 확인 실패·도엽 없음 안내문
//! - `core/TopographicSheets.cpp` 도엽 번호 격자 계산(`numberForCell` / `resolve`)
use serde_json::Value;
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub struct NgiiTopographicClient;

impl NgiiTopographicClient {
    pub const PORTAL_URL: &'static str = "https://map.ngii.go.kr/ms/map/NlipMap.do";
    /// `KaTopographicBrowser.cpp:49` `kHome` — 처음 여는 공식 화면
    pub const HOME_URL: &'static str = "https://map.ngii.go.kr/ms/map/NlipMap.do?tabGb=total";
    pub const PORTAL_ORIGIN: &'static str = "https://map.ngii.go.kr";
    pub const CONTOUR_LAYER_CODES: [&'static str; 4] = ["F0010000", "1000", "1011", "1012"];

    /// `MainWindowDownloads.cpp:187` — 조사 주변 5 km
    pub const SCOPE_RADIUS_M: f64 = 5000.0;
    /// `KaTopographicBrowser.cpp:675` — 자동 선택 도엽 상한
    pub const MAX_SHEETS: usize = 100;

    pub const fn portal_url() -> &'static str {
        Self::PORTAL_URL
    }

    pub const fn home_url() -> &'static str {
        Self::HOME_URL
    }

    /// `ngii-topographic.js:43` 도엽 후보 범위 질의(공개, 로그인 불필요)
    #[must_use]
    pub fn combine_buffer_query_url() -> String {
        format!("{}/ms/map/combineBufferQuery.do", Self::PORTAL_ORIGIN)
    }

    /// `ngii-topographic.js:63` 1:25,000 DXF 도엽 목록 질의(공개, 로그인 불필요)
    #[must_use]
    pub fn suchi_query_url() -> String {
        format!("{}/ms/map/suchiQuery.do", Self::PORTAL_ORIGIN)
    }

    /// `ngii-topographic.js:43` `{x, y, buffer}` — `x`/`y` 는 EPSG:5179 미터
    #[must_use]
    pub fn combine_buffer_body(x: f64, y: f64, buffer_m: f64) -> String {
        format!("x={}&y={}&buffer={}", fmt_num(x), fmt_num(y), fmt_num(buffer_m))
    }

    /// `ngii-topographic.js:63-64` `{index, type, scale, wkt, minx, miny, maxx, maxy}`
    #[must_use]
    pub fn suchi_body(area: &SheetScope) -> String {
        format!(
            "index=&type=0&scale=25000&wkt={}&minx={}&miny={}&maxx={}&maxy={}",
            pct(&area.wkt),
            fmt_num(area.minx),
            fmt_num(area.miny),
            fmt_num(area.maxx),
            fmt_num(area.maxy)
        )
    }

    /// `ngii-topographic.js:47-50` — 범위 응답 검증. 하나의 `POLYGON((…))` 만 받는다.
    pub fn parse_combine_buffer(body: &[u8], x: f64, y: f64, buffer_m: f64) -> Result<SheetScope, String> {
        let root: Value = serde_json::from_slice(body).map_err(|_| Self::scope_error().to_owned())?;
        let items = root
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| Self::scope_error().to_owned())?;
        if items.len() != 1 {
            return Err(Self::scope_error().to_owned());
        }
        let row = &items[0];
        let wkt = row
            .get("wkt")
            .and_then(Value::as_str)
            .ok_or_else(|| Self::scope_error().to_owned())?;
        let upper = wkt.trim_start().to_ascii_uppercase();
        if !upper.starts_with("POLYGON") || !upper.contains("((") || wkt.len() > 100_000 {
            return Err(Self::scope_error().to_owned());
        }
        let num = |key: &str| row.get(key).and_then(Value::as_f64);
        let (minx, miny, maxx, maxy) = (
            num("minx").ok_or_else(|| Self::scope_error().to_owned())?,
            num("miny").ok_or_else(|| Self::scope_error().to_owned())?,
            num("maxx").ok_or_else(|| Self::scope_error().to_owned())?,
            num("maxy").ok_or_else(|| Self::scope_error().to_owned())?,
        );
        if ![minx, miny, maxx, maxy].iter().all(|v| v.is_finite())
            || (minx - (x - buffer_m)).abs() > 1.0
            || (maxx - (x + buffer_m)).abs() > 1.0
            || (miny - (y - buffer_m)).abs() > 1.0
            || (maxy - (y + buffer_m)).abs() > 1.0
        {
            return Err(Self::scope_error().to_owned());
        }
        Ok(SheetScope {
            wkt: wkt.to_owned(),
            minx,
            miny,
            maxx,
            maxy,
        })
    }

    /// `ngii-topographic.js:96-116` — `suchiQuery` 응답 검증. 하나라도 규격이 다르면 전체를 거부한다.
    pub fn parse_suchi(body: &[u8], area: &SheetScope, wanted: &[String]) -> Result<SheetQuery, String> {
        let root: Value = serde_json::from_slice(body).map_err(|_| Self::scope_error().to_owned())?;
        let items = root
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| Self::scope_error().to_owned())?;
        if items.len() > Self::MAX_SHEETS {
            return Err(Self::scope_error().to_owned());
        }
        let mut products: Vec<SheetProduct> = Vec::with_capacity(items.len());
        for row in items {
            let num = row
                .get("num")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .ok_or_else(|| Self::scope_error().to_owned())?;
            let finite = |key: &str| row.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
            let (minx, miny, maxx, maxy) = (
                finite("minx").ok_or_else(|| Self::scope_error().to_owned())?,
                finite("miny").ok_or_else(|| Self::scope_error().to_owned())?,
                finite("maxx").ok_or_else(|| Self::scope_error().to_owned())?,
                finite("maxy").ok_or_else(|| Self::scope_error().to_owned())?,
            );
            let scale = row
                .get("scale")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .unwrap_or_default();
            let file_ext = row
                .get("fileExt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            let name = row.get("name").and_then(Value::as_str).unwrap_or_default();
            let projection = row.get("projection").and_then(Value::as_str).unwrap_or_default();
            let valid = num.len() == 6
                && num.bytes().all(|b| b.is_ascii_digit())
                && !products.iter().any(|p| p.num == num)
                && scale == "25000"
                && file_ext == "dxf"
                && !name.is_empty()
                && row.get("historyNo").is_some_and(|v| !v.is_null())
                && !projection.is_empty()
                && minx < maxx
                && miny < maxy
                && maxx >= area.minx
                && minx <= area.maxx
                && maxy >= area.miny
                && miny <= area.maxy;
            if !valid {
                return Err(Self::scope_error().to_owned());
            }
            products.push(SheetProduct {
                num,
                name: name.to_owned(),
                scale,
                file_ext,
                projection: projection.to_owned(),
                minx,
                miny,
                maxx,
                maxy,
            });
        }
        // `ngii-topographic.js:109` — 요청했는데 공식 목록에 없는 도엽
        let unavailable = wanted
            .iter()
            .filter(|n| !products.iter().any(|p| &p.num == *n))
            .cloned()
            .collect();
        Ok(SheetQuery {
            products,
            unavailable,
        })
    }

    /// `ngii-topographic.js:120` / `KaTopographicBrowser.cpp:843`
    #[must_use]
    pub const fn scope_error() -> &'static str {
        "도엽·축척·파일 형식 또는 포털 구조를 확인하지 못했습니다. 필요한 도엽을 직접 선택해 주세요."
    }

    /// `ngii-topographic.js:114` — 이 범위에 제공 중인 도엽이 없음
    #[must_use]
    pub const fn unavailable_message() -> &'static str {
        "이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."
    }

    /// `KaTopographicBrowser.cpp:851` — 필요한 지형도를 이미 모두 보관 중
    #[must_use]
    pub const fn cached_message() -> &'static str {
        "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다."
    }

    /// `KaTopographicBrowser.cpp:758-759` — 공식 사이트 응답을 확인하지 못했을 때
    #[must_use]
    pub const fn portal_timeout_message() -> &'static str {
        "공식 사이트의 응답을 확인하지 못했습니다. 계정 설정과 연결 상태를 확인하세요."
    }

    /// `KaTopographicBrowser.cpp:582` — 페이지 자체를 불러오지 못했을 때
    #[must_use]
    pub const fn page_load_failed_message() -> &'static str {
        "공식 사이트 페이지를 불러오지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요."
    }

    /// `KaTopographicBrowser.cpp:152` — 창의 안내문
    #[must_use]
    pub const fn browser_hint() -> &'static str {
        "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요."
    }

    /// `KaTopographicBrowser.cpp:523` — 공식 화면이 추가 확인을 요구할 때
    #[must_use]
    pub const fn needs_input_message() -> &'static str {
        "자동 받기에 추가 확인이 필요합니다. 로그인 설정과 연결 상태를 확인해 주세요."
    }

    /// `KaTopographicBrowser.cpp:614` — 자동 전송을 계속할 수 없을 때(외부 전송 프로그램 필요)
    #[must_use]
    pub const fn transfer_program_message() -> &'static str {
        "자동 전송을 계속할 수 없습니다. 전송 프로그램 연결이 필요합니다. 지도 작업은 계속할 수 있습니다."
    }

    /// C++ 은 로그인·신청·파일 준비·다운로드를 내장 브라우저(QWebEngine)의 공식 화면에서 한다
    /// (`KaTopographicBrowser.cpp:509-598`). Rust 에는 그 브라우저가 없으므로, C++ 이 같은
    /// 상황에 내는 안내(152·446·214-215)를 그대로 보여 주고 「받은 자료 불러오기」로 넘긴다.
    #[must_use]
    pub const fn login_stage_blocked_message() -> &'static str {
        "로그인·신청·다운로드는 국토정보플랫폼 공식 화면에서 진행합니다. 공식 화면에서 받은 DXF·ZIP 폴더는 「받은 자료 불러오기…」로 지도에 올릴 수 있습니다."
    }

    /// **C++ 에 대응이 없는 옛 계산이다.** 기존 시험이 이 값을 고정하고 있어 지우지 않는다.
    /// 실제 앱 경로는 아래 `sheets_for_bbox_25k` / `TopographicSheets` 를 쓴다.
    pub fn sheet_index_5k_for_tm(easting: f64, northing: f64) -> String {
        let lat_band = ((northing / 10_000.0).floor() as i32).clamp(33, 39);
        let lon_band = ((easting / 10_000.0).floor() as i32).clamp(125, 131);
        format!("{lat_band}{lon_band}01")
    }

    /// **C++ 에 대응이 없는 옛 계산이다.** 기존 시험이 이 값을 고정하고 있어 지우지 않는다.
    /// 실제 앱 경로는 아래 `sheets_for_bbox_25k` 를 쓴다.
    #[must_use]
    pub fn bbox_to_sheets_5k(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<String> {
        let mut codes = Vec::new();
        let x0 = (min_x / 2_500.0).floor() as i32;
        let x1 = (max_x / 2_500.0).floor() as i32;
        let y0 = (min_y / 2_500.0).floor() as i32;
        let y1 = (max_y / 2_500.0).floor() as i32;
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let lat_idx = (360 + (gy - 170).clamp(-30, 30)).clamp(330, 389);
                let sub_idx = (84 + (gx - 4).clamp(-80, 15)).clamp(1, 100);
                let code = format!("{lat_idx}{sub_idx:03}");
                if !codes.contains(&code) {
                    codes.push(code);
                }
            }
        }
        codes
    }

    /// 조사구역 BBOX(경위도)와 교차하는 1:25,000 도엽 번호 —
    /// C++ `TopographicSheets::select`(TopographicSheets.cpp:110-138)와 같은 격자 규칙.
    #[must_use]
    pub fn sheets_for_bbox_25k(box_wgs84: [f64; 4]) -> Vec<String> {
        TopographicSheets::numbers_in_geographic_box(box_wgs84)
    }
}

/// 지리 좌표 범위(경도/위도)
#[derive(Debug, Clone, PartialEq)]
pub struct SheetScope {
    pub wkt: String,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// 공식 `suchiQuery.do` 가 돌려준 1:25,000 DXF 도엽 한 장
#[derive(Debug, Clone, PartialEq)]
pub struct SheetProduct {
    pub num: String,
    pub name: String,
    pub scale: String,
    pub file_ext: String,
    pub projection: String,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// 도엽 질의 결과 — 받을 수 있는 도엽과, 요청했지만 공식 목록에 없는 도엽
#[derive(Debug, Clone, PartialEq)]
pub struct SheetQuery {
    pub products: Vec<SheetProduct>,
    pub unavailable: Vec<String>,
}

impl SheetQuery {
    /// `KaTopographicBrowser.cpp:868-871` — 신청할 도엽 목록을 8장까지 보여 주고 나머지는 "외 n장"
    #[must_use]
    pub fn order_sheets_text(&self) -> String {
        if self.products.is_empty() {
            return String::new();
        }
        let total = self.products.len();
        let mut listed: Vec<String> = self
            .products
            .iter()
            .take(8)
            .map(|p| format!("{} {}", p.num, p.name).trim().to_owned())
            .collect();
        if total > 8 {
            listed.push(format!("외 {}장", total - 8));
        }
        format!(
            "공식 신청할 도엽 {}장 · {} — 중지하려면 「취소」를 누르세요.",
            total,
            listed.join(", ")
        )
    }
}

/// C++ `TopographicSheets` — 1:25,000 도엽 번호 격자(경위도 7분 30초 = 0.125°)
#[derive(Debug, Clone, Copy)]
pub struct TopographicSheets;

impl TopographicSheets {
    /// C++ `TopographicSheets.cpp:10` `kGridSize` — 공식 7분 30초
    pub const GRID_SIZE: f64 = 0.125;
    /// C++ `TopographicSheets.cpp:11-12`
    pub const WEST: f64 = 124.0;
    pub const EAST: f64 = 132.0;

    /// C++ `TopographicSheets.cpp:24-31` `numberForCell`
    #[must_use]
    pub fn number_for_cell(x: i32, y: i32) -> String {
        let base_longitude = x.div_euclid(8);
        let base_latitude = y.div_euclid(8);
        let north_row = 7 - y.rem_euclid(8);
        let map50k = (north_row / 2) * 4 + x.rem_euclid(8) / 2 + 1;
        let quarter = (north_row % 2) * 2 + x.rem_euclid(2) + 1;
        format!(
            "{:02}{}{:02}{}",
            base_latitude,
            base_longitude % 10,
            map50k,
            quarter
        )
    }

    /// C++ `TopographicSheets.cpp:34-60` `resolve` — 6자리 숫자만 받는다.
    #[must_use]
    pub fn resolve(number: &str) -> Option<TopographicSheet> {
        if number.len() != 6 || !number.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let base_latitude: i32 = number[0..2].parse().ok()?;
        let longitude_digit: i32 = number[2..3].parse().ok()?;
        let map50k: i32 = number[3..5].parse().ok()?;
        let quarter: i32 = number[5..6].parse().ok()?;
        if base_latitude >= 90
            || longitude_digit == 2
            || longitude_digit == 3
            || !(1..=16).contains(&map50k)
            || !(1..=4).contains(&quarter)
        {
            return None;
        }
        let base_longitude: i32 = (if longitude_digit >= 4 { 120 } else { 130 }) + longitude_digit;
        let index50k = map50k - 1;
        let index25k = quarter - 1;
        let west = f64::from(base_longitude)
            + f64::from(index50k % 4) * 0.25
            + f64::from(index25k % 2) * Self::GRID_SIZE;
        let north = f64::from(base_latitude) + 1.0
            - f64::from(index50k / 4) * 0.25
            - f64::from(index25k / 2) * Self::GRID_SIZE;
        let center_longitude = west + Self::GRID_SIZE / 2.0;
        let zone = if center_longitude < 126.0 {
            TopographicZone::West
        } else if center_longitude < 128.0 {
            TopographicZone::Central
        } else if center_longitude < 130.0 {
            TopographicZone::East
        } else {
            TopographicZone::EastSea
        };
        Some(TopographicSheet {
            number: number.to_owned(),
            west,
            south: north - Self::GRID_SIZE,
            east: west + Self::GRID_SIZE,
            north,
            zone,
        })
    }

    /// 지리 좌표 범위가 걸치는 도엽 번호(정렬·중복 제거) — C++ `select` 의 후보 격자 훑기와 같은 규칙.
    #[must_use]
    pub fn numbers_in_geographic_box(box_wgs84: [f64; 4]) -> Vec<String> {
        let min_x = box_wgs84[0].min(box_wgs84[2]);
        let max_x = box_wgs84[0].max(box_wgs84[2]);
        let min_y = box_wgs84[1].min(box_wgs84[3]);
        let max_y = box_wgs84[1].max(box_wgs84[3]);
        if !(min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite()) {
            return Vec::new();
        }
        // C++ `TopographicSheets.cpp:113-117`
        let lo_x = ((min_x.clamp(Self::WEST, Self::EAST)) * 8.0).floor() as i32;
        let hi_x = 1055.min(((max_x.clamp(Self::WEST, Self::EAST)) * 8.0).floor() as i32);
        let lo_y = ((min_y.clamp(0.0, 90.0)) * 8.0).floor() as i32;
        let hi_y = 719.min(((max_y.clamp(0.0, 90.0)) * 8.0).floor() as i32);
        let mut out = Vec::new();
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                let number = Self::number_for_cell(x, y);
                if Self::resolve(&number).is_some() && !out.contains(&number) {
                    out.push(number);
                }
            }
        }
        out.sort();
        out
    }
}

/// 도엽이 속한 원점대(C++ `TopographicSheets::Zone`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopographicZone {
    West,
    Central,
    East,
    EastSea,
}

impl TopographicZone {
    /// C++ `TopographicSheets.cpp:62-71` `zoneName`
    #[must_use]
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::West => "서부원점",
            Self::Central => "중부원점",
            Self::East => "동부원점",
            Self::EastSea => "동해원점",
        }
    }

    /// C++ `TopographicSheets.cpp:53-57` — 5185/5186/5187/5188
    #[must_use]
    pub const fn epsg(self) -> u32 {
        match self {
            Self::West => 5185,
            Self::Central => 5186,
            Self::East => 5187,
            Self::EastSea => 5188,
        }
    }
}

/// 한 장의 1:25,000 도엽 — 지리(경위도) 도곽
#[derive(Debug, Clone, PartialEq)]
pub struct TopographicSheet {
    pub number: String,
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
    pub zone: TopographicZone,
}

impl TopographicSheet {
    #[must_use]
    pub const fn extent(&self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }

    /// C++ `TopographicSheets.cpp:58` — `World2010` 근거가 확립된 경우에만 채운다.
    #[must_use]
    pub fn crs_auth_id(&self, world_2010: bool) -> Option<String> {
        world_2010.then(|| format!("EPSG:{}", self.zone.epsg()))
    }
}

/// JS `String(x)` 와 같은 짧은 숫자 표기
fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// 폼 본문 값 퍼센트 인코딩(C++ `QUrlQuery::FullyEncoded` + `+`→`%2B`)
fn pct(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// 공식 포털의 두 POST 를 실제로 보내 1:25,000 DXF 도엽 목록을 받는다.
///
/// C++ `ngii-topographic.js:43-64` 와 같은 순서·같은 본문이다. `x`/`y` 는 EPSG:5179 미터,
/// `buffer_m` 은 조사 반경(`MainWindowDownloads.cpp:187` = 5000).
pub fn fetch_official_sheets_25k(
    x: f64,
    y: f64,
    buffer_m: f64,
    wanted: &[String],
) -> Result<SheetQuery, String> {
    let scope_body = NgiiTopographicClient::combine_buffer_body(x, y, buffer_m);
    let scope_bytes = http_post_form(&NgiiTopographicClient::combine_buffer_query_url(), &scope_body)?;
    let area = NgiiTopographicClient::parse_combine_buffer(&scope_bytes, x, y, buffer_m)?;
    let suchi_bytes =
        http_post_form(&NgiiTopographicClient::suchi_query_url(), &NgiiTopographicClient::suchi_body(&area))?;
    NgiiTopographicClient::parse_suchi(&suchi_bytes, &area, wanted)
}

/// curl 로 `application/x-www-form-urlencoded` POST (C++ 은 브라우저 fetch 로 같은 본문을 보낸다)
fn http_post_form(url: &str, body: &str) -> Result<Vec<u8>, String> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };
    let mut cmd = Command::new(curl_bin);
    cmd.args([
        "-sS",
        "--max-time",
        "30",
        "-A",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3",
        "-H",
        "Content-Type: application/x-www-form-urlencoded; charset=UTF-8",
        "-H",
        "Referer: https://map.ngii.go.kr/ms/map/NlipMap.do",
        "-X",
        "POST",
        "--data-binary",
        body,
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd
        .output()
        .map_err(|_| NgiiTopographicClient::portal_timeout_message().to_owned())?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(NgiiTopographicClient::portal_timeout_message().to_owned());
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ngii_bbox_to_sheets_gongju_gongsanseong_returns_deduplicated_367084_367085() {
        // 옛 계산은 C++ 대응이 없어 그대로 남긴다(기존 시험 고정값).
        let sheets =
            NgiiTopographicClient::bbox_to_sheets_5k(10_000.0, 442_500.0, 12_600.0, 443_000.0);
        assert_eq!(sheets, vec!["367084".to_owned(), "367085".to_owned()]);
    }

    /// C++ `TopographicSheets::select` 경로 — 경위도 BBOX 로 1:25,000 도엽 번호.
    #[test]
    fn test_ngii_sheets_for_bbox_25k_returns_sorted_unique_official_numbers() {
        // 공주 공산성 일대(127.10~127.16E, 36.44~36.48N)는 36도 37분30초~36도 30분 대의
        // 1:25,000 두 장에 걸친다(C++ `numberForCell` 로 계산한 값).
        let sheets = NgiiTopographicClient::sheets_for_bbox_25k([127.10, 36.44, 127.16, 36.48]);
        assert_eq!(
            sheets,
            vec!["367091".to_owned(), "367092".to_owned()],
            "got {sheets:?}"
        );
        // 각 번호는 C++ `resolve` 가 받아들이는 6자리 격자 번호이고 도곽이 7분 30초다.
        for number in &sheets {
            let sheet = TopographicSheets::resolve(number).expect(number);
            assert!((sheet.east - sheet.west - 7.5 / 60.0).abs() < 1e-12, "{number}");
        }
    }

    /// C++ `test_topographic_sheets.cpp:26-37` `officialExamplesAndExactGrid`
    #[test]
    fn test_cpp_topographic_sheets_resolve_matches_official_examples() {
        let example = TopographicSheets::resolve("367154").expect("367154");
        assert_eq!(example.extent(), [127.625, 36.0, 127.75, 36.125]);
        assert_eq!(example.zone, TopographicZone::Central);
        assert_eq!(example.crs_auth_id(false), None);
        assert!((example.extent()[2] - example.extent()[0] - 7.5 / 60.0).abs() < 1e-12);

        let gangneung = TopographicSheets::resolve("378044").expect("378044");
        assert_eq!(gangneung.extent(), [128.875, 37.75, 129.0, 37.875]);
        assert_eq!(gangneung.zone, TopographicZone::East);
        assert_eq!(gangneung.crs_auth_id(true).as_deref(), Some("EPSG:5187"));
    }

    /// C++ `test_topographic_sheets.cpp:38-43` `rejectsMalformedAndUnsupportedLongitudeCodes`
    #[test]
    fn test_cpp_topographic_sheets_rejects_malformed_numbers() {
        for bad in [
            "36715", "3671544", "367004", "367174", "367150", "367155", "36715A", " 367154",
            "363154", "362154",
        ] {
            assert!(
                TopographicSheets::resolve(bad).is_none(),
                "{bad} 는 거부되어야 한다"
            );
        }
        assert!(TopographicSheets::resolve("３６７１５４").is_none());
    }

    /// C++ `test_topographic_sheets.cpp:56-62` — 원점대 4개
    #[test]
    fn test_cpp_topographic_sheets_zone_codes_5185_to_5188() {
        for (number, zone, epsg) in [
            ("364011", TopographicZone::West, 5185_u32),
            ("366011", TopographicZone::Central, 5186),
            ("368011", TopographicZone::East, 5187),
            ("370011", TopographicZone::EastSea, 5188),
        ] {
            let sheet = TopographicSheets::resolve(number).expect(number);
            assert_eq!(sheet.zone, zone, "{number}");
            assert_eq!(sheet.zone.epsg(), epsg, "{number}");
            assert!(!sheet.zone.name_ko().is_empty());
        }
    }

    /// C++ `test_topographic_sheets.cpp:79-89` — 격자 경계 4장이 함께 나온다
    #[test]
    fn test_cpp_topographic_sheets_boundary_includes_four_cells() {
        // 경도 128.0(격자 x=1023/1024), 위도 36.875(격자 y=294/295) 네 칸 경계
        let ids = TopographicSheets::numbers_in_geographic_box([127.999, 36.874, 128.001, 36.876]);
        assert_eq!(ids.len(), 4, "got {ids:?}");
        for want in ["367042", "367044", "368011", "368013"] {
            assert!(ids.contains(&want.to_owned()), "{want} missing in {ids:?}");
        }
    }

    #[test]
    fn test_cpp_topographic_sheets_number_for_cell_round_trips() {
        for y in 288..=288 {
            for x in 1020..=1027 {
                let number = TopographicSheets::number_for_cell(x, y);
                assert_eq!(number.len(), 6, "{number}");
                let sheet = TopographicSheets::resolve(&number).expect(&number);
                let west_idx = (sheet.west * 8.0).floor() as i32;
                assert_eq!(west_idx, x, "{number} 서쪽 격자");
            }
        }
    }

    #[test]
    fn test_ngii_combine_buffer_and_suchi_bodies_match_cpp_provider() {
        assert_eq!(
            NgiiTopographicClient::combine_buffer_query_url(),
            "https://map.ngii.go.kr/ms/map/combineBufferQuery.do"
        );
        assert_eq!(
            NgiiTopographicClient::suchi_query_url(),
            "https://map.ngii.go.kr/ms/map/suchiQuery.do"
        );
        // `ngii-topographic.js:43` {x,y,buffer}
        assert_eq!(
            NgiiTopographicClient::combine_buffer_body(200_000.0, 500_000.0, 5000.0),
            "x=200000&y=500000&buffer=5000"
        );
        let area = SheetScope {
            wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
            minx: 195_000.0,
            miny: 495_000.0,
            maxx: 205_000.0,
            maxy: 505_000.0,
        };
        // `ngii-topographic.js:63-64` — index/type/scale 순서와 scale=25000
        assert_eq!(
            NgiiTopographicClient::suchi_body(&area),
            "index=&type=0&scale=25000&wkt=POLYGON%28%281%202%2C3%204%2C5%206%2C1%202%29%29&minx=195000&miny=495000&maxx=205000&maxy=505000"
        );
    }

    #[test]
    fn test_ngii_parse_combine_buffer_accepts_scope_and_rejects_mismatched_box() {
        let ok = br#"{"items":[{"wkt":"POLYGON((195000 495000,205000 495000,205000 505000,195000 505000,195000 495000))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
        let area = NgiiTopographicClient::parse_combine_buffer(ok, 200_000.0, 500_000.0, 5000.0)
            .expect("scope");
        assert_eq!(area.maxx, 205_000.0);
        // 범위가 x±radius 와 1m 넘게 다르면 C++ 은 'scope' 오류로 멈춘다.
        let bad = br#"{"items":[{"wkt":"POLYGON((1 2,3 4,5 6,1 2))","minx":190000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
        assert_eq!(
            NgiiTopographicClient::parse_combine_buffer(bad, 200_000.0, 500_000.0, 5000.0)
                .expect_err("1m 넘는 범위 차이는 거부"),
            NgiiTopographicClient::scope_error()
        );
        // 여러 개를 돌려주면 거부
        let many = br#"{"items":[{"wkt":"POLYGON((1 2))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000},{"wkt":"POLYGON((1 2))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
        assert!(
            NgiiTopographicClient::parse_combine_buffer(many, 200_000.0, 500_000.0, 5000.0)
                .is_err()
        );
    }

    #[test]
    fn test_ngii_parse_suchi_accepts_dxf_25k_and_reports_unavailable() {
        let area = SheetScope {
            wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
            minx: 195_000.0,
            miny: 495_000.0,
            maxx: 205_000.0,
            maxy: 505_000.0,
        };
        let body = br#"{"items":[
            {"num":"367154","name":"gongju","scale":"25000","fileExt":"DXF","historyNo":"1234","projection":"EPSG:5186",
             "minx":198000,"miny":498000,"maxx":202000,"maxy":502000},
            {"num":"367155","name":"gongju2","scale":"25000","fileExt":"dxf","historyNo":"1235","projection":"EPSG:5186",
             "minx":198000,"miny":498000,"maxx":202000,"maxy":502000}]}"#;
        let wanted = vec!["367154".to_owned(), "367999".to_owned()];
        let query = NgiiTopographicClient::parse_suchi(body, &area, &wanted).expect("suchi");
        assert_eq!(query.products.len(), 2);
        assert_eq!(query.products[0].num, "367154");
        assert_eq!(query.unavailable, vec!["367999".to_owned()]);
        // C++ `KaTopographicBrowser.cpp:870` 신청 목록 문구
        let text = query.order_sheets_text();
        assert!(text.starts_with("공식 신청할 도엽 2장 · 367154 gongju, 367155 gongju2"));
        assert!(text.ends_with("중지하려면 「취소」를 누르세요."));
    }

    #[test]
    fn test_ngii_parse_suchi_rejects_wrong_scale_or_extension() {
        let area = SheetScope {
            wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
            minx: 195_000.0,
            miny: 495_000.0,
            maxx: 205_000.0,
            maxy: 505_000.0,
        };
        for bad_row in [
            r#"{"num":"367154","name":"공주","scale":"50000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
            r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"shp","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
            r#"{"num":"36715","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
            r#"{"num":"367154","name":"","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
            r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":202000,"miny":498000,"maxx":198000,"maxy":502000}"#,
            r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":900000,"miny":498000,"maxx":902000,"maxy":502000}"#,
        ] {
            let body = format!("{{\"items\":[{bad_row}]}}");
            assert_eq!(
                NgiiTopographicClient::parse_suchi(body.as_bytes(), &area, &[])
                    .expect_err("C++ 규격이 아니면 거부"),
                NgiiTopographicClient::scope_error(),
                "{bad_row}"
            );
        }
    }

    #[test]
    fn test_ngii_stage_messages_match_cpp_strings() {
        assert_eq!(
            NgiiTopographicClient::portal_url(),
            "https://map.ngii.go.kr/ms/map/NlipMap.do"
        );
        assert!(NgiiTopographicClient::home_url().ends_with("tabGb=total"));
        assert_eq!(
            NgiiTopographicClient::browser_hint(),
            "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요."
        );
        assert_eq!(
            NgiiTopographicClient::unavailable_message(),
            "이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."
        );
        assert_eq!(
            NgiiTopographicClient::cached_message(),
            "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다."
        );
        assert_eq!(
            NgiiTopographicClient::transfer_program_message(),
            "자동 전송을 계속할 수 없습니다. 전송 프로그램 연결이 필요합니다. 지도 작업은 계속할 수 있습니다."
        );
        assert!(NgiiTopographicClient::login_stage_blocked_message().contains("받은 자료 불러오기"));
    }
}
