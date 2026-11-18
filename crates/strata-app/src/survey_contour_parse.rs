//! Coordinate-table reading for the contour preview.
//!
//! `target/cpp-ref/src/core/SurveyPointSchema.cpp:11-109` (names, headerless sheets,
//! comma numbers) and `SurveyPointReader.cpp:200-250` (skip a bad row, tag z == 0 or
//! |z| >= 999, average duplicates rounded to a millimetre).

use std::collections::HashMap;

use strata_storage::csv_control_points::decode_korean_text;

pub(super) const NO_SHEET: &str = "시트나 CAD 레이어가 없습니다.";
pub(super) const NO_COLUMNS: &str = "X·Y·표고 칸을 찾지 못했습니다. 칸을 직접 지정하세요.";

#[derive(Debug, Clone)]
pub(super) struct Elev {
    pub z: f64,
    pub suspicious: bool,
}

pub(super) fn read_table(bytes: &[u8]) -> Result<Vec<Elev>, &'static str> {
    let text = decode_korean_text(bytes);
    let raw: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if raw.is_empty() {
        return Err(NO_SHEET);
    }
    let sep = if raw[0].contains('\t') && !raw[0].contains(',') {
        '\t'
    } else {
        ','
    };
    let rows: Vec<Vec<String>> = raw.iter().map(|line| split_row(line, sep)).collect();
    let headerless = rows[0].iter().all(|cell| is_numeric_header(cell));
    let names: Vec<String> = if headerless {
        (1..=rows[0].len()).map(|i| format!("field_{i}")).collect()
    } else {
        rows[0].clone()
    };
    let cols = guess_columns(&names);
    if cols.x < 0 || cols.y < 0 || cols.z < 0 {
        return Err(NO_COLUMNS);
    }
    let data: &[Vec<String>] = if headerless { &rows } else { &rows[1..] };
    Ok(average_duplicates(data, cols))
}

struct Bucket {
    z_sum: f64,
    count: u32,
    suspicious: bool,
}

fn average_duplicates(rows: &[Vec<String>], cols: Columns) -> Vec<Elev> {
    let mut buckets: HashMap<(i64, i64), Bucket> = HashMap::new();
    for row in rows {
        let xs = field(row, cols.x);
        let ys = field(row, cols.y);
        let zs = field(row, cols.z);
        if xs.is_empty() && ys.is_empty() && zs.is_empty() {
            continue;
        }
        let (Some(x), Some(y)) = (parse_number(xs), parse_number(ys)) else {
            continue;
        };
        let Some(z) = parse_number(zs) else {
            continue;
        };
        let key = (round_i64(x * 1000.0), round_i64(y * 1000.0));
        let entry = buckets.entry(key).or_insert(Bucket {
            z_sum: 0.0,
            count: 0,
            suspicious: false,
        });
        entry.suspicious = entry.suspicious || z == 0.0 || z.abs() >= 999.0;
        entry.z_sum += z;
        entry.count += 1;
    }
    buckets
        .into_values()
        .map(|bucket| Elev {
            z: bucket.z_sum / f64::from(bucket.count),
            suspicious: bucket.suspicious,
        })
        .collect()
}

struct Columns {
    x: i32,
    y: i32,
    z: i32,
    id: i32,
}

fn guess_columns(names: &[String]) -> Columns {
    let mut cols = Columns {
        x: -1,
        y: -1,
        z: -1,
        id: -1,
    };
    for (index, name) in names.iter().enumerate() {
        let name = norm(name);
        let index = i32::try_from(index).unwrap_or(i32::MAX);
        if cols.id < 0 && is_id(&name) {
            cols.id = index;
        } else if cols.x < 0 && is_north(&name) {
            cols.x = index;
        } else if cols.y < 0 && is_east(&name) {
            cols.y = index;
        } else if cols.z < 0 && is_elev(&name) {
            cols.z = index;
        }
    }
    let count = i32::try_from(names.len()).unwrap_or(i32::MAX);
    if (cols.x < 0 || cols.y < 0 || cols.z < 0) && count >= 3 {
        let base = if count >= 4 && cols.id < 0 { 1 } else { 0 };
        if base + 2 < count {
            if cols.id < 0 && base == 1 {
                cols.id = 0;
            }
            if cols.x < 0 {
                cols.x = base;
            }
            if cols.y < 0 {
                cols.y = base + 1;
            }
            if cols.z < 0 {
                cols.z = base + 2;
            }
        }
    }
    cols
}

fn is_numeric_header(cell: &str) -> bool {
    let name = norm(cell);
    name.starts_with("field") || parse_number(&name).is_some()
}

fn field(row: &[String], index: i32) -> &str {
    usize::try_from(index)
        .ok()
        .and_then(|i| row.get(i))
        .map_or("", |cell| cell.trim())
}

fn norm(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .filter(|c| *c != ' ' && *c != '_')
        .collect()
}

fn is_id(name: &str) -> bool {
    matches!(
        name,
        "점번호" | "측점" | "점명" | "포인트명" | "번호" | "no" | "pt" | "id" | "name" | "point"
    )
}

fn is_north(name: &str) -> bool {
    matches!(
        name,
        "북" | "북쪽" | "x" | "x(n)" | "n" | "north" | "northing"
    )
}

fn is_east(name: &str) -> bool {
    matches!(
        name,
        "동" | "동쪽" | "y" | "y(e)" | "e" | "east" | "easting"
    )
}

fn is_elev(name: &str) -> bool {
    matches!(
        name,
        "표고" | "높이" | "해발" | "z" | "z(h)" | "h" | "el" | "elev" | "elevation"
    )
}

/// `SurveyPointSchema.cpp:101-109`.
pub(super) fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let commas = text.matches(',').count();
    let cleaned: String = if commas > 1 || text.contains('.') {
        text.chars().filter(|c| *c != ',').collect()
    } else {
        text.replace(',', ".")
    };
    let value: f64 = cleaned.parse().ok()?;
    value.is_finite().then_some(value)
}

fn round_i64(value: f64) -> i64 {
    let rounded = value.round();
    if rounded >= i64::MAX as f64 {
        i64::MAX
    } else if rounded <= i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
    }
}

fn split_row(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                cur.push(ch);
            }
        } else if ch == '"' && cur.is_empty() {
            quoted = true;
        } else if ch == sep {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    out.push(cur);
    out
}
