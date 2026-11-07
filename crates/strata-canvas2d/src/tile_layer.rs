use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Default, Clone)]
pub struct NonBlockingTileCache {
    is_drawing: bool,
    tiles_rgba: HashMap<TileCoord, Vec<u8>>,
}

impl NonBlockingTileCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_draw(&mut self) {
        self.is_drawing = true;
    }

    pub fn end_draw(&mut self) {
        self.is_drawing = false;
    }

    pub fn is_drawing(&self) -> bool {
        self.is_drawing
    }

    pub fn insert_tile_if_idle(&mut self, coord: TileCoord, rgba: Vec<u8>) -> bool {
        if self.is_drawing {
            return false;
        }
        self.tiles_rgba.insert(coord, rgba);
        true
    }
}
