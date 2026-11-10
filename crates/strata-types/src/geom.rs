//! L0 공유 기하 타입 — 범위(Extent)와 기하 열거형.
//!
//! ## 왜 여기인가
//!
//! C++ 은 QGIS 의 `QgsRectangle`·`QgsGeometry` 를 쓴다. Rust 에서 같은 역할을
//! 여러 크레이트가 각자 만들면 판정 규칙이 갈라진다. 실제로 범위 타입이
//! `strata-net/topo_types.rs`, `strata-net/dem_tiles.rs`, `strata-paleo/ngii_dem.rs`
//! 세 곳에 따로 있었고, **교차(intersects) 판정은 어디에도 없었다**.
//!
//! 그래서 L0(`strata-types`)에 한 벌만 두고 나머지가 가져다 쓴다.
//! 이 크레이트는 외부 GIS 크레이트에 의존하지 않으므로 여기에 두어도 안전하다.
//!
//! ## GeoRust 와의 관계
//!
//! 면적·중심·볼록껍질 같은 **알고리즘**은 [`geo`](https://georust.org) 크레이트가
//! 이미 제공하므로 다시 만들지 않는다. 여기에는 화면·필터 판정에 매일 쓰는
//! 최소 타입만 둔다.

/// 축 정렬 경계 상자. C++ `QgsRectangle` 자리.
///
/// 좌표계는 호출자가 정한다(작업 좌표계 미터가 보통).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Extent {
    /// 네 값으로 만든다. 최소·최대가 뒤집혀 있으면 바로잡는다.
    #[must_use]
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    /// 한 점에서 크기 0 인 범위.
    #[must_use]
    pub fn point(x: f64, y: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    /// 여러 점을 감싸는 범위. 점이 없으면 `None`.
    #[must_use]
    pub fn from_points<I: IntoIterator<Item = [f64; 2]>>(pts: I) -> Option<Self> {
        let mut it = pts.into_iter();
        let first = it.next()?;
        let mut e = Self::point(first[0], first[1]);
        for p in it {
            e.include_xy(p[0], p[1]);
        }
        Some(e)
    }

    /// 점 하나를 포함하도록 넓힌다(비어 있던 범위도 이걸로 시작할 수 있다).
    pub fn include_xy(&mut self, x: f64, y: f64) {
        if x < self.min_x {
            self.min_x = x;
        }
        if x > self.max_x {
            self.max_x = x;
        }
        if y < self.min_y {
            self.min_y = y;
        }
        if y > self.max_y {
            self.max_y = y;
        }
    }

    /// 폭.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// 높이.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// 중심.
    #[must_use]
    pub fn center(&self) -> [f64; 2] {
        [
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
        ]
    }

    /// 점을 포함하는가(경계 포함).
    ///
    /// C++ `QgsRectangle::contains` 와 같이 **경계 위의 점도 포함**으로 본다.
    /// 클릭 판정에서 경계를 빼면 화면 가장자리 선택이 안 된다.
    #[must_use]
    pub fn contains_xy(&self, p: [f64; 2]) -> bool {
        p[0] >= self.min_x && p[0] <= self.max_x && p[1] >= self.min_y && p[1] <= self.max_y
    }

    /// 다른 범위를 완전히 담는가.
    #[must_use]
    pub fn contains_extent(&self, o: &Self) -> bool {
        o.min_x >= self.min_x
            && o.max_x <= self.max_x
            && o.min_y >= self.min_y
            && o.max_y <= self.max_y
    }

    /// 겹치는가. 경계에서 맞닿기만 해도 참이다(`QgsRectangle::intersects` 와 같다).
    ///
    /// 뷰포트 필터에서 맞닿은 피처를 빼면 화면 가장자리가 비어 보인다.
    #[must_use]
    pub fn intersects(&self, o: &Self) -> bool {
        self.min_x <= o.max_x
            && self.max_x >= o.min_x
            && self.min_y <= o.max_y
            && self.max_y >= o.min_y
    }

    /// 두 범위를 함께 감싸는 범위.
    #[must_use]
    pub fn union(&self, o: &Self) -> Self {
        Self {
            min_x: self.min_x.min(o.min_x),
            min_y: self.min_y.min(o.min_y),
            max_x: self.max_x.max(o.max_x),
            max_y: self.max_y.max(o.max_y),
        }
    }

    /// 사방으로 `m` 만큼 넓힌다. 클릭 판정의 허용 오차용.
    #[must_use]
    pub fn expanded(&self, m: f64) -> Self {
        let m = m.abs();
        Self {
            min_x: self.min_x - m,
            min_y: self.min_y - m,
            max_x: self.max_x + m,
            max_y: self.max_y + m,
        }
    }

    /// 폭·높이가 모두 0 이하인가(그릴 것이 없는 범위).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }
}

/// 폴리곤 하나 — 외곽 링 + 구멍 링들.
///
/// 타입이 길어 따로 이름을 붙인다(clippy `type_complexity`).
pub type PolygonRings = (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>);

