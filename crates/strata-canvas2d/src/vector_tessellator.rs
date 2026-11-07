#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex2d {
    pub pos: [f32; 2],
    pub rgba: [u8; 4],
}

pub const SELECTION_FILL_RGBA: [u8; 4] = [255, 235, 59, 45];

#[derive(Debug, Clone, Copy, Default)]
pub struct ReusableSnapMarker {
    pub visible: bool,
    pub screen_pos: [f32; 2],
}

impl ReusableSnapMarker {
    pub fn update(&mut self, hit_screen_pos: Option<[f32; 2]>) {
        if let Some(pos) = hit_screen_pos {
            self.visible = true;
            self.screen_pos = pos;
        } else {
            self.visible = false;
        }
    }
}
