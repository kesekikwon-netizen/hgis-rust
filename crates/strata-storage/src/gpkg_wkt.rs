//! 저장할 도형 WKT 를 GPKG 도형으로 — 바뀌었거나 새로 넣는 도형에만 쓴다.
//!
//! 2차원 점·선·면과 그 Multi* 만 읽는다. Multi* 는 부분마다 따로 돌려준다: C++ 조사 표는 단일
//! 도형 형식이라 다중 면을 한 행에 넣으면 0 면적 고리가 되므로 조각마다 따로 저장한다
//! (CLAUDE.md 「multipolygon in a single-POLYGON GPKG table」). Z·M·EMPTY·GeometryCollection 은
//! 오류로 돌려 저장을 멈추게 한다(원본 파일은 그대로 남는다).

use crate::wkb_codec::WkbGeometry;

enum Node {
    Coord(Vec<f64>),
    List(Vec<Node>),
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.s.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
    }

    fn node(&mut self) -> Result<Node, String> {
        self.skip_ws();
        if self.s.get(self.pos) == Some(&b'(') {
            self.pos += 1;
            let mut items = Vec::new();
            loop {
                items.push(self.node()?);
                self.skip_ws();
                match self.s.get(self.pos) {
                    Some(b',') => self.pos += 1,
                    Some(b')') => {
                        self.pos += 1;
                        return Ok(Node::List(items));
                    }
                    _ => return Err("WKT 괄호가 맞지 않습니다".to_owned()),
                }
            }
        }
        let start = self.pos;
        while self.s.get(self.pos).is_some_and(|c| !matches!(c, b',' | b')' | b'(')) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.s[start..self.pos]).map_err(|e| e.to_string())?;
        let nums: Result<Vec<f64>, _> = text.split_whitespace().map(str::parse::<f64>).collect();
        match nums {
            Ok(v) if v.len() == 2 => Ok(Node::Coord(v)),
            Ok(_) => Err("2차원 좌표만 저장할 수 있습니다".to_owned()),
            Err(e) => Err(format!("WKT 좌표를 읽지 못했습니다: {e}")),
        }
    }
}

fn coord(n: &Node) -> Result<[f64; 2], String> {
    match n {
        Node::Coord(v) => Ok([v[0], v[1]]),
        // MULTIPOINT ((1 2),(3 4)) 처럼 괄호로 싼 점
        Node::List(items) if items.len() == 1 => coord(&items[0]),
        Node::List(_) => Err("점 자리에 목록이 있습니다".to_owned()),
    }
}

fn line(n: &Node) -> Result<Vec<[f64; 2]>, String> {
    match n {
        Node::List(items) => items.iter().map(coord).collect(),
        Node::Coord(_) => Err("선 자리에 점이 있습니다".to_owned()),
    }
}

fn polygon(n: &Node) -> Result<Vec<Vec<[f64; 2]>>, String> {
    match n {
        Node::List(rings) => rings.iter().map(line).collect(),
        Node::Coord(_) => Err("면 자리에 점이 있습니다".to_owned()),
    }
}

/// WKT 하나를 저장할 도형 부분들로 나눈다(단일 도형이면 하나).
pub fn wkt_to_parts(wkt: &str) -> Result<Vec<WkbGeometry>, String> {
    let wkt = wkt.trim();
    let open = wkt.find('(').ok_or_else(|| format!("저장할 수 없는 도형입니다: {wkt}"))?;
    let kind = wkt[..open].trim().to_ascii_uppercase();
    let mut p = Parser { s: &wkt.as_bytes()[open..], pos: 0 };
    let body = p.node()?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err("WKT 뒤에 남은 글이 있습니다".to_owned());
    }
    let Node::List(items) = &body else {
        return Err("WKT 형식이 아닙니다".to_owned());
    };
    match kind.as_str() {
        "POINT" => Ok(vec![WkbGeometry::Point(coord(&body)?)]),
        "LINESTRING" => Ok(vec![WkbGeometry::LineString(line(&body)?)]),
        "POLYGON" => Ok(vec![WkbGeometry::Polygon(polygon(&body)?)]),
        "MULTIPOINT" => items.iter().map(|n| coord(n).map(WkbGeometry::Point)).collect(),
        "MULTILINESTRING" => items.iter().map(|n| line(n).map(WkbGeometry::LineString)).collect(),
        "MULTIPOLYGON" => items.iter().map(|n| polygon(n).map(WkbGeometry::Polygon)).collect(),
        other => Err(format!("저장할 수 없는 도형 형식입니다: {other}")),
    }
}