/// 지오메트리 종류별 값. C++ `QgsGeometry` 자리.
///
/// `geo` 크레이트의 타입을 그대로 쓰지 않는 이유: 이 크레이트는 GIS 크레이트에
/// 의존하지 않는다(L0 원칙). 변환이 필요하면 상위 크레이트에서 한다.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point([f64; 2]),
    LineString(Vec<[f64; 2]>),
    /// 외곽 링 + 구멍들. 링은 닫혀 있지 않아도 된다(마지막 점이 첫 점과 같아도 무방).
    Polygon {
        exterior: Vec<[f64; 2]>,
        interiors: Vec<Vec<[f64; 2]>>,
    },
    MultiPolygon(Vec<PolygonRings>),
}

impl Geometry {
    /// 이 기하를 감싸는 범위. 점이 없으면 `None`.
    #[must_use]
    pub fn extent(&self) -> Option<Extent> {
        match self {
            Self::Point(p) => Some(Extent::point(p[0], p[1])),
            Self::LineString(pts) => Extent::from_points(pts.iter().copied()),
            Self::Polygon {
                exterior,
                interiors,
            } => {
                let mut e = Extent::from_points(exterior.iter().copied())?;
                for r in interiors {
                    if let Some(ie) = Extent::from_points(r.iter().copied()) {
                        e = e.union(&ie);
                    }
                }
                Some(e)
            }
            Self::MultiPolygon(parts) => {
                let mut acc: Option<Extent> = None;
                for (ext, ints) in parts {
                    if let Some(e) = Extent::from_points(ext.iter().copied()) {
                        acc = Some(match acc {
                            Some(a) => a.union(&e),
                            None => e,
                        });
                    }
                    for r in ints {
                        if let Some(ie) = Extent::from_points(r.iter().copied()) {
                            acc = Some(match acc {
                                Some(a) => a.union(&ie),
                                None => ie,
                            });
                        }
                    }
                }
                acc
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fixes_reversed_corners() {
        let e = Extent::new(10.0, 20.0, 0.0, 5.0);
        assert_eq!((e.min_x, e.min_y, e.max_x, e.max_y), (0.0, 5.0, 10.0, 20.0));
    }

    #[test]
    fn contains_and_intersects_follow_qgis_rules() {
        let a = Extent::new(0.0, 0.0, 10.0, 10.0);
        // 경계 위의 점은 포함.
        assert!(a.contains_xy([0.0, 0.0]));
        assert!(a.contains_xy([10.0, 10.0]));
        assert!(!a.contains_xy([10.1, 5.0]));

        // 맞닿기만 해도 교차.
        let edge = Extent::new(10.0, 0.0, 20.0, 10.0);
        assert!(a.intersects(&edge), "경계 접촉은 교차다");
        let apart = Extent::new(10.1, 0.0, 20.0, 10.0);
        assert!(!a.intersects(&apart));
    }

    #[test]
    fn contains_extent_needs_full_cover() {
        let big = Extent::new(0.0, 0.0, 100.0, 100.0);
        let small = Extent::new(10.0, 10.0, 20.0, 20.0);
        let over = Extent::new(90.0, 90.0, 110.0, 110.0);
        assert!(big.contains_extent(&small));
        assert!(!big.contains_extent(&over));
        // 교차는 하지만 담지는 못한다.
        assert!(big.intersects(&over));
    }

    #[test]
    fn from_points_and_geometry_extent_agree() {
        let ring = vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 50.0],
            [0.0, 50.0],
        ];
        let e = Extent::from_points(ring.iter().copied()).expect("점이 있다");
        assert_eq!(e, Extent::new(0.0, 0.0, 100.0, 50.0));

        let g = Geometry::Polygon {
            exterior: ring,
            interiors: vec![],
        };
        assert_eq!(g.extent(), Some(e));
    }

    #[test]
    fn polygon_with_hole_extent_covers_hole() {
        let g = Geometry::Polygon {
            exterior: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
            // 구멍이 바깥보다 커도 범위는 둘을 모두 감싼다.
            interiors: vec![vec![[-10.0, -10.0], [110.0, -10.0], [110.0, 110.0]]],
        };
        assert_eq!(
            g.extent(),
            Some(Extent::new(-10.0, -10.0, 110.0, 110.0))
        );
    }

    #[test]
    fn empty_geometry_has_no_extent() {
        assert_eq!(Geometry::LineString(vec![]).extent(), None);
        assert_eq!(Geometry::MultiPolygon(vec![]).extent(), None);
    }

    #[test]
    fn expanded_and_union() {
        let a = Extent::new(0.0, 0.0, 10.0, 10.0);
        let b = a.expanded(5.0);
        assert_eq!(b, Extent::new(-5.0, -5.0, 15.0, 15.0));
        let c = Extent::new(20.0, 20.0, 30.0, 30.0);
        assert_eq!(a.union(&c), Extent::new(0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn zero_size_extent_is_empty() {
        assert!(Extent::point(5.0, 5.0).is_empty());
        assert!(!Extent::new(0.0, 0.0, 1.0, 1.0).is_empty());
    }
}
