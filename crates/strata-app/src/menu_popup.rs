//! Layer menus from target/cpp-ref/src/app/MainWindowContextMenus.cpp:176-448, bounded to the app viewport.
//! Nested choices, checks, disabled reasons and separators use MenuEntry; checks/arrows are painted for missing font glyphs.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::layer_menu_model::{MenuEntry, MenuRow};

const ROW_H: f32 = 24.0;
const SEP_H: f32 = 9.0;
const INK: Color32 = Color32::from_rgb(30, 30, 28);
const GREY: Color32 = Color32::from_rgb(160, 158, 150);
const HOVER: Color32 = Color32::from_rgb(232, 230, 222);

fn popup_frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::popup(&ctx.style()).fill(Color32::WHITE).inner_margin(egui::Margin::same(4))
}

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
    // target/cpp-ref/src/app/MainWindowContextMenus.cpp:192 QMenu; the owner's monitor refinement bounds the same commands.
    let available = ctx.screen_rect().width() - popup_frame(ctx).total_margin().sum().x - 8.0;
    (widest + 28.0 + 30.0).max(160.0).min(available.max(1.0))
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
    let mut job = egui::text::LayoutJob::simple_singleline(r.text.clone(), FontId::proportional(12.0), ink);
    job.wrap = egui::text::TextWrapping { max_width: (width - 58.0).max(1.0), max_rows: 1, break_anywhere: true, ..Default::default() };
    let galley = p.layout_job(job);
    let elided = galley.elided;
    let text_top = rect.center().y - galley.size().y * 0.5;
    p.galley(Pos2::new(rect.left() + 28.0, text_top), galley, ink);
    if r.submenu.is_some() {
        paint_arrow(&p, Pos2::new(rect.right() - 12.0, rect.center().y), ink);
    }
    let tooltip = if elided && r.tooltip.is_empty() {
        r.text.clone()
    } else if elided {
        format!("{}\n{}", r.text, r.tooltip)
    } else {
        r.tooltip.clone()
    };
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_ui(|ui| {
            let width = (ui.ctx().screen_rect().width() - 32.0).max(1.0).min(ui.spacing().tooltip_width);
            ui.set_max_width(width);
            ui.add(egui::Label::new(&tooltip).wrap());
        })
    }
}

/// `entries` 를 `pos` 에 그리고(`level` 겹), `path` 에 열린 하위 메뉴를 기억한다. 고른 줄과 그린 메뉴 상자들을 돌려준다.
pub(crate) fn show_menu(ctx: &egui::Context, id: egui::Id, entries: &[MenuEntry], pos: Pos2, path: &mut Vec<usize>, level: usize) -> (Option<MenuPick>, Vec<Rect>) {
    let width = width_of(ctx, entries);
    let frame = popup_frame(ctx);
    let height = (ctx.screen_rect().height() - frame.total_margin().sum().y - 8.0).max(1.0);
    let mut pick = None;
    let mut submenu_at: Option<(usize, Rect)> = None;
    // target/cpp-ref/src/app/MainWindowContextMenus.cpp:192 QMenu: crossing parent rows towards its child keeps that child open.
    // The pinned egui menu uses the same public Rect ray API (egui-0.31.1/menu.rs:715-727); O(1) for the current open child.
    let child = path.get(level).and_then(|index| ctx.memory(|memory| memory.area_rect(id.with(*index).with(level + 1))));
    let towards_child = ctx.input(|input| {
        let pointer = &input.pointer;
        let direction = pointer.direction();
        !pointer.is_still() && direction != Vec2::ZERO && child.zip(pointer.hover_pos()).is_some_and(|(rect, position)| rect.intersects_ray(position, direction.normalized()))
    });
    if towards_child {
        ctx.request_repaint();
    }
    let area = egui::Area::new(id.with(level)).order(egui::Order::Foreground).fixed_pos(pos).constrain_to(ctx.screen_rect().shrink(4.0)).show(ctx, |ui| {
        frame.show(ui, |ui| {
            ui.set_width(width);
            ui.set_max_height(height);
            egui::ScrollArea::vertical().id_salt("context_menu_rows").drag_to_scroll(false).max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                for (i, e) in entries.iter().enumerate() {
                    match e {
                        MenuEntry::Separator => {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(width, SEP_H), Sense::hover());
                            ui.painter().line_segment(
                                [Pos2::new(r.left() + 6.0, r.center().y), Pos2::new(r.right() - 6.0, r.center().y)],
                                Stroke::new(1.0, Color32::from_rgb(222, 220, 212)),
                            );
                        }
                        MenuEntry::Row(r) => {
                            let open = path.get(level) == Some(&i);
                            let resp = row_ui(ui, r, width, open);
                            if resp.hovered() && (!towards_child || resp.clicked()) {
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
    });
    let mut rects = vec![area.response.rect];
    if let Some((i, row)) = submenu_at {
        if let Some(MenuEntry::Row(MenuRow { submenu: Some(items), .. })) = entries.get(i) {
            let child_width = width_of(ctx, items) + popup_frame(ctx).total_margin().sum().x;
            let right = area.response.rect.right() - 2.0;
            let x = if right + child_width <= ctx.screen_rect().right() - 4.0 { right } else { area.response.rect.left() - child_width + 2.0 };
            let (sub_pick, sub_rects) = show_menu(ctx, id.with(i), items, Pos2::new(x, row.top() - 4.0), path, level + 1);
            pick = pick.or(sub_pick);
            rects.extend(sub_rects);
        }
    }
    (pick, rects)
}

#[cfg(test)]
#[path = "menu_popup_tests.rs"]
mod tests;
