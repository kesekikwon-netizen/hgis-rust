//! C++ `GeoInfoBoreholeMaps.cpp:41` 의 `ri(double v, const char* hex, const char* lbl)` —
//! 시추 5종 도면의 `QgsColorRampShader::ColorRampItem` 목록을 hex·라벨·값 그대로 옮긴다.

/// C++ `QgsColorRampShader::ColorRampItem` 1:1 대응 (값, `#rrggbb`, 한국어 범례 라벨).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RampStop {
    pub value: f64,
    pub color_hex: &'static str,
    pub label: &'static str,
}

/// C++ `QgsColorRampShader(minVal, maxVal)` + `setColorRampItemList(items)` 1:1 대응.
#[derive(Debug, Clone, PartialEq)]
pub struct BoreholeColorRamp {
    pub min_value: f64,
    pub max_value: f64,
    pub stops: Vec<RampStop>,
}

impl BoreholeColorRamp {
    /// 램프 항목의 RGB. hex 표기가 잘못되면 `None`.
    #[must_use]
    pub fn stop_rgb(&self, index: usize) -> Option<[u8; 3]> {
        self.stops
            .get(index)
            .and_then(|s| parse_hex_rgb(s.color_hex))
    }
}

/// `#rrggbb` 16진 표기를 RGB 로 변환한다.
#[must_use]
pub fn parse_hex_rgb(hex: &str) -> Option<[u8; 3]> {
    let body = hex.trim().strip_prefix('#')?;
    if body.len() != 6 || !body.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(body.get(0..2)?, 16).ok()?;
    let g = u8::from_str_radix(body.get(2..4)?, 16).ok()?;
    let b = u8::from_str_radix(body.get(4..6)?, 16).ok()?;
    Some([r, g, b])
}

