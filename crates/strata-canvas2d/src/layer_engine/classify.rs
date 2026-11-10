//! 분류 렌더러 — 속성값마다 다른 색을 준다. C++ `LayerStyleKinds` 1:1.
//!
//! ## C++ 이 하는 일 (`LayerStyleKinds.cpp`)
//!
//! 1. 레이어에 `kind` 또는 `period` 필드가 있으면 그 필드로 분류한다.
//! 2. 고유값을 모아 정렬하고, 값마다 **안정 해시(FNV-1a)** 로 색을 고른다.
//!    `qHash` 는 Qt 6 에서 프로세스마다 시드가 달라 같은 값이 실행할 때마다
//!    다른 색이 되므로, C++ 도 FNV-1a 를 직접 쓴다. 여기서도 같게 한다.
//! 3. 색이 겹치면 `kStride = 7`(kCandidates=48 과 서로소)로 다음 후보를 찾는다.
//! 4. **빈 값·NULL·못 넣은 값은 "미분류" 범주로 남긴다** — 화면에서 사라지면 안 된다.
//!
//! ## 왜 직접 만들지 않는가
//!
//! 이건 색 배정 규칙이라 외부 크레이트에 없다. 다만 값 수집·정렬은 표준 라이브러리로
//! 충분하고, 색 변환(HSV)은 직접 계산한다(C++ `QColor::fromHsv` 와 같은 공식).

use std::collections::BTreeSet;

/// 색상 단계 수 — 15도 간격.
const HUE_STEPS: usize = 24;
/// 후보 색 수 — 각 색상을 밝은 톤·어두운 톤 두 가지로.
const CANDIDATES: usize = HUE_STEPS * 2;
/// 겹칠 때 다음 후보로 건너뛰는 폭. `CANDIDATES` 와 서로소(48)라 한 바퀴에 모두 본다.
const STRIDE: usize = 7;

/// 분류 필드 이름 후보. C++ `LayerStyleKinds::categoryField` 와 같은 순서.
pub const CATEGORY_FIELDS: [&str; 2] = ["kind", "period"];

/// 미분류 범주의 이름. C++ `LayerStyleKinds::unclassifiedLabel()`.
pub const UNCLASSIFIED_LABEL: &str = "미분류";

/// FNV-1a (32bit) over UTF-8. C++ `stableHash` 와 같은 값이 나온다.
///
/// `qHash` 는 프로세스마다 시드가 달라 같은 분류값이 실행할 때마다 다른 색이 된다.
/// 그래서 양쪽 다 FNV-1a 를 쓴다 — 앱을 다시 켜도 색이 그대로여야 한다.
#[must_use]
pub fn stable_hash(text: &str) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for b in text.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

/// 후보 색 하나. C++ `candidate(int)` — HSV(색상, 채도 170, 명도 215/150).
#[must_use]
pub fn candidate(index: usize) -> [u8; 3] {
    let i = index % CANDIDATES;
    let hue_deg = ((i % HUE_STEPS) * (360 / HUE_STEPS)) as f64;
    let value = if i < HUE_STEPS { 215.0 } else { 150.0 };
    hsv_to_rgb(hue_deg, 170.0 / 255.0, value / 255.0)
}

/// HSV(0~360, 0~1, 0~1) → RGB. `QColor::fromHsv` 와 같은 결과.
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    if s <= 0.0 {
        let g = (v * 255.0).round() as u8;
        return [g, g, g];
    }
    let h = h.rem_euclid(360.0);
    let sector = h / 60.0;
    let i = sector.floor();
    let f = sector - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i as i32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

/// 분류값 하나의 스타일. 채움은 알파 150, 테두리는 같은 색의 어두운 톤(C++ `darker(160)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryStyle {
    pub fill: [u8; 4],
    pub stroke: [u8; 4],
}

/// 분류 범주 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// 속성 원문. 미분류는 빈 문자열.
    pub value: String,
    /// 범례에 보일 이름.
    pub label: String,
    pub style: CategoryStyle,
}

/// 분류 렌더러 — 값 → 스타일 표.
///
/// C++ `QgsCategorizedSymbolRenderer` 자리. 마지막 범주는 언제나 **미분류**이며
/// 어떤 값이든 여기로 떨어진다(피처가 화면에서 사라지지 않게).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizedRenderer {
    pub field: String,
    pub categories: Vec<Category>,
}

