//! Paints XYZ tiles inside a circle on the screen: the reference tile layers end at a distance from the
//! survey area (owner 2026-11-15). Tiles wholly outside are neither asked for nor painted; a tile on the rim
//! is painted as a finer mesh that fades out at the circle, so the map does not end on tile borders.
//! Loading follows the background tiles (`basemap_paint::paint_basemap_tiles_with`): worker threads, the
//! shared request ceiling, requests only once the view rests.

use std::collections::HashSet;

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::basemap_cadastral_jobs::{BasemapFetcher, RestDecision, TexKey, MAX_UPLOADS_PER_FRAME};
use crate::basemap_fetch::tile_filename_key;
use crate::basemap_mesh::{paint_tile_mesh, screen_tile_range_in, tile_grid_points_in, TILE_MESH_STEPS};
use crate::basemap_tiles::TileGrid;
use crate::basemap_paint::{
    paint_tile_stand_in, tm_params_for, TileTextureCache, TileView, BASEMAP_MAX_ZOOM, BASEMAP_TEXTURE_LIMIT, FULL_UV,
};
use crate::basemap_tiles::TileKind;

/// Cells along one side of a tile on the rim.
pub(crate) const EDGE_MESH_STEPS: usize = 32;

/// The picture fades out over two cells (a sixteenth of the tile's width): a fade narrower than the cells that
/// carry it would show the grid as steps.
const FADE_CELLS: f32 = 2.0;

/// The circle in screen coordinates of this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScreenCircle {
    pub(crate) center: Pos2,
    pub(crate) radius: f32,
}

impl ScreenCircle {
    /// 1 well inside, 0 from `fade.clear` pixels inside the rim outwards, a straight ramp over the
    /// `fade.ramp` pixels before that.
    fn strength(&self, p: Pos2, fade: Fade) -> f32 {
        ((self.radius - fade.clear - (p - self.center).length()) / fade.ramp).clamp(0.0, 1.0)
    }
}

/// The fade of one tile, in screen pixels.
#[derive(Debug, Clone, Copy)]
struct Fade {
    /// One cell diagonal. A cell with a corner of any strength then lies wholly inside the circle, so no
    /// painted cell reaches beyond the rim.
    clear: f32,
    ramp: f32,
}

/// The fade for a tile whose screen box is `bounds`.
fn fade_for(bounds: Rect) -> Fade {
    let cell = (bounds.width().max(bounds.height()) / EDGE_MESH_STEPS as f32).max(f32::MIN_POSITIVE);
    Fade { clear: cell * std::f32::consts::SQRT_2, ramp: cell * FADE_CELLS }
}

/// Where a tile's screen box lies against the circle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TileCover {
    Inside,
    Edge,
    Outside,
}

pub(crate) fn tile_cover(bounds: Rect, circle: &ScreenCircle) -> TileCover {
    if bounds.distance_to_pos(circle.center) > circle.radius {
        return TileCover::Outside;
    }
    let corners = [bounds.left_top(), bounds.right_top(), bounds.left_bottom(), bounds.right_bottom()];
    let fade = fade_for(bounds);
    if corners.iter().all(|c| circle.strength(*c, fade) >= 1.0) {
        TileCover::Inside
    } else {
        TileCover::Edge
    }
}

/// Mesh of a tile on the rim. `pts` is the tile's (EDGE_MESH_STEPS + 1)² grid, rows north to south. The colour
/// of each corner carries the circle's strength there, so the picture fades out along the circle itself;
/// cells without any strength are left out, and what is left lies inside the circle.
pub(crate) fn edge_mesh(texture: egui::TextureId, pts: &[Pos2], circle: &ScreenCircle, tint: Color32) -> egui::Mesh {
    let n = EDGE_MESH_STEPS;
    let w = n + 1;
    let mut mesh = egui::Mesh::with_texture(texture);
    if pts.len() != w * w {
        return mesh;
    }
    let fade = fade_for(Rect::from_points(pts));
    let strength: Vec<f32> = pts.iter().map(|p| circle.strength(*p, fade)).collect();
    mesh.reserve_vertices(pts.len());
    for j in 0..=n {
        for i in 0..=n {
            let at = j * w + i;
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pts[at],
                uv: Pos2::new(i as f32 / n as f32, j as f32 / n as f32),
                color: tint.gamma_multiply(strength[at]),
            });
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * w + i;
            let c = a + w;
            if strength[a] > 0.0 || strength[a + 1] > 0.0 || strength[c] > 0.0 || strength[c + 1] > 0.0 {
                let (a, c) = (a as u32, c as u32);
                mesh.add_triangle(a, a + 1, c);
                mesh.add_triangle(a + 1, c + 1, c);
            }
        }
    }
    mesh
}

