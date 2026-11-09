//! 국토정보플랫폼 응답 검증·해석 (C++ `ngii-topographic.js:47-116`).
//!
//! `ngii_topographic` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use serde_json::Value;

use super::ngii_protocol::NgiiTopographicClient;
use super::ngii_sheets::{SheetProduct, SheetQuery, SheetScope};

impl NgiiTopographicClient {
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
}
