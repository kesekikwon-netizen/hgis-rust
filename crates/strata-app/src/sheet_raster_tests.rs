//! The picture in memory against what egui was asked to paint, through a context without a window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, pos2, vec2, Color32, FontId, LayerId, Pos2, RawInput, Rect, TextureOptions, ViewportId};

use super::{paint, Picture, Textures};

/// What `draw` paints on a page of `size` points, as a picture of `scale` pixels to the point.
fn drawn(size: [f32; 2], scale: f32, mut draw: impl FnMut(&egui::Context, &egui::Painter)) -> Picture {
    let ctx = egui::Context::default();
    let mut raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(size[0], size[1]))), ..Default::default() };
    raw.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(scale);
    let out = ctx.run(raw, |ctx| draw(ctx, &ctx.layer_painter(LayerId::background())));
    assert!((out.pixels_per_point - scale).abs() < 1e-6, "the page is painted at the scale asked for");
    let mut textures = Textures::default();
    textures.apply(&out.textures_delta);
    let primitives = ctx.tessellate(out.shapes, out.pixels_per_point);
    paint(&primitives, &textures, out.pixels_per_point, [(size[0] * scale) as usize, (size[1] * scale) as usize])
}

fn near(got: [u8; 3], want: [u8; 3]) -> bool {
    got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 2)
}

const WHITE: [u8; 3] = [255, 255, 255];

#[test]
fn a_print_region_keeps_its_source_position_and_clip_without_a_whole_page_buffer() {
    let ctx = egui::Context::default();
    let out = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(80.0, 40.0))), ..Default::default() }, |ctx| {
        let p = ctx.layer_painter(LayerId::background()).with_clip_rect(Rect::from_min_max(pos2(32.0, 6.0), pos2(62.0, 26.0)));
        p.rect_filled(Rect::from_min_max(pos2(30.0, 4.0), pos2(65.0, 30.0)), 0.0, Color32::from_rgb(200, 30, 30));
    });
    let mut textures = Textures::default();
    textures.apply(&out.textures_delta);
    let primitives = ctx.tessellate(out.shapes, out.pixels_per_point);
    let region = super::paint_region(&primitives, &textures, 2.0, [40, 40], [30.0, 4.0]);
    assert_eq!(region.size, [40, 40]);
    assert!(near(region.at([20, 20]), [200, 30, 30]), "source offset ignored: {:?}", region.at([20, 20]));
    assert_eq!(region.at([1, 1]), WHITE, "clip moved incorrectly");
}

/// A filled box covers its pixels with its colour and leaves the paper white around it.
#[test]
fn a_filled_box_has_its_colour_inside_and_white_paper_outside() {
    let picture = drawn([20.0, 20.0], 2.0, |_, p| {
        p.rect_filled(Rect::from_min_max(pos2(5.0, 5.0), pos2(15.0, 15.0)), 0.0, Color32::from_rgb(200, 30, 30));
    });
    assert_eq!(picture.size, [40, 40]);
    for at in [[20, 20], [12, 12], [27, 27], [12, 27]] {
        assert!(near(picture.at(at), [200, 30, 30]), "{at:?} is {:?}", picture.at(at));
    }
    for at in [[4, 4], [20, 6], [34, 20], [20, 34]] {
        assert_eq!(picture.at(at), WHITE, "{at:?}");
    }
}

/// A half-clear colour over another mixes as the screen mixes: the upper colour as egui stores it (a blue of
/// 128 in 255 is kept as 0, 0, 188 with the clearness multiplied in), plus what lies under by what the upper
/// one leaves: 200, 30, 30 under it gives 100, 15, 188 + 15.
#[test]
fn a_half_clear_colour_lets_the_colour_under_it_through() {
    let picture = drawn([20.0, 20.0], 1.0, |_, p| {
        p.rect_filled(Rect::from_min_max(pos2(2.0, 2.0), pos2(18.0, 18.0)), 0.0, Color32::from_rgb(200, 30, 30));
        p.rect_filled(Rect::from_min_max(pos2(6.0, 6.0), pos2(14.0, 14.0)), 0.0, Color32::from_rgba_unmultiplied(0, 0, 255, 128));
    });
    assert!(near(picture.at([10, 10]), [100, 15, 203]), "{:?}", picture.at([10, 10]));
    assert!(near(picture.at([4, 4]), [200, 30, 30]));
}

/// The two triangles of a box meet without a seam: no pixel of a half-clear box is painted twice or not at all.
#[test]
fn the_two_halves_of_a_box_meet_without_a_seam() {
    let picture = drawn([32.0, 32.0], 1.0, |_, p| {
        p.rect_filled(Rect::from_min_max(pos2(4.0, 4.0), pos2(28.0, 28.0)), 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 100));
    });
    let middle = picture.at([16, 16]);
    assert!(middle[0] > 120 && middle[0] < 190, "grey over white: {middle:?}");
    for y in 6..26 {
        for x in 6..26 {
            assert_eq!(picture.at([x, y]), middle, "[{x}, {y}]");
        }
    }
}

