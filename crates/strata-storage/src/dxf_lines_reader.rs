//! DXF(글자 형식)에서 선·점·글자를 읽는다 — 국토지리정보원 수치지형도용.
//!
//! C++ 은 GDAL(OGR DXF)이 읽는다. 여기서는 수치지형도에 나오는 것만 읽는다:
//! `LINE`, `LWPOLYLINE`, `POLYLINE`(+`VERTEX`), `POINT`, `TEXT`. 블록 삽입·호·원은 건너뛴다.
//! 글자는 CP949(한글 도엽 이름·지명)일 수 있어 UTF-8 이 아니면 CP949 로 푼다.
//! 파일은 「그룹 번호 한 줄 + 값 한 줄」의 되풀이다.

use std::io::{BufRead, BufReader};
use std::path::Path;

/// 읽어 낸 도형 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct DxfShape {
    /// DXF 의 레이어 이름(수치지형도는 지형지물 부호, 예: `F0017111` 등고선).
    pub layer: String,
    /// 선이면 꼭짓점 여럿, 점·글자면 하나.
    pub points: Vec<[f64; 2]>,
    /// 닫힌 선(면의 테두리)인가.
    pub closed: bool,
    /// 높이(등고선의 표고 등). 없으면 0.
    pub elevation: f64,
    /// `TEXT` 의 글자. 없으면 빈 글.
    pub text: String,
}

/// 읽을 도형 수의 상한(잘못된 파일로 메모리를 크게 쓰지 않게).
pub const MAX_DXF_SHAPES: usize = 2_000_000;

fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::EUC_KR.decode(bytes).0.into_owned(),
    }
}

#[derive(Default)]
struct Pending {
    kind: String,
    layer: String,
    xs: Vec<f64>,
    ys: Vec<f64>,
    x2: Option<f64>,
    y2: Option<f64>,
    closed: bool,
    elevation: f64,
    text: String,
}

impl Pending {
    fn finish(self, out: &mut Vec<DxfShape>) {
        let mut points: Vec<[f64; 2]> = self.xs.iter().zip(&self.ys).map(|(x, y)| [*x, *y]).collect();
        if self.kind == "LINE" {
            if let (Some(x), Some(y)) = (self.x2, self.y2) {
                points.push([x, y]);
            }
        }
        let enough = match self.kind.as_str() {
            "LINE" | "LWPOLYLINE" | "POLYLINE" => points.len() >= 2,
            "POINT" | "TEXT" => points.len() == 1,
            _ => false,
        };
        if enough && points.iter().all(|p| p[0].is_finite() && p[1].is_finite()) && out.len() < MAX_DXF_SHAPES {
            out.push(DxfShape { layer: self.layer, points, closed: self.closed, elevation: self.elevation, text: self.text });
        }
    }
}

/// DXF 파일의 `ENTITIES` 구역에서 도형을 읽는다.
pub fn read_dxf_shapes(path: &Path) -> Result<Vec<DxfShape>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("DXF 파일을 열지 못했습니다: {e}"))?;
    parse_dxf(BufReader::new(file))
}

