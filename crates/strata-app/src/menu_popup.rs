//! QMenu 처럼 그리는 팝업 메뉴 — 하위 메뉴(몇 겹이든, 마우스를 올리면 오른쪽에 열림), 체크, 흐림(툴팁에 이유),
//! 구분선. 줄은 `layer_menu_model::MenuEntry`. 글꼴에 ✓·› 가 없어 그려 넣는다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::layer_menu_model::{MenuEntry, MenuRow};

const ROW_H: f32 = 24.0;
const SEP_H: f32 = 9.0;
const INK: Color32 = Color32::from_rgb(30, 30, 28);
const GREY: Color32 = Color32::from_rgb(160, 158, 150);
const HOVER: Color32 = Color32::from_rgb(232, 230, 222);

/// 고른 줄(C++ 객체 이름과 값).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MenuPick {
    pub(crate) id: &'static str,
    pub(crate) value: String,
}

fn width_of(ctx: &egui::Context, entries: &[MenuEntry]) -> f32 {
    let widest = entries
        .iter()
        .filter_map(|e| if let MenuEntry::Row(r) = e { Some(r) } else { None })
        .map(|r| ctx.fonts(|f| f.layout_no_wrap(r.text.clone(), FontId::proportional(12.0), INK).size().x))
        .fold(0.0_f32, f32::max);
    (widest + 28.0 + 30.0).max(160.0)
}

fn paint_check(p: &egui::Painter, at: Pos2, ink: Color32) {
    let s = Stroke::new(1.6, ink);
    p.line_segment([at + Vec2::new(-4.0, 0.0), at + Vec2::new(-1.0, 3.0)], s);
    p.line_segment([at + Vec2::new(-1.0, 3.0), at + Vec2::new(4.0, -3.5)], s);
}

fn paint_arrow(p: &egui::Painter, at: Pos2, ink: Color32) {
    let s = Stroke::new(1.4, ink);
    p.line_segment([at + Vec2::new(-2.0, -4.0), at + Vec2::new(2.0, 0.0)], s);
    p.line_segment([at + Vec2::new(2.0, 0.0), at + Vec2::new(-2.0, 4.0)], s);
}

/// 한 줄을 그리고 응답을 돌려준다.
fn row_ui(ui: &mut egui::Ui, r: &MenuRow, width: f32, open: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, ROW_H), if r.enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter_at(rect);
    if r.enabled && (resp.hovered() || open) {
        p.rect_filled(rect, 3.0, HOVER);
    }
    let ink = if r.enabled { INK } else { GREY };
    if r.checked == Some(true) {
        paint_check(&p, Pos2::new(rect.left() + 13.0, rect.center().y), ink);
    }
    p.text(Pos2::new(rect.left() + 28.0, rect.center().y), egui::Align2::LEFT_CENTER, &r.text, FontId::proportional(12.0), ink);
    if r.submenu.is_some() {
        paint_arrow(&p, Pos2::new(rect.right() - 12.0, rect.center().y), ink);
    }
    if r.tooltip.is_empty() { resp } else { resp.on_hover_text(&r.tooltip) }
}

/// `entries` 를 `pos` 에 그리고(`level` 겹), `path` 에 열린 하위 메뉴를 기억한다. 고른 줄과 그린 메뉴 상자들을 돌려준다.
pub(crate) fn show_menu(ctx: &egui::Context, id: egui::Id, entries: &[MenuEntry], pos: Pos2, path: &mut Vec<usize>, level: usize) -> (Option<MenuPick>, Vec<Rect>) {
    let width = width_of(ctx, entries);
    let mut pick = None;
    let mut submenu_at: Option<(usize, Rect)> = None;
    let area = egui::Area::new(id.with(level)).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).fill(Color32::WHITE).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, e) in entries.iter().enumerate() {
                match e {
                    MenuEntry::Separator => {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(width, SEP_H), Sense::hover());
                        ui.painter().line_segment([Pos2::new(r.left() + 6.0, r.center().y), Pos2::new(r.right() - 6.0, r.center().y)], Stroke::new(1.0, Color32::from_rgb(222, 220, 212)));
                    }
                    MenuEntry::Row(r) => {
                        let open = path.get(level) == Some(&i);
                        let resp = row_ui(ui, r, width, open);
                        if resp.hovered() {
                            path.truncate(level);
                            if r.submenu.is_some() && r.enabled {
                                path.push(i);
                            }
                        }
                        if path.get(level) == Some(&i) {
                            submenu_at = Some((i, resp.rect));
                        }
                        if resp.clicked() && r.submenu.is_none() {
                            pick = Some(MenuPick { id: r.id, value: r.value.clone() });
                        }
                    }
                }
            }
        });
    });
    let mut rects = vec![area.response.rect];
    if let Some((i, row)) = submenu_at {
        if let Some(MenuEntry::Row(MenuRow { submenu: Some(items), .. })) = entries.get(i) {
            let (sub_pick, sub_rects) = show_menu(ctx, id, items, Pos2::new(area.response.rect.right() - 2.0, row.top() - 4.0), path, level + 1);
            pick = pick.or(sub_pick);
            rects.extend(sub_rects);
        }
    }
    (pick, rects)
}