impl CategorizedRenderer {
    /// 분류 필드 이름을 고른다. `kind` → `period` 순. 없으면 `None`.
    ///
    /// C++ `LayerStyleKinds::categoryField` 와 같은 순서·규칙.
    #[must_use]
    pub fn pick_field<'a, I>(&self, field_names: I) -> Option<&'a str>
    where
        I: IntoIterator<Item = &'a str> + Clone,
    {
        for want in CATEGORY_FIELDS {
            if field_names.clone().into_iter().any(|n| n == want) {
                return field_names
                    .into_iter()
                    .find(|n| *n == want);
            }
        }
        None
    }

    /// 속성값들로 범주 표를 만든다.
    ///
    /// 빈 값은 범주로 만들지 않고(공백만 있는 값 포함) 미분류로 떨어뜨린다.
    /// C++ 은 `!text.trimmed().isEmpty()` 인 값만 모은다.
    #[must_use]
    pub fn build(field: &str, values: &[String]) -> Self {
        let unique: BTreeSet<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();

        let mut taken: Vec<[u8; 3]> = Vec::new();
        let mut categories = Vec::with_capacity(unique.len() + 1);
        for kind in unique {
            let rgb = color_for_kind(kind, &taken);
            taken.push(rgb);
            categories.push(Category {
                value: kind.to_owned(),
                label: kind.to_owned(),
                style: CategoryStyle {
                    fill: [rgb[0], rgb[1], rgb[2], 150],
                    // C++ `color.darker(160)` — 밝기를 160/100 으로 나눈다.
                    stroke: [
                        (u16::from(rgb[0]) * 100 / 160) as u8,
                        (u16::from(rgb[1]) * 100 / 160) as u8,
                        (u16::from(rgb[2]) * 100 / 160) as u8,
                        255,
                    ],
                },
            });
        }
        // 미분류 — 빈 값·NULL·표에 없는 값이 여기로 온다.
        categories.push(Category {
            value: String::new(),
            label: UNCLASSIFIED_LABEL.to_owned(),
            style: CategoryStyle {
                fill: [37, 99, 235, 90],
                stroke: [37, 99, 235, 255],
            },
        });
        Self {
            field: field.to_owned(),
            categories,
        }
    }

    /// 값에 맞는 스타일. 못 찾으면 미분류(마지막 범주).
    #[must_use]
    pub fn style_for(&self, value: Option<&str>) -> CategoryStyle {
        let key = value.map(str::trim).unwrap_or("");
        if !key.is_empty() {
            if let Some(c) = self.categories.iter().find(|c| c.value == key) {
                return c.style;
            }
        }
        // 마지막은 언제나 미분류다. 없으면 기본 파랑.
        self.categories
            .last()
            .map(|c| c.style)
            .unwrap_or(CategoryStyle {
                fill: [37, 99, 235, 90],
                stroke: [37, 99, 235, 255],
            })
    }

    /// 범례에 보일 (이름, 스타일) 목록.
    #[must_use]
    pub fn legend(&self) -> Vec<(String, CategoryStyle)> {
        self.categories
            .iter()
            .map(|c| (c.label.clone(), c.style))
            .collect()
    }
}

