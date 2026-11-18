//! 「깨진 한글 바로잡기」 — `MainWindowContextMenus.cpp:350-356` calls `LayerOps::setShapefileEncoding`.

use crate::app_state::StrataAppState;
use crate::ref_vector::text::write_cpg;

impl StrataAppState {
    /// CP949, EUC-KR, UTF-8, or System (`이 컴퓨터 설정`). The `.cpg` is rewritten and the labels follow.
    pub(crate) fn set_shapefile_encoding(&mut self, key: &str, encoding: &str) -> bool {
        let Some(path) = self
            .ref_vectors
            .file_of(key)
            .map(std::path::Path::to_path_buf)
        else {
            return false;
        };
        if !write_cpg(&path, encoding) {
            return false;
        }
        self.ref_vectors.reload_shapefile(key);
        let _ = self.ref_vectors.shapefile_names(key);
        true
    }

    /// The name the map writes for this shapefile, in the encoding the layer is using now.
    pub(crate) fn shapefile_label_text(&mut self, key: &str) -> Vec<String> {
        self.ref_vectors.shapefile_names(key)
    }
}

#[cfg(test)]
#[path = "shapefile_encoding_tests.rs"]
mod tests;
