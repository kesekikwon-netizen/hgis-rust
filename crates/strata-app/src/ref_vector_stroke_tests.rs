//! The coverage of thin lines against values worked out by hand. A pixel (x, y) has its centre at
//! (x + 0.5, y + 0.5); the coverage is clamp(half width + 0.5 - distance of the centre from the line, 0, 1).

use super::*;

fn lines(of: &[&[[f32; 2]]]) -> Polylines {
    let mut all = Polylines::default();
    for line in of {
        all.starts.push(all.points.len() as u32);
        all.points.extend_from_slice(line);
    }
    all
}

fn at(mask: &[u8], width: usize, x: usize, y: usize) -> u8 {
    mask[y * width + x]
}

/// Two pixels wide along y = 10 from x = 10 to x = 30: the rows 9 and 10 (centres half a pixel away) are
/// covered fully, the rows 8 and 11 (1.5 away) not at all; the ends are round.
#[test]
fn a_line_two_pixels_wide_covers_two_rows() {
    let mask = coverage(&lines(&[&[[10.0, 10.0], [30.0, 10.0]]]), 40, 20, 2.0, 1);
    for x in [10, 20, 29] {
        assert_eq!((at(&mask, 40, x, 9), at(&mask, 40, x, 10)), (255, 255), "column {x}");
        assert_eq!((at(&mask, 40, x, 8), at(&mask, 40, x, 11)), (0, 0), "column {x}");
    }
    // One pixel before the end point: the centre (9.5, 9.5) lies 0.707 from (10, 10): 1.5 - 0.707 = 0.793.
    assert_eq!(at(&mask, 40, 9, 9), 202);
    // Two before: 1.58 away, outside.
    assert_eq!(at(&mask, 40, 8, 9), 0);
    assert_eq!(at(&mask, 40, 31, 10), 0);
}

/// One pixel wide through pixel centres covers that row fully; between two rows it covers each half.
#[test]
fn a_line_one_pixel_wide_is_shared_between_the_rows_it_lies_between() {
    let on_centres = coverage(&lines(&[&[[5.0, 10.5], [35.0, 10.5]]]), 40, 20, 1.0, 1);
    assert_eq!((at(&on_centres, 40, 20, 9), at(&on_centres, 40, 20, 10), at(&on_centres, 40, 20, 11)), (0, 255, 0));
    let between = coverage(&lines(&[&[[5.0, 10.0], [35.0, 10.0]]]), 40, 20, 1.0, 1);
    assert_eq!((at(&between, 40, 20, 9), at(&between, 40, 20, 10)), (128, 128));
    assert_eq!((at(&between, 40, 20, 8), at(&between, 40, 20, 11)), (0, 0));
}

/// Thinner than a pixel: as wide as one, lighter by the width (0.4: 255 x 0.4 = 102).
#[test]
fn a_line_thinner_than_a_pixel_is_lighter() {
    let mask = coverage(&lines(&[&[[5.0, 10.5], [35.0, 10.5]]]), 40, 20, 0.4, 1);
    assert_eq!((at(&mask, 40, 20, 9), at(&mask, 40, 20, 10), at(&mask, 40, 20, 11)), (0, 102, 0));
}

/// A slanted line: on the diagonal of the pixels it passes their centres (full), and the pixel beside the
/// diagonal has its centre 0.707 away (1 - 0.707 = 0.293 at one pixel width: 75).
#[test]
fn a_slanted_line_covers_by_distance() {
    let mask = coverage(&lines(&[&[[2.5, 2.5], [17.5, 17.5]]]), 20, 20, 1.0, 1);
    for i in [3, 8, 12, 16] {
        assert_eq!(at(&mask, 20, i, i), 255, "on the diagonal at {i}");
        assert_eq!(at(&mask, 20, i + 1, i), 75, "beside the diagonal at {i}");
        assert_eq!(at(&mask, 20, i + 2, i), 0);
    }
}