/// C++ `Qgis::ShaderInterpolationMethod::Linear` (`GeoInfoBoreholeMaps.cpp:66`) 대응 —
/// 항목 사이를 선형 보간하고, 첫/끝 항목 밖은 그 항목 색으로 클램프한다.
#[must_use]
pub fn interpolate_ramp_color(ramp: &BoreholeColorRamp, value: f64) -> [u8; 3] {
    let first = ramp.stops.first();
    let last = ramp.stops.last();
    let (Some(first), Some(last)) = (first, last) else {
        return [0, 0, 0];
    };
    if !value.is_finite() || value <= first.value {
        return parse_hex_rgb(first.color_hex).unwrap_or([0, 0, 0]);
    }
    if value >= last.value {
        return parse_hex_rgb(last.color_hex).unwrap_or([0, 0, 0]);
    }
    for pair in ramp.stops.windows(2) {
        let (lo, hi) = (&pair[0], &pair[1]);
        if value < hi.value {
            let span = hi.value - lo.value;
            let t = if span.abs() > f64::EPSILON {
                ((value - lo.value) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let a = parse_hex_rgb(lo.color_hex).unwrap_or([0, 0, 0]);
            let b = parse_hex_rgb(hi.color_hex).unwrap_or([0, 0, 0]);
            let mix = |x: u8, y: u8| -> u8 {
                let v = (f64::from(x) + t * (f64::from(y) - f64::from(x))).round();
                v.clamp(0.0, 255.0) as u8
            };
            return [
                mix(a[0], b[0]),
                mix(a[1], b[1]),
                mix(a[2], b[2]),
            ];
        }
    }
    parse_hex_rgb(last.color_hex).unwrap_or([0, 0, 0])
}

/// ② `GeoInfoBoreholeMaps.cpp:255-257` — 자연 퇴적층 두께(m), 래스터 범위 0.0~8.0
#[must_use]
pub fn alluvial_isopach_ramp() -> BoreholeColorRamp {
    BoreholeColorRamp {
        min_value: 0.0,
        max_value: 8.0,
        stops: vec![
            RampStop {
                value: 0.5,
                color_hex: "#fef3c7",
                label: "0.5m 이하",
            },
            RampStop {
                value: 1.5,
                color_hex: "#fde047",
                label: "1.5m 완경사",
            },
            RampStop {
                value: 3.0,
                color_hex: "#f59e0b",
                label: "3.0m 문화층 유망",
            },
            RampStop {
                value: 5.0,
                color_hex: "#d97706",
                label: "5.0m 후층",
            },
            RampStop {
                value: 8.0,
                color_hex: "#92400e",
                label: "8.0m+ 심부 매몰곡",
            },
        ],
    }
}

/// ③ `GeoInfoBoreholeMaps.cpp:268-269` — 성토·매립층 두께(m), 래스터 범위 0.0~5.0
#[must_use]
pub fn fill_thickness_ramp() -> BoreholeColorRamp {
    BoreholeColorRamp {
        min_value: 0.0,
        max_value: 5.0,
        stops: vec![
            RampStop {
                value: 0.3,
                color_hex: "#f1f5f9",
                label: "0.3m 이하 미교란",
            },
            RampStop {
                value: 1.0,
                color_hex: "#fdba74",
                label: "1.0m 얕은 성토",
            },
            RampStop {
                value: 2.5,
                color_hex: "#ea580c",
                label: "2.5m 중장비 제거",
            },
            RampStop {
                value: 5.0,
                color_hex: "#991b1b",
                label: "5.0m+ 심부 매립",
            },
        ],
    }
}

/// ④-A `GeoInfoBoreholeMaps.cpp:274-275` — 풍화암·기반암 출현 깊이(GL-m), 범위 0.0~12.0
#[must_use]
pub fn bedrock_depth_ramp() -> BoreholeColorRamp {
    BoreholeColorRamp {
        min_value: 0.0,
        max_value: 12.0,
        stops: vec![
            RampStop {
                value: 1.5,
                color_hex: "#dcfce7",
                label: "GL-1.5m 천부 기반암",
            },
            RampStop {
                value: 4.0,
                color_hex: "#4ade80",
                label: "GL-4.0m 사면",
            },
            RampStop {
                value: 7.5,
                color_hex: "#0d9488",
                label: "GL-7.5m 매몰 곡강",
            },
            RampStop {
                value: 12.0,
                color_hex: "#1e3a8a",
                label: "GL-12.0m+ 옛 골짜기",
            },
        ],
    }
}

/// ④-B `GeoInfoBoreholeMaps.cpp:279-283` — 풍화암·기반암 해발고도(EL.m).
///
/// 항목 값은 `mn`, `(mn + mx) * 0.5`, `mx` 로 C++ 와 같은 식으로 계산한다.
#[must_use]
pub fn bedrock_elev_ramp(mn: f64, mx: f64) -> BoreholeColorRamp {
    BoreholeColorRamp {
        min_value: mn,
        max_value: mx,
        stops: vec![
            RampStop {
                value: mn,
                color_hex: "#1e40af",
                label: "옛 골짜기 저점",
            },
            RampStop {
                value: (mn + mx) * 0.5,
                color_hex: "#10b981",
                label: "옛 사면",
            },
            RampStop {
                value: mx,
                color_hex: "#d97706",
                label: "기반암 능선",
            },
        ],
    }
}

/// ⑤ `GeoInfoBoreholeMaps.cpp:289-290` — 평균 지하수위(GL-m), 범위 0.5~6.0
#[must_use]
pub fn groundwater_ramp() -> BoreholeColorRamp {
    BoreholeColorRamp {
        min_value: 0.5,
        max_value: 6.0,
        stops: vec![
            RampStop {
                value: 1.0,
                color_hex: "#1d4ed8",
                label: "GL-1.0m 양수기 필수",
            },
            RampStop {
                value: 2.0,
                color_hex: "#38bdf8",
                label: "GL-2.0m 배수로 필수",
            },
            RampStop {
                value: 4.0,
                color_hex: "#a7f3d0",
                label: "GL-4.0m 보통",
            },
            RampStop {
                value: 6.0,
                color_hex: "#fef9c3",
                label: "GL-6.0m+ 양호",
            },
        ],
    }
}
