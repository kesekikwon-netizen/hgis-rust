use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::OnceLock;

use serde_json::Value;
use strata_types::error::{StrataError, StrataResult};

const KOREA_DONGS_JSON: &str = include_str!("../../../data/korea_dongs.json");

#[derive(Debug, Clone, PartialEq)]
pub struct AddressSearchHit {
    pub title: String,
    pub road_or_parcel: String,
    pub lon: f64,
    pub lat: f64,
}

/// C++ `LocationHit` (`LocationSearch.h:12-20`) 1:1
#[derive(Debug, Clone, PartialEq)]
pub struct LocationHit {
    pub title: String,
    pub detail: String,
    pub lon: f64,
    pub lat: f64,
    /// `[west, south, east, north]` in EPSG:4326 when available
    pub bbox: Option<[f64; 4]>,
    /// 19-digit PNU only populated for exact parcel-address result
    pub pnu: String,
}

/// C++ `KoreaRegionBounds` (`KoreaRegionCatalog.h:9-14`) 1:1
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KoreaRegionBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// C++ `KoreaSido` (`KoreaRegionCatalog.h:16-20`) 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KoreaSido {
    pub name: &'static str,
    pub short_name: &'static str,
    pub cities: &'static [&'static str],
}

pub const BUILTIN_KOREA_SIDO: &[KoreaSido] = &[
    KoreaSido {
        name: "서울특별시",
        short_name: "서울",
        cities: &[
            "종로구",
            "중구",
            "용산구",
            "성동구",
            "광진구",
            "동대문구",
            "중랑구",
            "성북구",
            "강북구",
            "도봉구",
            "노원구",
            "은평구",
            "서대문구",
            "마포구",
            "양천구",
            "강서구",
            "구로구",
            "금천구",
            "영등포구",
            "동작구",
            "관악구",
            "서초구",
            "강남구",
            "송파구",
            "강동구",
        ],
    },
    KoreaSido {
        name: "부산광역시",
        short_name: "부산",
        cities: &[
            "중구",
            "서구",
            "동구",
            "영도구",
            "부산진구",
            "동래구",
            "남구",
            "북구",
            "해운대구",
            "사하구",
            "금정구",
            "강서구",
            "연제구",
            "수영구",
            "사상구",
            "기장군",
        ],
    },
    KoreaSido {
        name: "대구광역시",
        short_name: "대구",
        cities: &[
            "중구",
            "동구",
            "서구",
            "남구",
            "북구",
            "수성구",
            "달서구",
            "달성군",
            "군위군",
        ],
    },
    KoreaSido {
        name: "인천광역시",
        short_name: "인천",
        cities: &[
            "중구",
            "동구",
            "미추홀구",
            "연수구",
            "남동구",
            "부평구",
            "계양구",
            "서구",
            "강화군",
            "옹진군",
        ],
    },
    KoreaSido {
        name: "광주광역시",
        short_name: "광주",
        cities: &["동구", "서구", "남구", "북구", "광산구"],
    },
    KoreaSido {
        name: "대전광역시",
        short_name: "대전",
        cities: &["동구", "중구", "서구", "유성구", "대덕구"],
    },
    KoreaSido {
        name: "울산광역시",
        short_name: "울산",
        cities: &["중구", "남구", "동구", "북구", "울주군"],
    },
    KoreaSido {
        name: "세종특별자치시",
        short_name: "세종",
        cities: &["세종시"],
    },
    KoreaSido {
        name: "경기도",
        short_name: "경기",
        cities: &[
            "수원시",
            "성남시",
            "의정부시",
            "안양시",
            "부천시",
            "광명시",
            "평택시",
            "동두천시",
            "안산시",
            "고양시",
            "과천시",
            "구리시",
            "남양주시",
            "오산시",
            "시흥시",
            "군포시",
            "의왕시",
            "하남시",
            "용인시",
            "파주시",
            "이천시",
            "안성시",
            "김포시",
            "화성시",
            "광주시",
            "양주시",
            "포천시",
            "여주시",
            "연천군",
            "가평군",
            "양평군",
        ],
    },
    KoreaSido {
        name: "강원특별자치도",
        short_name: "강원",
        cities: &[
            "춘천시",
            "원주시",
            "강릉시",
            "동해시",
            "태백시",
            "속초시",
            "삼척시",
            "홍천군",
            "횡성군",
            "영월군",
            "평창군",
            "정선군",
            "철원군",
            "화천군",
            "양구군",
            "인제군",
            "고성군",
            "양양군",
        ],
    },
    KoreaSido {
        name: "충청북도",
        short_name: "충북",
        cities: &[
            "청주시",
            "충주시",
            "제천시",
            "보은군",
            "옥천군",
            "영동군",
            "증평군",
            "진천군",
            "괴산군",
            "음성군",
            "단양군",
        ],
    },
    KoreaSido {
        name: "충청남도",
        short_name: "충남",
        cities: &[
            "천안시",
            "공주시",
            "보령시",
            "아산시",
            "서산시",
            "논산시",
            "계룡시",
            "당진시",
            "금산군",
            "부여군",
            "서천군",
            "청양군",
            "홍성군",
            "예산군",
            "태안군",
        ],
    },
    KoreaSido {
        name: "전북특별자치도",
        short_name: "전북",
        cities: &[
            "전주시",
            "군산시",
            "익산시",
            "정읍시",
            "남원시",
            "김제시",
            "완주군",
            "진안군",
            "무주군",
            "장수군",
            "임실군",
            "순창군",
            "고창군",
            "부안군",
        ],
    },
    KoreaSido {
        name: "전라남도",
        short_name: "전남",
        cities: &[
            "목포시",
            "여수시",
            "순천시",
            "나주시",
            "광양시",
            "담양군",
            "곡성군",
            "구례군",
            "고흥군",
            "보성군",
            "화순군",
            "장흥군",
            "강진군",
            "해남군",
            "영암군",
            "무안군",
            "함평군",
            "영광군",
            "장성군",
            "완도군",
            "진도군",
            "신안군",
        ],
    },
    KoreaSido {
        name: "경상북도",
        short_name: "경북",
        cities: &[
            "포항시",
            "경주시",
            "김천시",
            "안동시",
            "구미시",
            "영주시",
            "영천시",
            "상주시",
            "문경시",
            "경산시",
            "의성군",
            "청송군",
            "영양군",
            "영덕군",
            "청도군",
            "고령군",
            "성주군",
            "칠곡군",
            "예천군",
            "봉화군",
            "울진군",
            "울릉군",
        ],
    },
    KoreaSido {
        name: "경상남도",
        short_name: "경남",
        cities: &[
            "창원시",
            "진주시",
            "통영시",
            "사천시",
            "김해시",
            "밀양시",
            "거제시",
            "양산시",
            "의령군",
            "함안군",
            "창녕군",
            "고성군",
            "남해군",
            "하동군",
            "산청군",
            "함양군",
            "거창군",
            "합천군",
        ],
    },
    KoreaSido {
        name: "제주특별자치도",
        short_name: "제주",
        cities: &["제주시", "서귀포시"],
    },
];

