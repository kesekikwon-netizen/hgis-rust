//! 조사 파일(EPSG:5186)을 작업 좌표계(EPSG:5187)로 바꾸는 값이 PROJ 와 같은지.
//! 기준값은 OSGeo4W gdaltransform(PROJ) 로 구했다:
//! `gdaltransform -s_srs EPSG:5186 -t_srs EPSG:5187 -output_xy` (demo.gpkg 조사구역 모서리·기준점).

use super::CrsTransformer;
use strata_types::crs::CrsCode;

const PROJ_5186_TO_5187: [([f64; 2], [f64; 2]); 5] = [
    ([200000.0, 450000.0], [21149.9056088904, 451863.684731702]),
    ([200120.0, 450000.0], [21269.9267789586, 451861.182962828]),
    ([200120.0, 450100.0], [21272.0116072648, 451961.200577206]),
    ([200000.0, 450100.0], [21151.9904394108, 451963.702398923]),
    ([200050.0, 450020.0], [21200.3313998556, 451882.64585687]),
];

#[test]
fn epsg5186_to_5187_matches_proj_within_5_cm() {
    let t = CrsTransformer::new(CrsCode::Epsg5186, CrsCode::Epsg5187);
    let mut worst = 0.0_f64;
    for (src, want) in PROJ_5186_TO_5187 {
        let got = t.transform_xy(src[0], src[1]);
        let err = (got[0] - want[0]).hypot(got[1] - want[1]);
        worst = worst.max(err);
        assert!(err <= 0.05, "{src:?} -> {got:?}, PROJ {want:?}, error {err:.4} m");
    }
    eprintln!("max error vs PROJ: {worst:.6} m");
}