fn parse_dxf(mut reader: impl BufRead) -> Result<Vec<DxfShape>, String> {
    let mut out = Vec::new();
    let mut current: Option<Pending> = None;
    // `POLYLINE` 은 뒤따르는 `VERTEX` 들을 `SEQEND` 까지 모은다.
    let mut polyline: Option<Pending> = None;
    let mut in_entities = false;
    let mut section_next = false;
    let (mut code_line, mut value_line) = (Vec::new(), Vec::new());
    loop {
        code_line.clear();
        value_line.clear();
        let read = reader.read_until(b'\n', &mut code_line).map_err(|e| format!("DXF 를 읽지 못했습니다: {e}"))?;
        if read == 0 || reader.read_until(b'\n', &mut value_line).map_err(|e| format!("DXF 를 읽지 못했습니다: {e}"))? == 0 {
            break;
        }
        let Ok(code) = decode(&code_line).trim().parse::<i32>() else {
            return Err("DXF 형식이 아닙니다(그룹 번호를 읽지 못했습니다).".to_owned());
        };
        let value = decode(&value_line);
        let value = value.trim();
        if code == 0 {
            // 앞 도형을 마무리한다.
            if let Some(done) = current.take() {
                match (done.kind.as_str(), polyline.as_mut()) {
                    ("VERTEX", Some(line)) => {
                        line.xs.extend(done.xs);
                        line.ys.extend(done.ys);
                    }
                    ("POLYLINE", _) => {
                        // 머리의 좌표(늘 0,0 인 임시 점)는 꼭짓점이 아니다.
                        polyline = Some(Pending { xs: Vec::new(), ys: Vec::new(), ..done });
                    }
                    _ => done.finish(&mut out),
                }
            }
            match value {
                "SECTION" => section_next = true,
                "ENDSEC" => in_entities = false,
                "SEQEND" => {
                    if let Some(line) = polyline.take() {
                        line.finish(&mut out);
                    }
                }
                "EOF" => break,
                kind if in_entities => current = Some(Pending { kind: kind.to_owned(), ..Pending::default() }),
                _ => {}
            }
            continue;
        }
        if section_next && code == 2 {
            in_entities = value == "ENTITIES";
            section_next = false;
            continue;
        }
        let Some(shape) = current.as_mut() else { continue };
        let number = || value.parse::<f64>().unwrap_or(f64::NAN);
        match code {
            8 => shape.layer = value.to_owned(),
            10 => shape.xs.push(number()),
            20 => shape.ys.push(number()),
            11 => shape.x2 = Some(number()),
            21 => shape.y2 = Some(number()),
            // 높이: LWPOLYLINE 은 38, 그 밖은 첫 점의 30.
            38 => shape.elevation = number(),
            30 if shape.kind != "LWPOLYLINE" && shape.elevation == 0.0 => shape.elevation = number(),
            70 if shape.kind == "LWPOLYLINE" || shape.kind == "POLYLINE" => {
                shape.closed = value.parse::<i32>().is_ok_and(|flags| flags & 1 == 1);
            }
            1 if shape.kind == "TEXT" => shape.text = value.to_owned(),
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dxf(body: &str) -> Vec<DxfShape> {
        let text = format!("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{body}0\nENDSEC\n0\nEOF\n");
        parse_dxf(text.as_bytes()).expect("읽기")
    }

    #[test]
    fn lines_polylines_points_and_text_are_read_with_layer_and_height() {
        let shapes = dxf(concat!(
            "0\nLINE\n8\nA0010000\n10\n1.0\n20\n2.0\n30\n0.0\n11\n3.0\n21\n4.0\n31\n0.0\n",
            "0\nLWPOLYLINE\n8\nF0017111\n90\n3\n70\n1\n38\n120.0\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n",
            "0\nPOLYLINE\n8\nF0017114\n66\n1\n70\n0\n10\n0\n20\n0\n30\n50.0\n",
            "0\nVERTEX\n8\nF0017114\n10\n5\n20\n5\n30\n50.0\n0\nVERTEX\n8\nF0017114\n10\n6\n20\n7\n30\n50.0\n0\nSEQEND\n",
            "0\nPOINT\n8\nF0027217\n10\n9\n20\n8\n30\n33.5\n",
            "0\nTEXT\n8\nH0040000\n10\n1\n20\n1\n40\n2.5\n1\n광덕\n",
            "0\nCIRCLE\n8\nX\n10\n1\n20\n1\n40\n3\n",
        ));
        assert_eq!(shapes.len(), 5, "{shapes:?}");
        assert_eq!((shapes[0].layer.as_str(), shapes[0].points.clone()), ("A0010000", vec![[1.0, 2.0], [3.0, 4.0]]));
        assert_eq!((shapes[1].points.len(), shapes[1].closed, shapes[1].elevation), (3, true, 120.0));
        // POLYLINE 은 머리의 (0,0) 이 아니라 VERTEX 들이 꼭짓점이다.
        assert_eq!(shapes[2].points, vec![[0.0, 0.0], [5.0, 5.0], [6.0, 7.0]][1..].to_vec());
        assert_eq!(shapes[2].elevation, 50.0);
        assert_eq!((shapes[3].points.clone(), shapes[3].elevation), (vec![[9.0, 8.0]], 33.5));
        assert_eq!(shapes[4].text, "광덕");
    }

    #[test]
    fn korean_cp949_names_are_decoded_and_non_dxf_is_refused() {
        let mut bytes = b"0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\nH\n10\n1\n20\n1\n1\n".to_vec();
        bytes.extend_from_slice(&encoding_rs::EUC_KR.encode("전의").0);
        bytes.extend_from_slice(b"\n0\nENDSEC\n0\nEOF\n");
        assert_eq!(parse_dxf(bytes.as_slice()).expect("읽기")[0].text, "전의");
        assert!(parse_dxf(&b"PK\x03\x04 not dxf\nxx\n"[..]).is_err());
        // ENTITIES 밖(블록 정의 등)의 도형은 읽지 않는다.
        let text = "0\nSECTION\n2\nBLOCKS\n0\nLINE\n8\nA\n10\n0\n20\n0\n11\n1\n21\n1\n0\nENDSEC\n0\nEOF\n";
        assert!(parse_dxf(text.as_bytes()).expect("읽기").is_empty());
    }
}