fn dong_table() -> &'static BTreeMap<String, Vec<String>> {
    static TABLE: OnceLock<BTreeMap<String, Vec<String>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = BTreeMap::new();
        if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(KOREA_DONGS_JSON) {
            for (k, v) in obj {
                if let Value::Array(arr) = v {
                    let mut list = Vec::new();
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            let d = s.trim();
                            if !d.is_empty() && !list.iter().any(|x| x == d) {
                                list.push(d.to_owned());
                            }
                        }
                    }
                    map.insert(k, list);
                }
            }
        }
        map
    })
}

/// C++ `KoreaRegionCatalog` (`KoreaRegionCatalog.cpp`) 1:1
#[derive(Debug, Clone, Copy)]
pub struct KoreaRegionCatalog;

impl KoreaRegionCatalog {
    #[must_use]
    pub fn all_sido() -> &'static [KoreaSido] {
        BUILTIN_KOREA_SIDO
    }

    #[must_use]
    pub fn canonical_sido(name: &str) -> String {
        let t = name.trim();
        if t.is_empty() {
            return String::new();
        }
        match t {
            "강원" | "강원도" => return "강원특별자치도".to_owned(),
            "전북" | "전라북도" | "전북도" => return "전북특별자치도".to_owned(),
            "제주" | "제주도" => return "제주특별자치도".to_owned(),
            _ => {}
        }
        for s in BUILTIN_KOREA_SIDO {
            if s.name == t || s.short_name == t {
                return s.name.to_owned();
            }
        }
        t.to_owned()
    }

    #[must_use]
    pub fn cities_of(sido_name: &str) -> Vec<String> {
        let canon = Self::canonical_sido(sido_name);
        for s in BUILTIN_KOREA_SIDO {
            if s.name == canon {
                return s.cities.iter().map(|&c| c.to_owned()).collect();
            }
        }
        Vec::new()
    }

    #[must_use]
    pub fn dongs_of(sido_name: &str, city_name: &str) -> Vec<String> {
        let city = city_name.trim();
        if city.is_empty() || city == "시·군·구" {
            return Vec::new();
        }
        let key = format!("{}|{}", Self::canonical_sido(sido_name), city);
        dong_table().get(&key).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn overview_bounds(sido: &str) -> Option<KoreaRegionBounds> {
        let b = match Self::canonical_sido(sido).as_str() {
            "서울특별시" => KoreaRegionBounds {
                west: 126.76,
                south: 37.42,
                east: 127.19,
                north: 37.71,
            },
            "부산광역시" => KoreaRegionBounds {
                west: 128.79,
                south: 35.03,
                east: 129.31,
                north: 35.39,
            },
            "대구광역시" => KoreaRegionBounds {
                west: 128.35,
                south: 35.60,
                east: 128.90,
                north: 36.33,
            },
            "인천광역시" => KoreaRegionBounds {
                west: 126.57,
                south: 37.33,
                east: 126.80,
                north: 37.64,
            },
            "광주광역시" => KoreaRegionBounds {
                west: 126.64,
                south: 35.05,
                east: 127.03,
                north: 35.26,
            },
            "대전광역시" => KoreaRegionBounds {
                west: 127.24,
                south: 36.18,
                east: 127.56,
                north: 36.51,
            },
            "울산광역시" => KoreaRegionBounds {
                west: 128.97,
                south: 35.32,
                east: 129.47,
                north: 35.73,
            },
            "세종특별자치시" => KoreaRegionBounds {
                west: 127.12,
                south: 36.40,
                east: 127.42,
                north: 36.74,
            },
            "경기도" => KoreaRegionBounds {
                west: 126.51,
                south: 36.89,
                east: 127.85,
                north: 38.29,
            },
            "강원특별자치도" => KoreaRegionBounds {
                west: 127.09,
                south: 37.02,
                east: 129.37,
                north: 38.62,
            },
            "충청북도" => KoreaRegionBounds {
                west: 127.27,
                south: 36.01,
                east: 128.66,
                north: 37.26,
            },
            "충청남도" => KoreaRegionBounds {
                west: 126.11,
                south: 35.97,
                east: 127.64,
                north: 37.07,
            },
            "전북특별자치도" => KoreaRegionBounds {
                west: 126.42,
                south: 35.29,
                east: 127.92,
                north: 36.16,
            },
            "전라남도" => KoreaRegionBounds {
                west: 126.15,
                south: 34.29,
                east: 127.79,
                north: 35.49,
            },
            "경상북도" => KoreaRegionBounds {
                west: 127.79,
                south: 35.56,
                east: 129.59,
                north: 37.15,
            },
            "경상남도" => KoreaRegionBounds {
                west: 127.57,
                south: 34.76,
                east: 129.22,
                north: 35.91,
            },
            "제주특별자치도" => KoreaRegionBounds {
                west: 126.16,
                south: 33.19,
                east: 126.95,
                north: 33.57,
            },
            _ => return None,
        };
        Some(b)
    }

    #[must_use]
    pub fn compose_address(sido: &str, city: &str, dong: &str, lot: &str) -> String {
        let mut parts = Vec::new();
        let s = Self::canonical_sido(sido);
        if !s.is_empty() {
            parts.push(s);
        }
        let c = city.trim();
        if !c.is_empty() && c != "시·군·구" {
            parts.push(c.to_owned());
        }
        let d = dong.trim();
        if !d.is_empty() && d != "동·읍·면" {
            parts.push(d.to_owned());
        }
        let l = lot.trim();
        if !l.is_empty() {
            parts.push(l.to_owned());
        }
        parts.join(" ")
    }
}

