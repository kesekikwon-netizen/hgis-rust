//! SVG 호(arc)·베지어 곡선을 폴리라인으로 펴는 계산.
//!
//! `svg_path_paint.rs` 가 300줄을 넘어 이 파일로 나눴다.

use eframe::egui::Pos2;


pub(crate) fn flatten_cubic(
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

pub(crate) fn flatten_quad(
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
pub(crate) fn flatten_arc(
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