/// A picture is stretched over its box: with the nearest pixel each half keeps its colour, with the smooth
/// filter the middle is a mix of both.
#[test]
fn a_picture_is_stretched_over_its_box() {
    let two = egui::ColorImage { size: [2, 1], pixels: vec![Color32::from_rgb(250, 0, 0), Color32::from_rgb(0, 0, 250)] };
    let shown = |options: TextureOptions| {
        let two = two.clone();
        drawn([20.0, 10.0], 1.0, move |ctx, p| {
            let texture = ctx.load_texture("two", two.clone(), options);
            p.image(texture.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(20.0, 10.0)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        })
    };
    let nearest = shown(TextureOptions::NEAREST);
    assert!(near(nearest.at([3, 5]), [250, 0, 0]) && near(nearest.at([9, 5]), [250, 0, 0]));
    assert!(near(nearest.at([10, 5]), [0, 0, 250]) && near(nearest.at([17, 5]), [0, 0, 250]));
    let smooth = shown(TextureOptions::LINEAR);
    assert!(near(smooth.at([1, 5]), [250, 0, 0]) && near(smooth.at([18, 5]), [0, 0, 250]));
    let middle = smooth.at([10, 5]);
    assert!(middle[0] > 90 && middle[0] < 160 && middle[2] > 90 && middle[2] < 160, "{middle:?}");
}

/// Nothing is painted outside the box a painter is cut to.
#[test]
fn a_cut_painter_paints_inside_its_box_only() {
    let picture = drawn([20.0, 20.0], 2.0, |_, p| {
        let cut = p.with_clip_rect(Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 20.0)));
        cut.rect_filled(Rect::from_min_max(pos2(2.0, 2.0), pos2(18.0, 18.0)), 0.0, Color32::BLACK);
    });
    assert!(near(picture.at([10, 20]), [0, 0, 0]));
    assert_eq!(picture.at([21, 20]), WHITE);
    assert_eq!(picture.at([30, 20]), WHITE);
}

/// Letters are painted from letters made for the scale of the picture: at four pixels to the point their
/// strokes are fully black inside, and nothing is painted away from them.
#[test]
fn letters_are_sharp_at_the_scale_of_the_picture() {
    let picture = drawn([120.0, 30.0], 4.0, |_, p| {
        p.text(pos2(8.0, 6.0), egui::Align2::LEFT_TOP, "Strata", FontId::proportional(14.0), Color32::BLACK);
    });
    let dark = |at: [usize; 2]| picture.at(at)[0] < 40;
    let mut black = 0;
    for y in 0..picture.size[1] {
        for x in 0..picture.size[0] {
            if dark([x, y]) {
                black += 1;
                assert!((24..300).contains(&x) && (16..100).contains(&y), "ink away from the letters at [{x}, {y}]");
            }
        }
    }
    // Six letters of 14 points at 4 pixels to the point: strokes about 5 pixels wide. Letters are strokes, not
    // filled boxes: well under half of the box the word takes is ink.
    let word_box = (300 - 24) * (100 - 16);
    assert!(black > 800 && black * 2 < word_box, "{black} black pixels of {word_box}");
}

/// A picture nobody handed over is counted and left out; the rest is painted.
#[test]
fn an_unknown_picture_is_counted_and_left_out() {
    let picture = drawn([20.0, 20.0], 1.0, |_, p| {
        p.rect_filled(Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 4.0)), 0.0, Color32::BLACK);
        p.image(egui::TextureId::User(77), Rect::from_min_max(pos2(8.0, 8.0), pos2(16.0, 16.0)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    });
    assert_eq!(picture.unknown_pictures, 1);
    assert!(near(picture.at([2, 2]), [0, 0, 0]));
    assert_eq!(picture.at([12, 12]), WHITE);
}

/// A letter is painted from its shape, not as the box it sits in: the middle of an 「O」 stays white.
#[test]
fn the_hole_of_a_letter_stays_white() {
    let picture = drawn([60.0, 60.0], 3.0, |_, p| {
        p.text(pos2(30.0, 30.0), egui::Align2::CENTER_CENTER, "O", FontId::proportional(40.0), Color32::BLACK);
    });
    assert_eq!(picture.at([90, 90]), WHITE, "the hole of the O");
    let ring = (0..90).filter(|x| picture.at([*x, 90])[0] < 60).count();
    assert!(ring >= 4, "the left side of the ring is ink: {ring} pixels");
}
