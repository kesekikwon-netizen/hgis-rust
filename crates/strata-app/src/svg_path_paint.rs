//! Lucide SVG path 를 egui 로 그린다.
//!
//! C++ `KaIconsMockupEngine.cpp` 가 `QSvgRenderer` 로 그리던 것을 같은 좌표계
//! (24×24 viewBox)에서 직접 옮긴다. 지원 명령은 Lucide 가 실제로 쓰는 것만:
//! `M m L l H h V v C c Q q T t A a Z z`.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use crate::ribbon_icon_svg::{svg_parts, SvgPart};

const VIEW: f32 = 24.0;

/// C++ `kGlyphUnits / kTileUnits` = 18 / 32 — 타일 안 글리프 상자 비율.
pub const GLYPH_RATIO: f32 = 18.0 / 32.0;

/// 아이콘 id 로 글리프를 그린다. 모르는 id 면 `false` 를 돌려준다.
pub fn paint_svg_icon(painter: &egui::Painter, rect: egui::Rect, id: &str, ink: Color32) -> bool {
    let Some(parts) = svg_parts(id) else {
        return false;
    };
    let side = rect.width().min(rect.height());
    let scale = side / VIEW;
    let origin = rect.center() - Vec2::splat(side * 0.5);
    let to_pt = |x: f32, y: f32| Pos2::new(origin.x + x * scale, origin.y + y * scale);

    // Lucide 기본 stroke-width="2" — 24 viewBox 기준
    let stroke = Stroke::new((2.0 * scale).max(1.0), ink);

    for part in parts {
        match part {
            SvgPart::Path(d) => draw_path(painter, d, &to_pt, stroke),
            SvgPart::Circle(v) => {
                let f = nums(v);
                if f.len() >= 3 {
                    painter.circle_stroke(to_pt(f[0], f[1]), f[2] * scale, stroke);
                }
            }
            SvgPart::Line(v) => {
                let f = nums(v);
                if f.len() >= 4 {
                    painter.line_segment([to_pt(f[0], f[1]), to_pt(f[2], f[3])], stroke);
                }
            }
            SvgPart::Rect(v) => {
                let f = nums(v);
                if f.len() >= 4 {
                    let r = egui::Rect::from_min_size(
                        to_pt(f[0], f[1]),
                        Vec2::new(f[2] * scale, f[3] * scale),
                    );
                    let rad = if f.len() > 4 { f[4] * scale } else { 0.0 };
                    if rad > 0.0 {
                        painter.rect_stroke(r, rad, stroke, egui::StrokeKind::Middle);
                    } else {
                        painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                    }
                }
            }
            SvgPart::Polyline(v) => {
                let f = nums(v);
                let pts: Vec<Pos2> = f.chunks_exact(2).map(|c| to_pt(c[0], c[1])).collect();
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
        }
    }
    true
}

/// `a b c` 또는 `1|2|3` 을 f32 목록으로.
fn nums(s: &str) -> Vec<f32> {
    s.split(|c: char| c == '|' || c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f32>().ok())
        .collect()
}

/// SVG path 서브셋 파서. 절대/상대 명령을 모두 24 좌표계 점으로 바꾼다.
fn draw_path(painter: &egui::Painter, d: &str, to_pt: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke) {
    let tok = tokenize(d);
    let mut i = 0usize;
    let mut cur = (0.0_f32, 0.0_f32);
    let mut start = (0.0_f32, 0.0_f32);
    let mut cmd = 'M';
    let mut pts: Vec<Pos2> = Vec::new();
    let mut last_ctrl: Option<(f32, f32)> = None;

    let rel = |c: char| c.is_ascii_lowercase();

    while i < tok.len() {
        if let Tok::Cmd(c) = tok[i] {
            cmd = c;
            i += 1;
            if c == 'Z' || c == 'z' {
                let mut closed = std::mem::take(&mut pts);
                let s_pt = to_pt(start.0, start.1);
                if closed.len() >= 2 && (closed[closed.len() - 1] - s_pt).length() < 0.2 {
                    closed.pop();
                }
                if closed.len() >= 3 {
                    painter.add(egui::Shape::closed_line(closed, stroke));
                } else if closed.len() == 2 {
                    painter.add(egui::Shape::line(closed, stroke));
                }
                cur = start;
                last_ctrl = None;
                continue;
            }
            continue;
        }
        let lower = cmd.to_ascii_lowercase();
        let ox = if rel(cmd) { cur.0 } else { 0.0 };
        let oy = if rel(cmd) { cur.1 } else { 0.0 };
        let need = match lower {
            'm' | 'l' | 't' => 2,
            'h' | 'v' => 1,
            'c' => 6,
            'q' | 's' => 4,
            'a' => 7,
            _ => {
                i += 1;
                continue;
            }
        };
        if i + need > tok.len() {
            break;
        }
        let mut f = [0.0_f32; 7];
        let mut ok = true;
        for k in 0..need {
            match tok[i + k] {
                Tok::Num(v) => f[k] = v,
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            i += 1;
            continue;
        }
        i += need;

        match lower {
            'm' => {
                flush_open_pts(painter, &mut pts, stroke);
                cur = (ox + f[0], oy + f[1]);
                start = cur;
                pts.push(to_pt(cur.0, cur.1));
                last_ctrl = None;
                // 뒤따르는 좌표쌍은 lineto 로 이어 그린다 (SVG 규칙)
                cmd = if rel(cmd) { 'l' } else { 'L' };
            }
            'l' => {
                cur = (ox + f[0], oy + f[1]);
                pts.push(to_pt(cur.0, cur.1));
                last_ctrl = None;
            }
            'h' => {
                cur = (ox + f[0], cur.1);
                pts.push(to_pt(cur.0, cur.1));
                last_ctrl = None;
            }
            'v' => {
                cur = (cur.0, oy + f[0]);
                pts.push(to_pt(cur.0, cur.1));
                last_ctrl = None;
            }
            'c' => {
                let c1 = (ox + f[0], oy + f[1]);
                let c2 = (ox + f[2], oy + f[3]);
                let end = (ox + f[4], oy + f[5]);
                flatten_cubic(&mut pts, cur, c1, c2, end, to_pt);
                cur = end;
                last_ctrl = Some(c2);
            }
            'q' => {
                let c1 = (ox + f[0], oy + f[1]);
                let end = (ox + f[2], oy + f[3]);
                flatten_quad(&mut pts, cur, c1, end, to_pt);
                cur = end;
                last_ctrl = Some(c1);
            }
            't' => {
                let c1 = last_ctrl
                    .map(|c| (2.0 * cur.0 - c.0, 2.0 * cur.1 - c.1))
                    .unwrap_or(cur);
                let end = (ox + f[0], oy + f[1]);
                flatten_quad(&mut pts, cur, c1, end, to_pt);
                cur = end;
                last_ctrl = Some(c1);
            }
            'a' => {
                // 호는 타원 근사 대신 끝점만 이어 그린다 (Lucide 아이콘에서 짧은 호)
                let end = (ox + f[5], oy + f[6]);
                flatten_arc(
                    &mut pts,
                    cur,
                    f[0],
                    f[1],
                    f[2],
                    f[3] != 0.0,
                    f[4] != 0.0,
                    end,
                    to_pt,
                );
                cur = end;
                last_ctrl = None;
            }
            _ => {}
        }
    }
    flush_open_pts(painter, &mut pts, stroke);
}

fn flush_open_pts(painter: &egui::Painter, pts: &mut Vec<Pos2>, stroke: Stroke) {
    let seg = std::mem::take(pts);
    if seg.len() >= 2 {
        let first = seg[0];
        let last = seg[seg.len() - 1];
        if (last - first).length() < 0.25 * stroke.width && seg.len() == 2 {
            painter.circle_filled(first, stroke.width * 0.55, stroke.color);
        } else {
            let cap_r = (stroke.width * 0.42).max(0.5);
            painter.add(egui::Shape::line(seg, stroke));
            painter.circle_filled(first, cap_r, stroke.color);
            painter.circle_filled(last, cap_r, stroke.color);
        }
    }
}

fn flatten_cubic(
    pts: &mut Vec<Pos2>,
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    to_pt: &dyn Fn(f32, f32) -> Pos2,
) {
    const N: usize = 10;
    for k in 1..=N {
        let t = k as f32 / N as f32;
        let u = 1.0 - t;
        let x =
            u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0;
        let y =
            u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1;
        pts.push(to_pt(x, y));
    }
}

fn flatten_quad(
    pts: &mut Vec<Pos2>,
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    to_pt: &dyn Fn(f32, f32) -> Pos2,
) {
    const N: usize = 10;
    for k in 1..=N {
        let t = k as f32 / N as f32;
        let u = 1.0 - t;
        let x = u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0;
        let y = u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1;
        pts.push(to_pt(x, y));
    }
}

/// W3C SVG 1.1 Appendix F.6.5 / F.6.6 — 타원 호(endpoint parameterization → center parameterization) 정밀 보간
#[allow(clippy::too_many_arguments)]
fn flatten_arc(
    pts: &mut Vec<Pos2>,
    p0: (f32, f32),
    rx_in: f32,
    ry_in: f32,
    rot_deg: f32,
    large: bool,
    sweep: bool,
    p1: (f32, f32),
    to_pt: &dyn Fn(f32, f32) -> Pos2,
) {
    let dx2 = (p0.0 - p1.0) * 0.5;
    let dy2 = (p0.1 - p1.1) * 0.5;
    if dx2.abs() < 1e-5 && dy2.abs() < 1e-5 {
        return;
    }
    let mut rx = rx_in.abs();
    let mut ry = ry_in.abs();
    if rx < 1e-5 || ry < 1e-5 {
        pts.push(to_pt(p1.0, p1.1));
        return;
    }
    let phi = rot_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // Step 1 (F.6.5.1): (x1', y1')
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    // F.6.6.3: 반지름 보정
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    // Step 2 (F.6.5.2): (cx', cy')
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let x1p2 = x1p * x1p;
    let y1p2 = y1p * y1p;
    let num = (rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2).max(0.0);
    let den = (rx2 * y1p2 + ry2 * x1p2).max(1e-9);
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let coef = sign * (num / den).sqrt();
    let cxp = coef * (rx * y1p / ry);
    let cyp = coef * (-ry * x1p / rx);

    // Step 3 (F.6.5.3): (cx, cy)
    let cx = cos_phi * cxp - sin_phi * cyp + (p0.0 + p1.0) * 0.5;
    let cy = sin_phi * cxp + cos_phi * cyp + (p0.1 + p1.1) * 0.5;

    // Step 4 (F.6.5.5 - F.6.5.6): theta1, dtheta
    let vec_angle = |ux: f32, uy: f32, vx: f32, vy: f32| -> f32 {
        let cross = ux * vy - uy * vx;
        let dot = ux * vx + uy * vy;
        cross.atan2(dot)
    };
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta1 = vec_angle(1.0, 0.0, ux, uy);
    let mut dtheta = vec_angle(ux, uy, vx, vy);
    if !sweep && dtheta > 0.0 {
        dtheta -= std::f32::consts::TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += std::f32::consts::TAU;
    }

    let steps = ((dtheta.abs() * 10.0).ceil() as usize).clamp(6, 28);
    for k in 1..=steps {
        let t = k as f32 / steps as f32;
        let theta = theta1 + dtheta * t;
        let (st, ct) = theta.sin_cos();
        let x = cos_phi * rx * ct - sin_phi * ry * st + cx;
        let y = sin_phi * rx * ct + cos_phi * ry * st + cy;
        pts.push(to_pt(x, y));
    }
}

#[derive(Clone, Copy, Debug)]
enum Tok {
    Cmd(char),
    Num(f32),
}

fn tokenize(d: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b: Vec<char> = d.chars().collect();
    let mut i = 0usize;
    let mut cur_cmd = 'M';
    let mut arg_idx = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphabetic() {
            out.push(Tok::Cmd(c));
            cur_cmd = c;
            arg_idx = 0;
            i += 1;
            continue;
        }
        if c.is_whitespace() || c == ',' {
            i += 1;
            continue;
        }
        // SVG 호(A/a) 명령의 4번째·5번째 인자(large-arc-flag, sweep-flag)는 단일 문자 '0' 또는 '1'
        if (cur_cmd == 'A' || cur_cmd == 'a')
            && (arg_idx % 7 == 3 || arg_idx % 7 == 4)
            && (c == '0' || c == '1')
        {
            out.push(Tok::Num(if c == '1' { 1.0 } else { 0.0 }));
            arg_idx += 1;
            i += 1;
            continue;
        }
        let start = i;
        let mut seen_dot = false;
        if c == '-' || c == '+' {
            i += 1;
        }
        while i < b.len() {
            if b[i].is_ascii_digit() {
                i += 1;
            } else if b[i] == '.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else {
                break;
            }
        }
        if i < b.len() && (b[i] == 'e' || b[i] == 'E') {
            i += 1;
            if i < b.len() && (b[i] == '-' || b[i] == '+') {
                i += 1;
            }
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
        let s: String = b[start..i].iter().collect();
        if let Ok(v) = s.parse::<f32>() {
            out.push(Tok::Num(v));
            arg_idx += 1;
        }
        if i == start {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // D26 — C++ Lucide SVG 52개가 모두 토큰화·파싱된다.
    #[test]
    fn test_svg_icons_tokenize_and_parse() {
        // MainWindowRibbon.cpp 가 쓰는 리본 id 전부
        let ids = [
            "new",
            "open",
            "save",
            "save_as",
            "select",
            "measure",
            "draw_poly",
            "trench_grid",
            "buffer",
            "cadastral",
            "topo_download",
            "heritage",
            "contour",
            "dem",
            "survey_contour",
            "soil",
            "paleo",
            "geology",
            "river",
            "old_map",
            "georef",
            "pdf",
            "print",
            "geotiff",
            "export_convert",
            "more",
        ];
        for id in ids {
            let parts = svg_parts(id);
            assert!(parts.is_some(), "missing C++ icon: {id}");
            let parts = parts.unwrap_or(&[]);
            assert!(!parts.is_empty(), "empty icon: {id}");
        }

        // 모든 path 데이터가 숫자 토큰을 낸다
        for id in ids {
            if let Some(parts) = svg_parts(id) {
                for part in parts {
                    if let SvgPart::Path(d) = part {
                        let toks = tokenize(d);
                        assert!(!toks.is_empty(), "{id}: path produced no tokens");
                        assert!(
                            toks.iter().any(|t| matches!(t, Tok::Num(_))),
                            "{id}: path has no numbers"
                        );
                    }
                }
            }
        }
    }

    // KaIconsMockupEngine.cpp:25-26 — 글리프는 타일의 18/32
    #[test]
    fn test_glyph_ratio_matches_cpp() {
        assert!((GLYPH_RATIO - 0.5625).abs() < 1e-6);
    }
}