/// C++ `kaIsRoadAddress` (`AddressQuery.h:7-19`) 1:1
#[must_use]
pub fn is_road_address(query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() < 2 {
        return false;
    }
    let last = words[words.len() - 1];
    let last_trimmed = last.strip_suffix('번').unwrap_or(last);
    let valid_bldg = if let Some((main, sub)) = last_trimmed.split_once('-') {
        !main.is_empty()
            && main.chars().all(|c| c.is_ascii_digit())
            && !sub.is_empty()
            && sub.chars().all(|c| c.is_ascii_digit())
    } else {
        !last_trimmed.is_empty() && last_trimmed.chars().all(|c| c.is_ascii_digit())
    };
    if !valid_bldg {
        return false;
    }
    for &word in &words[..words.len() - 1] {
        if word.ends_with("대로") || word.ends_with("길") {
            return true;
        }
        if word.ends_with('로') && word != "세종로" {
            return true;
        }
    }
    false
}

/// C++ `KaAppBar::looksLikeLot` (`KaAppBar.cpp:78-83`) 1:1
#[must_use]
pub fn looks_like_lot(query: &str) -> bool {
    if is_road_address(query) {
        return false;
    }
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() < 2 {
        return false;
    }
    let last = words[words.len() - 1];
    let last = last.strip_prefix('산').unwrap_or(last);
    let last = last.strip_suffix("번지").unwrap_or(last);
    if let Some((main, sub)) = last.split_once('-') {
        !main.is_empty()
            && main.chars().all(|c| c.is_ascii_digit())
            && !sub.is_empty()
            && sub.chars().all(|c| c.is_ascii_digit())
    } else {
        !last.is_empty() && last.chars().all(|c| c.is_ascii_digit())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParcelAddress {
    area: Vec<String>,
    mountain: bool,
    main: u32,
    sub: u32,
}

fn is_legal_locality(token: &str) -> bool {
    if token == "세종로" || token.ends_with('동') || token.ends_with('리') {
        return true;
    }
    if let Some(stem) = token.strip_suffix('가') {
        if let Some(idx) = stem.rfind('동') {
            let digits = &stem[idx + '동'.len_utf8()..];
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
        let digits: String = stem
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return true;
        }
    }
    false
}

/// C++ `parseParcelAddress` (`LocationSearch.cpp:31-65`) 1:1
fn parse_parcel_address(text: &str) -> Option<ParcelAddress> {
    let normalized = text.replace(['\u{2013}', '\u{2011}'], "-");
    let mut tokens: Vec<String> = normalized
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();
    if tokens.len() < 2 {
        return None;
    }
    let mut lot_str = tokens.pop()?;
    if lot_str == "번지" {
        lot_str = tokens.pop()?;
    } else if let Some(stripped) = lot_str.strip_suffix("번지") {
        lot_str = stripped.trim().to_owned();
    }
    let mut mountain = false;
    if let Some(stripped) = lot_str.strip_prefix('산') {
        mountain = true;
        lot_str = stripped.trim().to_owned();
    } else if tokens.last().map(String::as_str) == Some("산") {
        mountain = true;
        tokens.pop();
    }
    let (main, sub) = if let Some((m_str, s_str)) = lot_str.split_once('-') {
        let m: u32 = m_str.trim().parse().ok()?;
        let s: u32 = s_str.trim().parse().ok()?;
        if m == 0 || m > 9999 || s == 0 || s > 9999 {
            return None;
        }
        (m, s)
    } else {
        let m: u32 = lot_str.trim().parse().ok()?;
        if m == 0 || m > 9999 {
            return None;
        }
        (m, 0)
    };
    if tokens.is_empty() {
        return None;
    }
    tokens[0] = KoreaRegionCatalog::canonical_sido(&tokens[0]);
    let sejong = tokens[0] == "세종특별자치시";
    if sejong && tokens.get(1).map(String::as_str) == Some("세종시") {
        tokens.remove(1);
    }
    let province_known = KoreaRegionCatalog::all_sido()
        .iter()
        .any(|p| p.name == tokens[0]);
    if !province_known || tokens.len() < if sejong { 2 } else { 3 } {
        return None;
    }
    if !sejong
        && !tokens[1].ends_with('시')
        && !tokens[1].ends_with('군')
        && !tokens[1].ends_with('구')
    {
        return None;
    }
    let last_area = tokens.last()?;
    if !is_legal_locality(last_area) {
        return None;
    }
    Some(ParcelAddress {
        area: tokens,
        mountain,
        main,
        sub,
    })
}

/// C++ `sameParcel` (`LocationSearch.cpp:67-86`) 1:1
fn same_parcel(requested: &ParcelAddress, result: &ParcelAddress) -> bool {
    if requested.mountain != result.mountain
        || requested.main != result.main
        || requested.sub != result.sub
    {
        return false;
    }
    if requested.area == result.area {
        return true;
    }
    let mut expanded = result.area.clone();
    if expanded.len() == requested.area.len() + 1
        && expanded.len() > 3
        && expanded[1].ends_with('시')
        && expanded[2].ends_with('구')
    {
        expanded.remove(2);
        return requested.area == expanded;
    }
    false
}

fn percent_encode(input: &str) -> String {
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

fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };
    let mut cmd = Command::new(curl_bin);
    cmd.args([
        "-sS",
        "--max-time",
        "15",
        "-A",
        "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)",
        "-H",
        "Accept: application/json",
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("위치 검색 호출 실패: {e}"))?;
    if !output.status.success() {
        return Err("network_error".to_owned());
    }
    Ok(output.stdout)
}

fn read_json_coord(val: Option<&Value>) -> Option<f64> {
    let v = val?;
    let n = if let Some(f) = v.as_f64() {
        f
    } else if let Some(s) = v.as_str() {
        s.parse::<f64>().ok()?
    } else {
        return None;
    };
    n.is_finite().then_some(n)
}

fn parcel_not_found_msg() -> String {
    "지번 주소를 찾지 못했습니다. 시·군·구, 법정동·리, 번지를 확인하세요. 예: 고아읍 봉한리 / 산 12-3"
        .to_owned()
}

fn road_not_found_msg() -> String {
    "도로명주소를 찾지 못했습니다. 시·군·구, 도로명, 건물번호를 확인하세요. 예: 안동시 풍천면 하회종가길 40"
        .to_owned()
}

fn search_nominatim_sync(query: &str) -> Result<Vec<LocationHit>, String> {
    let q_enc = percent_encode(query);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={q_enc}&format=json&addressdetails=1&limit=12&countrycodes=kr&accept-language=ko"
    );
    let body = http_get_bytes(&url).map_err(|_| {
        "위치 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인하고 잠시 후 다시 검색하세요."
            .to_owned()
    })?;
    let doc: Value = serde_json::from_slice(&body).map_err(|_| {
        "위치 검색 서버가 올바른 결과를 보내지 않았습니다. 잠시 후 다시 검색하세요.".to_owned()
    })?;
    let Some(arr) = doc.as_array() else {
        return Err(
            "위치 검색 서버가 올바른 결과를 보내지 않았습니다. 잠시 후 다시 검색하세요.".to_owned(),
        );
    };
    let mut hits = Vec::new();
    for o in arr {
        let title = o
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if title.is_empty() {
            continue;
        }
        let detail = format!(
            "{} / {}",
            o.get("type").and_then(Value::as_str).unwrap_or(""),
            o.get("class").and_then(Value::as_str).unwrap_or("")
        );
        let Some(lat) = read_json_coord(o.get("lat")) else {
            continue;
        };
        let Some(lon) = read_json_coord(o.get("lon")) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        let bbox = o
            .get("boundingbox")
            .and_then(Value::as_array)
            .and_then(|bb| {
                if bb.len() != 4 {
                    return None;
                }
                let south = read_json_coord(bb.first())?;
                let north = read_json_coord(bb.get(1))?;
                let west = read_json_coord(bb.get(2))?;
                let east = read_json_coord(bb.get(3))?;
                if south >= -90.0
                    && north <= 90.0
                    && south < north
                    && west >= -180.0
                    && east <= 180.0
                    && west < east
                {
                    Some([west, south, east, north])
                } else {
                    None
                }
            });
        hits.push(LocationHit {
            title,
            detail,
            lon,
            lat,
            bbox,
            pnu: String::new(),
        });
    }
    if hits.is_empty() {
        Err("검색 결과 없음 (주소·지번·상호를 다시 입력)".to_owned())
    } else {
        Ok(hits)
    }
}

/// C++ `LocationSearch::search` / `LocationSearch::searchParcel` (`LocationSearch.cpp:227-470`) 1:1
pub fn execute_location_search(
    query: &str,
    parcel: bool,
    vworld_api_key: &str,
) -> Result<Vec<LocationHit>, String> {
    let q_trim = query.trim();
    if q_trim.is_empty() {
        return Err("주소·지번·지역·상호를 입력하세요".to_owned());
    }
    let key = vworld_api_key.trim();
    let road = !parcel && is_road_address(q_trim);
    if road {
        if key.is_empty() {
            return Err(
                "도로명주소 검색에는 VWorld API 키가 필요합니다. VWorld API 키 설정을 확인하세요."
                    .to_owned(),
            );
        }
        return search_vworld_sync(q_trim, false, true, key);
    }
    if parcel {
        let Some(parsed) = parse_parcel_address(q_trim) else {
            return Err(
                "지번 검색에는 시·군·구와 법정동·리, 번지가 필요합니다. 읍·면은 리까지 입력하세요. 예: 고아읍 봉한리 / 산 12-3"
                    .to_owned(),
            );
        };
        if key.is_empty() {
            return Err(
                "정확한 지번 검색에는 VWorld API 키가 필요합니다. VWorld API 키 설정을 확인하세요."
                    .to_owned(),
            );
        }
        let lot_fmt = if parsed.sub > 0 {
            format!(
                "{}{}-{}",
                if parsed.mountain { "산" } else { "" },
                parsed.main,
                parsed.sub
            )
        } else {
            format!("{}{}", if parsed.mountain { "산" } else { "" }, parsed.main)
        };
        let normalized_q = format!("{} {}", parsed.area.join(" "), lot_fmt);
        return search_vworld_sync(&normalized_q, true, false, key);
    }
    if !key.is_empty() {
        if let Ok(hits) = search_vworld_sync(q_trim, false, false, key) {
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
    }
    search_nominatim_sync(q_trim)
}

fn search_vworld_sync(
    query: &str,
    parcel: bool,
    road: bool,
    key: &str,
) -> Result<Vec<LocationHit>, String> {
    let size = if parcel { "1000" } else { "12" };
    let search_type = if parcel || road { "ADDRESS" } else { "place" };
    let category_param = if parcel {
        "&category=PARCEL"
    } else if road {
        "&category=ROAD"
    } else {
        ""
    };
    let q_enc = percent_encode(query);
    let key_enc = percent_encode(key);
    let url = format!(
        "https://api.vworld.kr/req/search?service=search&request=search&version=2.0&crs=EPSG:4326&size={size}&page=1&query={q_enc}&type={search_type}{category_param}&format=json&errorformat=json&key={key_enc}"
    );
    let body = match http_get_bytes(&url) {
        Ok(b) => b,
        Err(_) => {
            if parcel || road {
                return Err(if road {
                    "도로명 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 검색하세요."
                        .to_owned()
                } else {
                    "지번 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 검색하세요."
                        .to_owned()
                });
            }
            return search_nominatim_sync(query);
        }
    };
    let root: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let status = root
        .pointer("/response/status")
        .and_then(Value::as_str)
        .unwrap_or("ERROR");
    if status != "OK" {
        if parcel || road {
            return Err(if road {
                if status == "NOT_FOUND" {
                    road_not_found_msg()
                } else {
                    "도로명 검색 서버가 정상 결과를 보내지 않았습니다. VWorld API 키와 서비스 상태를 확인하세요."
                        .to_owned()
                }
            } else if status == "NOT_FOUND" {
                parcel_not_found_msg()
            } else {
                "지번 검색 서버가 정상 결과를 보내지 않았습니다. VWorld API 키와 서비스 상태를 확인하세요."
                    .to_owned()
            });
        }
        return search_nominatim_sync(query);
    }

    let Some(items) = root
        .pointer("/response/result/items")
        .and_then(Value::as_array)
    else {
        return Err(if parcel {
            parcel_not_found_msg()
        } else if road {
            road_not_found_msg()
        } else {
            "검색한 위치를 찾지 못했습니다. 주소·지번·장소 이름을 확인한 뒤 다시 검색하세요."
                .to_owned()
        });
    };

    if parcel {
        let count = root
            .pointer("/response/record/total")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })
            .unwrap_or(0);
        let pages = root
            .pointer("/response/page/total")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })
            .unwrap_or(1);
        if count > items.len() as i64 || pages > 1 {
            return Err(
                "지번 후보가 너무 많아 정확한 필지를 확인하지 못했습니다. 구·법정동·리까지 입력하세요."
                    .to_owned(),
            );
        }
    }

    let requested = if parcel {
        parse_parcel_address(query)
    } else {
        None
    };
    let mut hits = Vec::new();
    let mut seen_pnus = BTreeSet::new();

    for o in items {
        let addr = o.get("address");
        let road_str = addr
            .and_then(|a| a.get("road"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let parcel_str = addr
            .and_then(|a| a.get("parcel"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let mut title = o
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if road && !road_str.is_empty() {
            title = road_str.to_owned();
        }
        if title.is_empty() {
            title = road_str.to_owned();
        }
        let mut detail = parcel_str.to_owned();
        let mut pnu = String::new();

        if parcel {
            let (Some(req), Some(res_addr)) = (&requested, parse_parcel_address(&detail)) else {
                continue;
            };
            if !same_parcel(req, &res_addr) {
                continue;
            }
            title = detail.clone();
            pnu = o.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
            if pnu.len() != 19 || !pnu.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let expected_mtn = if res_addr.mountain { "2" } else { "1" };
            let pnu_main = pnu.get(11..15).and_then(|s| s.parse::<u32>().ok());
            let pnu_sub = pnu.get(15..19).and_then(|s| s.parse::<u32>().ok());
            if pnu.get(10..11) != Some(expected_mtn)
                || pnu_main != Some(res_addr.main)
                || pnu_sub != Some(res_addr.sub)
            {
                continue;
            }
            if seen_pnus.contains(&pnu) {
                continue;
            }
        }
        if detail.is_empty() {
            detail = road_str.to_owned();
        }
        let pt = o.get("point");
        let Some(lon) = read_json_coord(pt.and_then(|p| p.get("x"))) else {
            continue;
        };
        let Some(lat) = read_json_coord(pt.and_then(|p| p.get("y"))) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        if parcel && (!(124.0..=132.0).contains(&lon) || !(33.0..=39.0).contains(&lat)) {
            continue;
        }
        if parcel {
            seen_pnus.insert(pnu.clone());
        }
        if !title.is_empty() {
            hits.push(LocationHit {
                title,
                detail,
                lon,
                lat,
                bbox: None,
                pnu,
            });
        }
    }

    if parcel && hits.len() > 1 {
        return Err(
            "같은 지번에 서로 다른 필지 후보가 있습니다. 구·법정동·리까지 정확히 입력하세요."
                .to_owned(),
        );
    }
    if hits.is_empty() {
        Err(if parcel {
            parcel_not_found_msg()
        } else if road {
            road_not_found_msg()
        } else {
            "검색한 위치를 찾지 못했습니다. 주소·지번·장소 이름을 확인한 뒤 다시 검색하세요."
                .to_owned()
        })
    } else {
        Ok(hits)
    }
}

/// C++ `KaRegionLocator.h` 지오코딩 오류 코드 (`GeocodeError`) 1:1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeocodeError {
    NetworkFailure,
    NotFound,
}

/// C++ `KaRegionLocator::parseAddressOrCadastral` 행정구역·지번 정규 파싱 결과 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCadastralQuery {
    pub sido: String,
    pub sigungu: String,
    pub eup_myeon_dong: String,
    pub ri_or_detail: String,
    pub jibun: String,
}

/// C++ `KaRegionLocator` 지번 검색 실패 시 안내 팝업 문구 1:1
#[must_use]
pub const fn format_geocode_error_ko(err: GeocodeError) -> &'static str {
    match err {
        GeocodeError::NotFound => {
            "해당 지번을 찾을 수 없습니다. 지번 표기를 확인하거나 지도에서 직접 위치를 클릭하세요."
        }
        GeocodeError::NetworkFailure => {
            "주소 검색 서버와 통신할 수 없습니다. 네트워크 연결을 확인하거나 지도에서 직접 위치를 클릭하세요."
        }
    }
}