/// Draws the ready tiles of `kind` that reach into `circle` and asks for the missing ones, nearest to the
/// view centre first. Returns the number of tiles painted from their own picture.
// The same parts as paint_basemap_tiles_with, plus the circle.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_circle_tiles_with<M, F>(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    kind: TileKind,
    z: u8,
    view: &TileView,
    circle: &ScreenCircle,
    textures: &mut TileTextureCache,
    fetcher: &BasemapFetcher,
    tint: Color32,
    make_job: &mut M,
) -> usize
where
    M: FnMut(TileKind, u8, u32, u32) -> F,
    F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
{
    if !kind.is_xyz() || !rect.is_positive() || tile_cover(rect, circle) == TileCover::Outside {
        return 0;
    }
    let Some(params) = tm_params_for(view.work_crs) else {
        return 0;
    };
    let grid = kind.grid();
    let Some((x0, y0, x1, y1)) = screen_tile_range_in(grid, view, &params, rect, z) else {
        return 0;
    };
    let count = u64::from(x1.saturating_sub(x0) + 1) * u64::from(y1.saturating_sub(y0) + 1);
    if count == 0 || count > 256 {
        return 0;
    }

    let (done, more_done) = fetcher.take_done_limited(MAX_UPLOADS_PER_FRAME);
    for (dk, image) in done {
        let name = format!("{}_{}_{}_{}", tile_filename_key(dk.0), dk.1, dk.2, dk.3);
        textures.insert(dk, ctx.load_texture(name, image, egui::TextureOptions::LINEAR));
    }
    if more_done {
        ctx.request_repaint();
    }

    let mut painted = 0_usize;
    let mut used: HashSet<TexKey> = HashSet::new();
    let mut missing: Vec<(f32, u32, u32)> = Vec::new();
    let center = rect.center();
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let Some(pts) = tile_grid_points_in(grid, view, &params, rect, (z, tx, ty), TILE_MESH_STEPS) else {
                continue;
            };
            let bounds = Rect::from_points(&pts);
            if !bounds.intersects(rect) {
                continue;
            }
            let cover = tile_cover(bounds, circle);
            if cover == TileCover::Outside {
                continue;
            }
            let k = (kind, z, tx, ty);
            if let Some(h) = textures.get(&k) {
                if cover == TileCover::Inside {
                    paint_tile_mesh(painter, h.id(), &pts, FULL_UV, tint);
                } else if let Some(fine) = tile_grid_points_in(grid, view, &params, rect, (z, tx, ty), EDGE_MESH_STEPS) {
                    painter.add(egui::Shape::mesh(edge_mesh(h.id(), &fine, circle, tint)));
                }
                used.insert(k);
                painted += 1;
                continue;
            }
            // A stand-in from another zoom fills a whole tile: only where the whole tile is inside. (The
            // stand-in painter knows the web-mercator grid only; a tile of another grid waits for its own.)
            if cover == TileCover::Inside && grid == TileGrid::WebMercator {
                paint_tile_stand_in(painter, view, &params, rect, (z, tx, ty), &pts, BASEMAP_MAX_ZOOM, tint, |sz, sx, sy| {
                    let sk = (kind, sz, sx, sy);
                    textures.get(&sk).map(|h| {
                        used.insert(sk);
                        h.id()
                    })
                });
            }
            missing.push(((pts[pts.len() / 2] - center).length_sq(), tx, ty));
        }
    }

    match fetcher.observe_view(ctx, view, rect) {
        RestDecision::AtRest => {
            missing.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| (a.2, a.1).cmp(&(b.2, b.1))));
            for &(_, tx, ty) in &missing {
                fetcher.request_with((kind, z, tx, ty), Some(ctx), make_job(kind, z, tx, ty));
            }
            if let Some(delay) = fetcher.earliest_retry_delay(missing.iter().map(|&(_, tx, ty)| (kind, z, tx, ty))) {
                ctx.request_repaint_after(delay);
            }
        }
        RestDecision::Waiting(delay) if !missing.is_empty() => ctx.request_repaint_after(delay),
        RestDecision::Dragging | RestDecision::Waiting(_) => {}
    }
    if textures.len() > BASEMAP_TEXTURE_LIMIT {
        textures.retain_kinds(&[kind], &used);
    }
    painted
}
