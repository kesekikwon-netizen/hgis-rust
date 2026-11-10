//! 그리기 도형을 **오프스크린 픽셀**로 구워 검증한다.
//!
//! 이것이 QGIS 엔진보다 나은 핵심이다. QGIS 는 QPainter 로 그린 결과를 픽셀로
//! 검증하려면 창을 띄우거나 별도 스냅샷 장치가 필요하다. Rust 엔진은 도형 목록을
//! 내므로 **창 없이** 픽셀을 만들고 "무엇이 어디에 그려졌는지" 수치로 확인할 수 있다.
//!
//! 쓰임:
//! - 회귀 시험: "이 레이어를 켜면 빨간 픽셀이 1,200개 이상" 같이 고정
//! - 스냅샷 없이 확인: 서버·CI 에서도 돈다
//! - 조판·PDF 도 같은 도형을 쓰므로 미리보기를 픽셀로 검사

use super::draw::LayerDraw;
use super::raster_paint::paint_shape;

/// RGBA 픽셀 판.
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    /// RGBA 순서, 길이 = width * height * 4.
    pub px: Vec<u8>,
}

impl Raster {
    /// 배경색으로 채운 판을 만든다.
    #[must_use]
    pub fn new(width: u32, height: u32, bg: [u8; 4]) -> Self {
        let mut px = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for _ in 0..(width as usize) * (height as usize) {
            px.extend_from_slice(&bg);
        }
        Self { width, height, px }
    }

    /// 한 점의 색.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Some([self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]])
    }

    /// 알파를 섞어 칠한다(위에 덮기).
    pub fn blend(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x >= self.width || y >= self.height || c[3] == 0 {
            return;
        }
        let i = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let a = u32::from(c[3]);
        let inv = 255 - a;
        for (dst_px, src_ch) in self.px[i..i + 3].iter_mut().zip(c.iter()) {
            let dst = u32::from(*dst_px);
            let src = u32::from(*src_ch);
            *dst_px = ((src * a + dst * inv) / 255) as u8;
        }
        self.px[i + 3] = 255;
    }

    /// 특정 색 계열 픽셀 수(시험용). `tolerance` 는 채널당 허용 오차.
    #[must_use]
    pub fn count_near(&self, target: [u8; 4], tolerance: u8) -> usize {
        let tol = i32::from(tolerance);
        let t = [
            i32::from(target[0]),
            i32::from(target[1]),
            i32::from(target[2]),
        ];
        let mut n = 0;
        for chunk in self.px.chunks_exact(4) {
            let d = (i32::from(chunk[0]) - t[0]).abs()
                + (i32::from(chunk[1]) - t[1]).abs()
                + (i32::from(chunk[2]) - t[2]).abs();
            if d <= tol * 3 {
                n += 1;
            }
        }
        n
    }

    /// 칠해진(배경이 아닌) 픽셀 수.
    #[must_use]
    pub fn count_painted(&self, bg: [u8; 4]) -> usize {
        self.px
            .chunks_exact(4)
            .filter(|c| c[0] != bg[0] || c[1] != bg[1] || c[2] != bg[2])
            .count()
    }

    /// 위아래 뒤집기(PNG 저장 시 필요할 수 있다).
    #[must_use]
    pub fn flipped_y(&self) -> Self {
        let stride = (self.width as usize) * 4;
        let mut px = Vec::with_capacity(self.px.len());
        for row in (0..self.height as usize).rev() {
            px.extend_from_slice(&self.px[row * stride..(row + 1) * stride]);
        }
        Self {
            width: self.width,
            height: self.height,
            px,
        }
    }
}

/// 도형들을 판에 굽는다. 라벨은 글꼴이 없으므로 **자리 표시 점**으로만 찍는다.
pub fn rasterize(draws: &[LayerDraw], width: u32, height: u32, bg: [u8; 4]) -> Raster {
    let mut r = Raster::new(width, height, bg);
    for d in draws {
        for s in &d.shapes {
            paint_shape(&mut r, s);
        }
    }
    r
}
