use super::embed_face;

#[test]
fn the_subset_still_has_the_hangul_glyph() {
    let face = embed_face("유구배치도".chars()).expect("a Hangul face");
    let parsed = ttf_parser::Face::parse(&face.ttf, 0).expect("subset font");
    let gid = ttf_parser::GlyphId(*face.cid.get(&'유').expect("유 is in the subset"));
    assert!(gid.0 > 0);
    assert!(parsed.glyph_bounding_box(gid).is_some(), "유 has no outline");
}