/// Where lines meet or cross the larger coverage counts once; a line made of many pieces is the same line.
#[test]
fn joints_and_crossings_are_not_darkened_twice() {
    let cross = coverage(&lines(&[&[[5.0, 10.5], [35.0, 10.5]], &[[20.5, 2.0], [20.5, 18.0]]]), 40, 20, 0.5, 1);
    assert_eq!(at(&cross, 40, 20, 10), 128, "the crossing is as dark as either line");
    let one = coverage(&lines(&[&[[5.0, 10.3], [35.0, 10.3]]]), 40, 20, 1.5, 1);
    let pieces: Vec<[f32; 2]> = (0..=300).map(|i| [5.0 + i as f32 * 0.1, 10.3]).collect();
    let many = coverage(&lines(&[&pieces]), 40, 20, 1.5, 1);
    let worst = one.iter().zip(&many).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    assert!(worst <= 1, "differs by {worst}");
}

/// A long slanted line is cut into pieces of eight pixels; the cuts do not show.
#[test]
fn a_long_line_is_as_if_drawn_in_one_piece() {
    let mask = coverage(&lines(&[&[[1.5, 1.5], [181.5, 91.5]]]), 200, 100, 1.0, 1);
    // Along the line (slope 1/2) at pixel centres it passes through.
    for i in [0, 10, 44, 45, 80] {
        assert_eq!(at(&mask, 200, 1 + 2 * i, 1 + i), 255, "step {i}");
    }
    // The centre (2.5, 1.5) lies 1 along x from (1.5, 1.5): 1 / sqrt(5) = 0.447 from the line: 0.553.
    assert_eq!(at(&mask, 200, 2, 1), 141);
    assert_eq!(at(&mask, 200, 82, 41), 141, "the same beside a cut between two pieces");
}

/// The picture is the same however many bands it is filled in, also where a line crosses a band edge.
#[test]
fn bands_do_not_change_the_picture() {
    let all = lines(&[&[[3.0, 2.0], [90.0, 61.0], [10.0, 40.0]], &[[50.0, 0.0], [50.0, 64.0]], &[[20.5, 31.5]]]);
    let one = coverage(&all, 96, 64, 1.3, 1);
    for threads in [2, 3, 8, 64, 1000] {
        assert_eq!(coverage(&all, 96, 64, 1.3, threads), one, "{threads} bands");
    }
    assert!(one.iter().any(|c| *c > 0));
    // A line of a single point is a dot.
    assert_eq!(at(&one, 96, 20, 31), 255);
}

/// Lines outside the picture and useless widths leave the mask empty; nothing is written out of bounds.
#[test]
fn nothing_outside_the_picture_is_written() {
    let outside = lines(&[&[[-50.0, -50.0], [-10.0, -3.0]], &[[100.0, 5.0], [200.0, 5.0]], &[[5.0, 300.0], [6.0, 400.0]]]);
    assert!(coverage(&outside, 40, 20, 2.0, 4).iter().all(|c| *c == 0));
    let through = lines(&[&[[-1000.0, 10.5], [1000.0, 10.5]]]);
    let mask = coverage(&through, 40, 20, 1.0, 4);
    assert!((0..40).all(|x| at(&mask, 40, x, 10) == 255));
    for width in [0.0, -1.0, f32::NAN] {
        assert!(coverage(&through, 40, 20, width, 2).iter().all(|c| *c == 0));
    }
    assert!(coverage(&through, 0, 20, 1.0, 2).is_empty());
}

/// Laying a colour over a premultiplied picture through the mask: source over.
#[test]
fn a_colour_is_laid_over_through_the_mask() {
    // Three pixels: empty, empty, opaque white.
    let mut picture = vec![0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255];
    lay_over(&mut picture, &[255, 128, 128], [128, 64, 0, 255]);
    assert_eq!(&picture[0..4], [128, 64, 0, 255], "full cover on nothing: the colour");
    assert_eq!(&picture[4..8], [64, 32, 0, 128], "half cover on nothing: half of it, premultiplied");
    assert_eq!(&picture[8..12], [191, 159, 127, 255], "half cover on white: half way to the colour");
    // A colour with its own opacity, and no cover at all.
    let mut picture = vec![10, 20, 30, 40, 0, 0, 0, 0];
    lay_over(&mut picture, &[0, 255], [200, 100, 50, 130]);
    assert_eq!(&picture[0..4], [10, 20, 30, 40], "no cover: untouched");
    assert_eq!(&picture[4..8], [102, 51, 25, 130]);
}