/// C++ `KaRegionLocator` 행정구역·지번 문자열 파서 (`"충남 부여군 부여읍 쌍북리 123"` 등) 1:1
pub fn parse_cadastral_query_ko(query: &str) -> Option<ParsedCadastralQuery> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.len() < 4 {
        return None;
    }
    let sido = tokens[0].to_owned();
    let sigungu = tokens[1].to_owned();
    let eup_myeon_dong = tokens[2].to_owned();
    let (ri_or_detail, jibun) = if tokens.len() >= 5 {
        (tokens[3].to_owned(), tokens[4..].join(" "))
    } else {
        (String::new(), tokens[3].to_owned())
    };
    let has_digit = jibun.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return None;
    }
    Some(ParsedCadastralQuery {
        sido,
        sigungu,
        eup_myeon_dong,
        ri_or_detail,
        jibun,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidoPreset {
    pub name_ko: &'static str,
    pub center_5187: [f64; 2],
}

pub const SIDO_PRESETS: [SidoPreset; 10] = [
    SidoPreset {
        name_ko: "경북 경주(월성·황룡사)",
        center_5187: [219_850.0, 360_120.0],
    },
    SidoPreset {
        name_ko: "서울(풍납토성·몽촌토성)",
        center_5187: [32_150.0, 547_300.0],
    },
    SidoPreset {
        name_ko: "충남 부여(관북리·부소산성)",
        center_5187: [-9_420.0, 408_950.0],
    },
    SidoPreset {
        name_ko: "충남 공주(공산성·송산리)",
        center_5187: [10_680.0, 428_510.0],
    },
    SidoPreset {
        name_ko: "전북 익산(왕궁리·미륵사지)",
        center_5187: [-5_120.0, 375_240.0],
    },
    SidoPreset {
        name_ko: "경남 김해(봉황동·대성동)",
        center_5187: [189_240.0, 293_110.0],
    },
    SidoPreset {
        name_ko: "경남 함안(말이산 고분군)",
        center_5187: [146_720.0, 297_850.0],
    },
    SidoPreset {
        name_ko: "대구·경산(임당동·조영동)",
        center_5187: [176_850.0, 359_400.0],
    },
    SidoPreset {
        name_ko: "전남 나주(복암리·반남)",
        center_5187: [-25_840.0, 267_900.0],
    },
    SidoPreset {
        name_ko: "강원 춘천(중도 유적)",
        center_5187: [81_400.0, 588_100.0],
    },
];

#[must_use]
pub const fn sido_presets() -> &'static [SidoPreset] {
    &SIDO_PRESETS
}

#[must_use]
pub fn build_vworld_search_url(query: &str, api_key: &str) -> String {
    format!(
        "https://api.vworld.kr/req/search?service=search&request=search&version=2.0\
         &crs=EPSG:4326&size=10&page=1&query={query}&type=address&category=parcel\
         &format=json&errorformat=json&key={api_key}"
    )
}

pub fn parse_vworld_search_json(json_bytes: &[u8]) -> StrataResult<Vec<AddressSearchHit>> {
    let root: Value = serde_json::from_slice(json_bytes)
        .map_err(|e| StrataError::Network(format!("VWorld 주소 검색 JSON 파싱 실패: {e}")))?;
    let status = root
        .pointer("/response/status")
        .and_then(Value::as_str)
        .unwrap_or("ERROR");
    if status != "OK" {
        return Ok(Vec::new());
    }
    let Some(items) = root
        .pointer("/response/result/items")
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::with_capacity(items.len());
    for item in items {
        let lon = item
            .pointer("/point/x")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok());
        let lat = item
            .pointer("/point/y")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok());
        let (Some(lon), Some(lat)) = (lon, lat) else {
            continue;
        };
        let parcel = item
            .pointer("/address/parcel")
            .and_then(Value::as_str)
            .unwrap_or("");
        let road = item
            .pointer("/address/road")
            .and_then(Value::as_str)
            .unwrap_or(parcel);
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(parcel)
            .to_owned();
        hits.push(AddressSearchHit {
            title,
            road_or_parcel: if !parcel.is_empty() {
                parcel.to_owned()
            } else {
                road.to_owned()
            },
            lon,
            lat,
        });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_address_search_parser_extracts_parcel_coords_and_sido_presets_exist(
    ) -> StrataResult<()> {
        assert!(!sido_presets().is_empty());
        let sample = r#"{
            "response": {
                "status": "OK",
                "result": {
                    "items": [
                        {
                            "title": "경상북도 경주시 인왕동 76",
                            "address": { "parcel": "경상북도 경주시 인왕동 76" },
                            "point": { "x": "129.2247", "y": "35.8347" }
                        }
                    ]
                }
            }
        }"#;
        let hits = parse_vworld_search_json(sample.as_bytes())?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "경상북도 경주시 인왕동 76");
        assert!((hits[0].lon - 129.2247).abs() < 1e-4);
        assert!((hits[0].lat - 35.8347).abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn test_region_locator_parsing_chungnam_buyeo_ssangbuk_123_extracts_admin_hierarchy_and_jibun()
    {
        let parsed = parse_cadastral_query_ko("충남 부여군 부여읍 쌍북리 123")
            .expect("valid cadastral query should parse");
        assert_eq!(parsed.sido, "충남");
        assert_eq!(parsed.sigungu, "부여군");
        assert_eq!(parsed.eup_myeon_dong, "부여읍");
        assert_eq!(parsed.ri_or_detail, "쌍북리");
        assert_eq!(parsed.jibun, "123");
        assert!(format_geocode_error_ko(GeocodeError::NotFound)
            .contains("해당 지번을 찾을 수 없습니다."));
    }
}