/// 값 하나의 색. 이미 쓴 색이면 `STRIDE` 씩 건너뛰어 빈 색을 찾는다.
///
/// C++ `LayerStyleKinds::colorForKind` 와 같은 규칙. 후보를 다 돌아도 비지 않으면
/// 그 값의 첫 선택을 그대로 쓴다(색이 겹쳐도 피처가 사라지는 것보다 낫다).
#[must_use]
pub fn color_for_kind(kind: &str, taken: &[[u8; 3]]) -> [u8; 3] {
    let start = (stable_hash(kind.trim()) as usize) % CANDIDATES;
    let mut first_free: Option<[u8; 3]> = None;
    for step in 0..CANDIDATES {
        let color = candidate(start + step * STRIDE);
        if first_free.is_none() {
            first_free = Some(color);
        }
        if !taken.contains(&color) {
            return color;
        }
    }
    first_free.unwrap_or_else(|| candidate(start))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a 는 실행 사이에 **같은 값**을 낸다 — 앱을 다시 켜도 색이 같아야 한다.
    #[test]
    fn stable_hash_is_deterministic() {
        assert_eq!(stable_hash("청동기"), stable_hash("청동기"));
        assert_eq!(stable_hash(""), 2_166_136_261);
        // FNV-1a("a") = 0xe40c292c
        assert_eq!(stable_hash("a"), 0xe40c_292c);
    }

    /// 같은 값은 같은 색, 다른 값은 (거의) 다른 색.
    #[test]
    fn same_kind_gets_same_color() {
        let a = color_for_kind("청동기", &[]);
        let b = color_for_kind("청동기", &[]);
        assert_eq!(a, b);
        let c = color_for_kind("철기", &[]);
        assert_ne!(a, c, "다른 분류값은 다른 색이어야 한다");
    }

    /// 이미 쓴 색은 피한다 — 범례에서 두 범주 색이 같으면 구별이 안 된다.
    #[test]
    fn taken_colors_are_avoided() {
        let first = color_for_kind("A", &[]);
        let second = color_for_kind("A", &[first]);
        assert_ne!(first, second, "이미 쓴 색을 다시 주면 안 된다");
    }

    /// 후보를 다 써도 패닉하지 않고 무언가를 준다.
    #[test]
    fn exhaustion_does_not_panic() {
        let mut taken = Vec::new();
        for i in 0..CANDIDATES {
            taken.push(candidate(i));
        }
        let c = color_for_kind("가득찬", &taken);
        // 값이 나오면 된다(겹쳐도 피처가 사라지는 것보다 낫다).
        let _ = c;
    }

    /// **핵심**: 빈 값·NULL·모르는 값은 미분류로 떨어져 화면에서 사라지지 않는다.
    #[test]
    fn unclassified_fallback_catches_everything() {
        let r = CategorizedRenderer::build(
            "kind",
            &[
                "청동기".to_owned(),
                "철기".to_owned(),
                "  ".to_owned(),
                String::new(),
            ],
        );
        // 값 2개 + 미분류 1개
        assert_eq!(r.categories.len(), 3);
        assert_eq!(r.categories.last().expect("미분류").label, UNCLASSIFIED_LABEL);

        let fallback = r.categories.last().expect("미분류").style;
        assert_eq!(r.style_for(None), fallback, "NULL 은 미분류");
        assert_eq!(r.style_for(Some("")), fallback, "빈 문자열은 미분류");
        assert_eq!(r.style_for(Some("  ")), fallback, "공백은 미분류");
        assert_eq!(r.style_for(Some("없는값")), fallback, "표에 없는 값은 미분류");
        assert_ne!(r.style_for(Some("청동기")), fallback, "분류된 값은 자기 색");
    }

    /// 카테고리는 정렬돼 나온다(범례 순서가 실행마다 달라지면 안 된다).
    #[test]
    fn categories_are_sorted_and_unique() {
        let r = CategorizedRenderer::build(
            "kind",
            &[
                "철기".to_owned(),
                "청동기".to_owned(),
                "철기".to_owned(),
                "신석기".to_owned(),
            ],
        );
        let values: Vec<&str> = r
            .categories
            .iter()
            .filter(|c| !c.value.is_empty())
            .map(|c| c.value.as_str())
            .collect();
        assert_eq!(values.len(), 3, "중복은 하나로");
        let mut sorted = values.clone();
        sorted.sort_unstable();
        assert_eq!(values, sorted, "정렬돼야 한다");
    }

    /// 분류 필드는 kind 를 period 보다 먼저 고른다 — C++ 과 같은 순서.
    #[test]
    fn picks_kind_before_period() {
        let r = CategorizedRenderer::build("kind", &[]);
        assert_eq!(r.pick_field(["period", "kind"]), Some("kind"));
        assert_eq!(r.pick_field(["period", "name"]), Some("period"));
        assert_eq!(r.pick_field(["name"]), None);
    }

    /// 후보 색은 서로 다르고 채도가 있다(회색 덩어리 방지).
    #[test]
    fn candidates_are_distinct_colors() {
        let mut seen = Vec::new();
        for i in 0..CANDIDATES {
            let c = candidate(i);
            assert!(c.iter().any(|ch| *ch != c[0]) || c[0] == 0, "무채색이면 안 된다: {c:?}");
            assert!(!seen.contains(&c), "후보 {i} 가 중복: {c:?}");
            seen.push(c);
        }
    }

    /// 채움은 알파 150, 테두리는 불투명 — C++ 과 같다.
    #[test]
    fn style_alpha_matches_cpp() {
        let r = CategorizedRenderer::build("kind", &["청동기".to_owned()]);
        let s = r.style_for(Some("청동기"));
        assert_eq!(s.fill[3], 150, "C++ kindFill.setAlpha(150)");
        assert_eq!(s.stroke[3], 255, "테두리는 불투명");
    }

    /// 범례는 모든 범주를 담는다.
    #[test]
    fn legend_lists_every_category() {
        let r = CategorizedRenderer::build("kind", &["A".to_owned(), "B".to_owned()]);
        let legend = r.legend();
        assert_eq!(legend.len(), 3, "A, B, 미분류");
        assert_eq!(legend.last().expect("마지막").0, UNCLASSIFIED_LABEL);
    }
}
