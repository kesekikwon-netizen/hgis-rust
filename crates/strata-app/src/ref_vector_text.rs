//! Shapefile text after 「깨진 한글 바로잡기」.
//! `LayerOps::setShapefileEncoding` (`target/cpp-ref/src/core/LayerOps.cpp:467-484`) writes the chosen
//! name into the `.cpg` and the layer is read again with that provider encoding. Map labels use that text.

use std::path::Path;

use strata_engine::{read_shapefile, shapefile_facts, Cell, ColumnKind};

use super::{RefVectorLayers, Shapes};

impl RefVectorLayers {
    /// Names already read for this layer. Painting uses this so a frame does not open the file again.
    pub(crate) fn cached_shapefile_names(&self, key: &str) -> Vec<String> {
        self.layers
            .iter()
            .find(|layer| layer.key == key)
            .and_then(|layer| layer.label_names.as_ref())
            .map(|(_, names)| names.clone())
            .unwrap_or_default()
    }

    /// The first text column, decoded with the provider encoding (the `.cpg` line, else the open-time guess).
    pub(crate) fn shapefile_names(&mut self, key: &str) -> Vec<String> {
        let Some(layer) = self.layers.iter().find(|l| l.key == key) else {
            return Vec::new();
        };
        if !layer
            .spec
            .file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("shp"))
        {
            return Vec::new();
        }
        let path = layer.spec.file.clone();
        let encoding = provider_encoding(&path);
        if let Some((_, names)) = layer
            .label_names
            .as_ref()
            .filter(|(used, _)| used == &encoding)
        {
            return names.clone();
        }
        let names = read_names(&path, &encoding);
        if let Some(layer) = self.layers.iter_mut().find(|l| l.key == key) {
            layer.label_names = Some((encoding, names.clone()));
        }
        names
    }

    /// Drop the cached text and ask for the shapes again (`layer->reload`, `triggerRepaint`).
    pub(crate) fn reload_shapefile(&mut self, key: &str) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.key == key) {
            layer.label_names = None;
            layer.shapes = Shapes::Wanted;
            layer.picture = None;
            layer.drawing = None;
            layer.failed = None;
        }
    }
}

/// The encoding `setProviderEncoding` stored: the first line of the `.cpg` (`LayerOps.cpp:472-480`).
/// No `.cpg`: the same guess as open (`shapefile_encoding`, `LayerOps.cpp:359-464`).
pub(crate) fn provider_encoding(path: &Path) -> String {
    let raw = std::fs::read(path.with_extension("cpg"))
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    let line = raw
        .as_deref()
        .and_then(|text| text.lines().next())
        .map(str::trim)
        .filter(|line| !line.is_empty());
    line.map(str::to_owned)
        .unwrap_or_else(|| strata_refmap::heritage::shapefile_encoding(path))
}

fn gdal_encoding(token: &str) -> String {
    match token.to_ascii_uppercase().as_str() {
        "UTF-8" | "UTF8" => "UTF-8".to_owned(),
        "CP949" | "949" => "CP949".to_owned(),
        // The menu value (`MainWindowContextMenus.cpp:351`). GDAL is asked for that name.
        "EUC-KR" => "EUC-KR".to_owned(),
        // "System" is the locale codec (`setProviderEncoding("System")`, LayerOps.cpp:479).
        _ => token.to_owned(),
    }
}

fn read_names(path: &Path, encoding: &str) -> Vec<String> {
    let gdal = gdal_encoding(encoding);
    let Ok(facts) = shapefile_facts(path, &gdal) else {
        return Vec::new();
    };
    let Some(column) = facts
        .columns
        .iter()
        .find(|(_, kind)| *kind == ColumnKind::Text)
        .map(|(name, _)| name.clone())
    else {
        return Vec::new();
    };
    let mut names = Vec::new();
    if read_shapefile(path, &gdal, &[column.as_str()], &mut |row| {
        let text = match row.cells.first() {
            Some(Cell::Text(text)) => text.clone(),
            _ => String::new(),
        };
        names.push(text);
        true
    })
    .is_err()
    {
        return Vec::new();
    }
    names
}

/// `setShapefileEncoding` writes `encoding` and a newline (`LayerOps.cpp:474-476`).
pub(crate) fn write_cpg(path: &Path, encoding: &str) -> bool {
    if !path.is_file()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("shp"))
    {
        return false;
    }
    std::fs::write(path.with_extension("cpg"), format!("{encoding}\n")).is_ok()
}
