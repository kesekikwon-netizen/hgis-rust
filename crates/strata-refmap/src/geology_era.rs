//! Geologic eras of the KIGAM 1:50,000 map: the class of a unit from the server's era text, its legend rank
//! and its colour (C++ `GeologyMapService.cpp:66-139`, `:346-356`).

#[cfg(test)]
#[path = "geology_era_tests.rs"]
mod tests;

/// One line of the C++ table `kEras` (`GeologyMapService.cpp:73-96`): the text looked for in the server's
/// era, the class it stands for and that class's colour (the standard era colours of the International
/// Commission on Stratigraphy, as the C++ comment says). Younger eras first: the order is the legend order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Era {
    pub keyword: &'static str,
    pub name: &'static str,
    pub rgb: [u8; 3],
}

const fn era(keyword: &'static str, name: &'static str, r: u8, g: u8, b: u8) -> Era {
    Era { keyword, name, rgb: [r, g, b] }
}

/// `kEras`. Periods come before the eras that hold them, so that the finer name is found first.
pub const ERAS: [Era; 21] = [
    era("제4기", "제4기", 249, 249, 127),
    era("네오기", "네오기(신제3기)", 255, 230, 25),
    era("신제3기", "네오기(신제3기)", 255, 230, 25),
    era("팔레오기", "팔레오기(고제3기)", 253, 154, 82),
    era("고제3기", "팔레오기(고제3기)", 253, 154, 82),
    era("백악기", "백악기", 127, 198, 78),
    era("쥐라기", "쥐라기", 52, 178, 201),
    era("쥬라기", "쥐라기", 52, 178, 201),
    era("트라이아스기", "트라이아스기", 129, 43, 146),
    era("페름기", "페름기", 240, 64, 40),
    era("석탄기", "석탄기", 103, 165, 153),
    era("데본기", "데본기", 203, 140, 55),
    era("실루리아기", "실루리아기", 179, 225, 182),
    era("오르도비스기", "오르도비스기", 0, 146, 112),
    era("캄브리아기", "캄브리아기", 127, 160, 86),
    era("신생대", "신생대", 242, 249, 29),
    era("중생대", "중생대", 103, 197, 202),
    era("고생대", "고생대", 153, 192, 141),
    era("원생누대", "원생누대", 247, 53, 99),
    era("시생누대", "시생누대", 240, 4, 127),
    era("선캄브리아", "선캄브리아시대", 247, 67, 112),
];

/// The class of a unit whose era text names none of the keywords (`kEraUnknown`, `:97`).
pub const ERA_UNKNOWN: &str = "시대미상";
/// The colour of a class that is not in the table (`eraColor`, `:355`).
pub const UNKNOWN_RGB: [u8; 3] = [200, 200, 200];

/// The keyword is in the text, also when the text is written with spaces (the Jeju layer writes 「제 4기」;
/// `eraTextMatches`, `:118-122`).
fn matches(era_text: &str, keyword: &str) -> bool {
    era_text.contains(keyword) || era_text.replace(' ', "").contains(&keyword.replace(' ', ""))
}

/// Legend rank of an era text: its line in the table, 21 when it names none (`eraRank`, `:125-132`).
#[must_use]
pub fn era_rank(era_text: &str) -> usize {
    ERAS.iter().position(|e| matches(era_text, e.keyword)).unwrap_or(ERAS.len())
}

/// The class of an era text (`GeologyMapService::eraClass`, `:346-350`).
#[must_use]
pub fn era_class(era_text: &str) -> &'static str {
    ERAS.get(era_rank(era_text)).map_or(ERA_UNKNOWN, |e| e.name)
}

/// The colour of a class (`GeologyMapService::eraColor`, `:352-356`).
#[must_use]
pub fn era_color(class: &str) -> [u8; 3] {
    ERAS.iter().find(|e| e.name == class).map_or(UNKNOWN_RGB, |e| e.rgb)
}

/// Hue (degrees), saturation and value (0 to 1) of a colour.
fn to_hsv(rgb: [u8; 3]) -> (f64, f64, f64) {
    let [r, g, b] = rgb.map(|c| f64::from(c) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, if max == 0.0 { 0.0 } else { delta / max }, max)
}

fn from_hsv(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let sector = (hue / 60.0).rem_euclid(6.0);
    let within = sector.fract();
    let (p, q, t) = (value * (1.0 - saturation), value * (1.0 - saturation * within), value * (1.0 - saturation * (1.0 - within)));
    // Truncated on purpose: `sector` is 0.0 to below 6.0.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rgb = match sector as u8 {
        0 => [value, t, p],
        1 => [q, value, p],
        2 => [p, value, t],
        3 => [p, q, value],
        4 => [t, p, value],
        _ => [value, p, q],
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    rgb.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// What Qt's `QColor::lighter(factor)` gives (the C++ shades units of one era with it, `:139`). As its
/// documentation says, the colour is taken to HSV and the value multiplied by `factor` percent; a factor
/// below 100 divides the value by `10000 / factor` percent instead (whole-number division). What the
/// documentation leaves open was observed on Qt 6.11 for every era colour (`geology_era_tests.rs`): a
/// value that would pass full brightness stays there, and the saturation is lowered by the excess.
#[must_use]
pub fn lighter(rgb: [u8; 3], factor: u32) -> [u8; 3] {
    if factor == 0 {
        return rgb;
    }
    let (hue, mut saturation, mut value) = to_hsv(rgb);
    if factor < 100 {
        value = value * 100.0 / f64::from(10_000 / factor);
    } else {
        value = value * f64::from(factor) / 100.0;
        if value > 1.0 {
            saturation = (saturation - (value - 1.0)).max(0.0);
            value = 1.0;
        }
    }
    from_hsv(hue, saturation, value)
}

/// The colour of a unit when the official map colour could not be read: the colour of its era, shaded by
/// its symbol so that units of one era differ (`unitFallbackColor`, `:134-140`).
#[must_use]
pub fn unit_fallback_color(era_text: &str, symbol: &str) -> [u8; 3] {
    let sum: u32 = symbol.bytes().map(u32::from).sum();
    // -2..=2 steps of 12 percent: 76, 88, 100, 112, 124.
    lighter(era_color(era_class(era_text)), 76 + (sum % 5) * 12)
}

/// Qt's `QColor::lightness()` of a colour (observed, `geology_era_tests.rs`): the mean of the largest and
/// the smallest channel, halves rounded up.
#[must_use]
pub fn lightness(rgb: [u8; 3]) -> u8 {
    let (max, min) = (rgb[0].max(rgb[1]).max(rgb[2]), rgb[0].min(rgb[1]).min(rgb[2]));
    u8::try_from((u16::from(max) + u16::from(min)).div_ceil(2)).unwrap_or(u8::MAX)
}
