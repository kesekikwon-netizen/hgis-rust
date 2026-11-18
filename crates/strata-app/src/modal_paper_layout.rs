//! Paper GUI origin and bounds, target/cpp-ref/src/app/KaPaperDialog.cpp:43-47,191-198.
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Trigger {
    Ribbon,
    Sheet,
}

#[derive(Clone, Copy)]
struct Position {
    pass: u64,
    rect: Rect,
}
#[derive(Clone, Copy)]
pub(super) enum Entry {
    Clicked(Trigger),
    Keyboard,
}
#[derive(Clone, Copy)]
struct Pending {
    pass: u64,
    entry: Entry,
}

pub(crate) fn note_trigger(ctx: &egui::Context, kind: Trigger, rect: Rect, opened: bool) {
    // The owner requests a visible connection; GUI rectangles never become sheet millimetres.
    // target/cpp-ref/src/app/MainWindowExport.cpp:204-217 and KaDrawingStudio.cpp:447-454.
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new(("paper-origin", kind)), Position { pass, rect });
        if opened {
            data.insert_temp(egui::Id::new("paper-entry"), Pending { pass, entry: Entry::Clicked(kind) });
        }
    });
}

pub(crate) fn keyboard_entry(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("paper-entry"), Pending { pass, entry: Entry::Keyboard }));
}

pub(super) fn take_entry(ctx: &egui::Context) -> Option<Entry> {
    let pending = ctx.data_mut(|data| {
        let id = egui::Id::new("paper-entry");
        let entry = data.get_temp::<Pending>(id);
        data.remove::<Pending>(id);
        entry
    });
    pending.filter(|entry| entry.pass == ctx.cumulative_pass_nr()).map(|entry| entry.entry)
}

pub(super) struct Placement {
    pub width: f32,
    pub height_limit: f32,
    origin: Option<Rect>,
    anchor: Align2,
    offset: Vec2,
}

impl Placement {
    pub fn new(ctx: &egui::Context, source: Option<Trigger>) -> Self {
        let screen = ctx.screen_rect();
        let width = 440.0_f32.min((screen.width() - 24.0).max(1.0));
        let origin = source
            .and_then(|kind| ctx.data(|data| data.get_temp::<Position>(egui::Id::new(("paper-origin", kind)))))
            .filter(|p| p.pass == ctx.cumulative_pass_nr() && screen.contains_rect(p.rect))
            .map(|p| p.rect);
        let Some(rect) = origin else {
            return Self {
                width,
                height_limit: screen.height() - 24.0,
                origin: None,
                anchor: Align2::CENTER_CENTER,
                offset: Vec2::ZERO,
            };
        };
        let x = (rect.center().x - width * 0.5).clamp(screen.left() + 12.0, (screen.right() - width - 12.0).max(screen.left() + 12.0));
        if rect.center().y > screen.center().y {
            let y = rect.top() - 8.0;
            Self {
                width,
                height_limit: y - screen.top() - 12.0,
                origin,
                anchor: Align2::LEFT_BOTTOM,
                offset: Pos2::new(x, y) - screen.left_bottom(),
            }
        } else {
            let y = rect.bottom() + 8.0;
            Self {
                width,
                height_limit: screen.bottom() - y - 12.0,
                origin,
                anchor: Align2::LEFT_TOP,
                offset: Pos2::new(x, y) - screen.left_top(),
            }
        }
    }

    pub fn area(&self, id: egui::Id) -> egui::Area {
        egui::Modal::default_area(id).anchor(self.anchor, self.offset).default_width(self.width)
    }

    pub fn paint_connection(&self, response: &egui::Response, fill: Color32, edge: Stroke) {
        let Some(origin) = self.origin else { return };
        let card = response.rect;
        let x = origin.center().x.clamp(card.left() + 18.0, card.right() - 18.0);
        let (y, tip) = if self.anchor == Align2::LEFT_BOTTOM {
            (card.bottom() - 1.0, Pos2::new(x, origin.top() - 1.0))
        } else {
            (card.top() + 1.0, Pos2::new(x, origin.bottom() + 1.0))
        };
        let (a, b) = (Pos2::new(x - 8.0, y), Pos2::new(x + 8.0, y));
        let painter = response.ctx.layer_painter(response.layer_id);
        painter.add(egui::Shape::convex_polygon(vec![tip, b, a], fill, Stroke::NONE));
        painter.line_segment([a, tip], edge);
        painter.line_segment([tip, b], edge);
    }
}
