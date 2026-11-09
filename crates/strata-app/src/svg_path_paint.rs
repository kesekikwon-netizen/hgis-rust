//! Lucide SVG path 를 egui 로 그린다.
//!
//! C++ `KaIconsMockupEngine.cpp` 가 `QSvgRenderer` 로 그리던 것을 같은 좌표계
//! (24×24 viewBox)에서 직접 옮긴다. 지원 명령은 Lucide 가 실제로 쓰는 것만:
//! `M m L l H h V v C c Q q T t A a Z z`.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use crate::ribbon_icon_svg::{svg_parts, SvgPart};
use crate::svg_curve_flatten::{flatten_arc, flatten_cubic, flatten_quad};
use crate::svg_path_tokenize::{tokenize, Tok};


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

pub(crate) fn flush_open_pts(painter: &egui::Painter, pts: &mut Vec<Pos2>, stroke: Stroke) {
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
